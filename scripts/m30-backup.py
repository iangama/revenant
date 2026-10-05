#!/usr/bin/env python3
"""Seal local Revenant backups and prove restoration in a disposable container."""

import argparse
import hashlib
import json
import os
import re
import secrets
import signal
import stat
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
IMAGE = (
    "postgres:16.15-alpine@sha256:"
    "cf78e76683b9ca8c5733cbbdce6c9262b45b6767934dd0a95e671f9a0fc20685"
)
SOURCE = "infra-postgres-1"
TABLES = tuple(
    sorted(
        (
            "acquisition_milestones",
            "acquisition_claims",
            "accounts",
            "characters",
            "inventory",
            "progression",
            "activity_history",
            "replay_events",
            "inventory_reward_grants",
            "progression_reward_grants",
            "equipment_loadouts",
            "module_states",
            "module_loadout_slots",
            "module_operations",
            "route_operations",
            "route_operation_participants",
            "cooperation_operations",
            "cooperation_operation_participants",
        )
    )
)
SEQUENCES = ("activity_history_id_seq", "replay_events_id_seq")
PAYLOADS = ("archive.pgdump", "schema.sql", "state.json", "manifest.json")
MAX_FILE_BYTES = 64 * 1024**2


class Rejected(Exception):
    """A fixed diagnostic category; never include subprocess output or row data."""


def require(condition, category):
    if not condition:
        raise Rejected(category)


def run(args, *, data=None, timeout=120):
    try:
        result = subprocess.run(
            args, input=data, capture_output=True, timeout=timeout, check=False
        )
    except (OSError, subprocess.TimeoutExpired) as error:
        raise Rejected("command_unavailable_or_timeout") from error
    require(result.returncode == 0, "command_failed")
    require(len(result.stdout) <= MAX_FILE_BYTES, "output_size_limit")
    return result.stdout


def digest(data):
    return hashlib.sha256(data).hexdigest()


def encoded(value):
    return (json.dumps(value, sort_keys=True, separators=(",", ":")) + "\n").encode()


def private_path(path, *, directory=False):
    require(
        path.is_absolute() and path == path.resolve(),
        "absolute_nonsymlink_path_required",
    )
    info = path.lstat()
    kind = stat.S_ISDIR if directory else stat.S_ISREG
    require(kind(info.st_mode), "regular_file_or_directory_required")
    require(info.st_uid == os.getuid(), "owner_mismatch")
    require(
        stat.S_IMODE(info.st_mode) == (0o700 if directory else 0o600),
        "private_mode_required",
    )


def read_private(path):
    private_path(path)
    # Open once with O_NOFOLLOW; validated bytes, not paths, are used for restore.
    fd = os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK)
    with os.fdopen(fd, "rb") as handle:
        info = os.fstat(handle.fileno())
        require(
            stat.S_ISREG(info.st_mode)
            and info.st_uid == os.getuid()
            and stat.S_IMODE(info.st_mode) == 0o600,
            "private_file_required",
        )
        data = handle.read(MAX_FILE_BYTES + 1)
    require(len(data) <= MAX_FILE_BYTES, "file_size_limit")
    return data


def write_new(path, data):
    fd = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, 0o600)
    with os.fdopen(fd, "wb") as handle:
        handle.write(data)
        handle.flush()
        os.fsync(handle.fileno())
    private_path(path)


def source_identity():
    info = json.loads(run(["docker", "inspect", SOURCE]))[0]
    labels = info["Config"].get("Labels") or {}
    require(
        labels.get("com.docker.compose.project") == "infra"
        and labels.get("com.docker.compose.service") == "postgres",
        "source_labels_mismatch",
    )
    require(
        info["State"].get("Health", {}).get("Status") == "healthy", "source_not_healthy"
    )
    require(info["Config"]["Image"] == IMAGE, "source_image_mismatch")
    mounts = [
        m for m in info["Mounts"] if m["Destination"] == "/var/lib/postgresql/data"
    ]
    require(
        len(mounts) == 1
        and mounts[0].get("Type") == "volume"
        and mounts[0].get("Name") == "infra_revenant-postgres",
        "source_volume_mismatch",
    )
    require(
        not info["HostConfig"].get("PortBindings"),
        "close_database_maintenance_port_first",
    )
    require(
        sql(info["Id"], "revenant", "SELECT current_database(), current_user;").strip()
        == b"revenant|revenant",
        "source_database_identity_mismatch",
    )
    return info["Id"]


def sql(container, database, query):
    return run(
        [
            "docker",
            "exec",
            "-i",
            container,
            "psql",
            "-X",
            "-qAt",
            "-v",
            "ON_ERROR_STOP=1",
            "-U",
            "revenant",
            "-d",
            database,
        ],
        data=query.encode(),
    )


def snapshot(container, database):
    table_list = sql(
        container,
        database,
        "SELECT tablename FROM pg_tables WHERE schemaname='public' ORDER BY tablename;",
    )
    require(
        table_list.decode().splitlines() == list(TABLES), "table_inventory_mismatch"
    )
    sequence_list = sql(
        container,
        database,
        "SELECT sequencename FROM pg_sequences WHERE schemaname='public' "
        "ORDER BY sequencename;",
    )
    require(
        sequence_list.decode().splitlines() == list(SEQUENCES),
        "sequence_inventory_mismatch",
    )
    queries = [
        "BEGIN ISOLATION LEVEL REPEATABLE READ READ ONLY; SET LOCAL TIME ZONE 'UTC';"
    ]
    queries += [
        f"SELECT COALESCE(jsonb_agg(to_jsonb(t) ORDER BY to_jsonb(t)::text),'[]'::jsonb) "
        f"FROM public.{table} t;"
        for table in TABLES
    ]
    queries += [
        f"SELECT jsonb_build_object('last_value',last_value,'is_called',is_called) "
        f"FROM public.{name};"
        for name in SEQUENCES
    ]
    queries += ["COMMIT;"]
    values = [
        json.loads(line)
        for line in sql(container, database, "\n".join(queries)).splitlines()
    ]
    require(len(values) == len(TABLES) + len(SEQUENCES), "export_cardinality_mismatch")
    return encoded(dict(zip(TABLES + SEQUENCES, values, strict=True)))


def schema(container, database):
    raw = run(
        [
            "docker",
            "exec",
            container,
            "pg_dump",
            "-U",
            "revenant",
            "-d",
            database,
            "--schema-only",
            "--no-owner",
            "--no-privileges",
        ]
    )
    # pg_dump randomizes psql's restriction guard; all SQL definitions stay exact.
    return re.sub(rb"^\\(?:un)?restrict [^\n]*\n", b"", raw, flags=re.MULTILINE)


def migrations():
    paths = sorted((ROOT / "runtime/persistence/migrations").glob("*.sql"))
    require(len(paths) == 9, "migration_inventory_changed")
    return {p.name: p.read_bytes() for p in paths}


def scan_material(data):
    require(
        not re.search(
            rb"-----BEGIN [A-Z ]*PRIVATE KEY-----|"
            rb"postgres(?:ql)?://[^\s/]+:[^\s/]+@|SCRAM-SHA-256\$",
            data,
        ),
        "credential_material_detected",
    )
    state_root = Path(
        os.environ.get("XDG_STATE_HOME", str(Path.home() / ".local/state"))
    )
    current = Path(
        os.environ.get(
            "REVENANT_SECRETS_DIR",
            str(state_root / "revenant/m30-secrets/current"),
        )
    )
    for folder in (current, current.parent / "previous"):
        if not folder.is_dir():
            continue
        for path in folder.iterdir():
            if path.name.endswith(("_password", "_url")) and path.is_file():
                value = read_private(path).strip()
                require(len(value) >= 16, "invalid_secret_scan_input")
                require(value not in data, "known_secret_detected")


class Disposable:
    def __init__(self):
        self.token = secrets.token_hex(16)
        self.name = "m30g5restore-" + self.token
        self.database = "m30_restore_" + self.token
        self.container = None

    def __enter__(self):
        # No pre-existing named resource is ever adopted or removed.
        names = run(["docker", "ps", "-a", "--format", "{{.Names}}"])
        require(
            self.name not in names.decode().splitlines(), "disposable_container_exists"
        )
        try:
            self.container = (
                run(
                    [
                        "docker",
                        "create",
                        "--name",
                        self.name,
                        "--label",
                        "revenant.m30.restore=" + self.token,
                        "--network",
                        "none",
                        "--memory",
                        "256m",
                        "--memory-swap",
                        "256m",
                        "--pids-limit",
                        "64",
                        "--security-opt",
                        "no-new-privileges:true",
                        "--log-driver",
                        "none",
                        "--tmpfs",
                        "/var/lib/postgresql/data:rw,nosuid,nodev,size=128m",
                        "--tmpfs",
                        "/var/run/postgresql:rw,nosuid,nodev,size=16m",
                        "-e",
                        "POSTGRES_USER=revenant",
                        "-e",
                        "POSTGRES_DB=postgres",
                        "-e",
                        "POSTGRES_HOST_AUTH_METHOD=trust",
                        IMAGE,
                        "postgres",
                        "-c",
                        "listen_addresses=",
                    ]
                )
                .decode()
                .strip()
            )
            require(
                re.fullmatch(r"[a-f0-9]{64}", self.container), "invalid_container_id"
            )
            run(["docker", "start", self.container])
            deadline = time.monotonic() + 45
            while True:
                try:
                    sql(self.container, "postgres", "SELECT 1;")
                    break
                except Rejected:
                    require(time.monotonic() < deadline, "disposable_start_timeout")
                    time.sleep(0.25)
            require(
                sql(self.container, "postgres", "SHOW listen_addresses;").strip()
                == b"",
                "disposable_network_listener_detected",
            )
            require(
                sql(
                    self.container,
                    "postgres",
                    f"SELECT 1 FROM pg_database WHERE datname='{self.database}';",
                ).strip()
                == b"",
                "restore_database_exists",
            )
            sql(
                self.container,
                "postgres",
                f'CREATE DATABASE "{self.database}" TEMPLATE template0;',
            )
            return self
        except BaseException:
            if not self.container:
                # Docker may create the resource before its client is interrupted.
                # Recover only this invocation's random ownership label, never a name alone.
                owned = (
                    run(
                        [
                            "docker",
                            "ps",
                            "-aq",
                            "--no-trunc",
                            "--filter",
                            "label=revenant.m30.restore=" + self.token,
                        ]
                    )
                    .decode()
                    .splitlines()
                )
                require(len(owned) <= 1, "ambiguous_cleanup_ownership")
                self.container = owned[0] if owned else None
            self.close()
            raise

    def close(self):
        if self.container:
            info = json.loads(run(["docker", "inspect", self.container]))[0]
            require(
                info["Config"]["Labels"].get("revenant.m30.restore") == self.token,
                "cleanup_ownership_mismatch",
            )
            require(
                info["HostConfig"]["NetworkMode"] == "none" and not info["Mounts"],
                "cleanup_mount_boundary_mismatch",
            )
            run(["docker", "rm", "-f", self.container])
            self.container = None

    def __exit__(self, *_):
        self.close()


def restore_and_compare(payload):
    started = time.monotonic()
    with Disposable() as target:
        command = ["docker", "exec", "-i", target.container, "pg_restore"]
        listing = run(command + ["--list"], data=payload["archive.pgdump"])
        require(b"Format: CUSTOM" in listing, "custom_archive_required")
        # --list alone does not inspect compressed table data or detect all truncation.
        decoded = run(
            command + ["--no-owner", "--no-privileges", "--file=-"],
            data=payload["archive.pgdump"],
        )
        scan_material(decoded)
        run(
            command
            + [
                "-U",
                "revenant",
                "-d",
                target.database,
                "--exit-on-error",
                "--single-transaction",
                "--no-owner",
                "--no-privileges",
            ],
            data=payload["archive.pgdump"],
        )
        for attempt in range(3):
            if attempt:
                ddl = b"BEGIN;\nSELECT pg_advisory_xact_lock(824180018);\n"
                ddl += b"\n".join(migrations().values()) + b"\nCOMMIT;\n"
                sql(target.container, target.database, ddl.decode())
            require(
                snapshot(target.container, target.database) == payload["state.json"],
                "restored_rows_or_sequences_differ",
            )
            require(
                schema(target.container, target.database) == payload["schema.sql"],
                "restored_schema_differs",
            )
    return {
        "disposable_restore": "pass",
        "migration_passes": 2,
        "tables": len(TABLES),
        "sequences": len(SEQUENCES),
        "state_sha256": digest(payload["state.json"]),
        "schema_sha256": digest(payload["schema.sql"]),
        "database_restore_duration_ms": round((time.monotonic() - started) * 1000),
        "exact_container_removed": True,
        "application_recovery": "not_exercised_by_backup_command",
    }


def load_bundle(directory):
    private_path(directory, directory=True)
    require(
        {p.name for p in directory.iterdir()} == set(PAYLOADS) | {"SHA256SUMS"},
        "bundle_inventory_mismatch",
    )
    files = {name: read_private(directory / name) for name in PAYLOADS}
    expected = b"".join(
        f"{digest(files[name])}  {name}\n".encode() for name in PAYLOADS
    )
    require(read_private(directory / "SHA256SUMS") == expected, "checksum_mismatch")
    require(files["archive.pgdump"].startswith(b"PGDMP"), "custom_archive_required")
    manifest = json.loads(files["manifest.json"])
    require(
        manifest["format"] == "revenant.m30.backup.v1"
        and manifest["postgres_image"] == IMAGE,
        "backup_format_mismatch",
    )
    require(
        manifest["migrations"]
        == {name: digest(data) for name, data in migrations().items()},
        "backup_migration_revision_mismatch",
    )
    require(
        manifest["state_sha256"] == digest(files["state.json"])
        and manifest["schema_sha256"] == digest(files["schema.sql"]),
        "manifest_digest_mismatch",
    )
    for name in ("schema.sql", "state.json", "manifest.json"):
        scan_material(files[name])
    return files


def create(directory):
    require(
        directory.is_absolute() and directory == directory.resolve(),
        "absolute_nonsymlink_path_required",
    )
    require(not directory.is_relative_to(ROOT), "backup_must_be_outside_repository")
    require(not directory.exists(), "backup_directory_exists")
    container = source_identity()
    directory.mkdir(mode=0o700)
    private_path(directory, directory=True)
    before = snapshot(container, "revenant")
    schema_before = schema(container, "revenant")
    archive = run(
        [
            "docker",
            "exec",
            container,
            "pg_dump",
            "-U",
            "revenant",
            "-d",
            "revenant",
            "--format=custom",
            "--no-privileges",
            "--lock-wait-timeout=5s",
        ]
    )
    require(
        before == snapshot(container, "revenant")
        and schema_before == schema(container, "revenant"),
        "source_changed_during_backup",
    )
    payload = {
        "archive.pgdump": archive,
        "schema.sql": schema_before,
        "state.json": before,
    }
    for name, data in payload.items():
        if name != "archive.pgdump":
            scan_material(data)
        write_new(directory / name, data)
    result = restore_and_compare(payload)
    require(
        container == source_identity()
        and before == snapshot(container, "revenant")
        and schema_before == schema(container, "revenant"),
        "source_changed_during_restore_drill",
    )
    manifest = {
        "format": "revenant.m30.backup.v1",
        "version": "0.2.0",
        "created_at": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
        "postgres_image": IMAGE,
        "source_database": "revenant",
        "migrations": {name: digest(data) for name, data in migrations().items()},
        "row_counts": {
            name: len(rows)
            for name, rows in json.loads(before).items()
            if name in TABLES
        },
        **result,
    }
    payload["manifest.json"] = encoded(manifest)
    write_new(directory / "manifest.json", payload["manifest.json"])
    write_new(
        directory / "SHA256SUMS",
        b"".join(f"{digest(payload[name])}  {name}\n".encode() for name in PAYLOADS),
    )
    # Validate the on-disk seal, not just the buffers that were written.
    load_bundle(directory)
    fd = os.open(directory, os.O_RDONLY | os.O_DIRECTORY)
    try:
        os.fsync(fd)
    finally:
        os.close(fd)
    return {"backup_sealed": True, "source_unchanged": True, **result}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("action", choices=("create", "verify"))
    parser.add_argument(
        "directory", type=Path, help="absolute owner-only Linux backup directory"
    )
    args = parser.parse_args()
    os.umask(0o077)

    def interrupted(_number, _frame):
        raise Rejected("interrupted")

    signal.signal(signal.SIGTERM, interrupted)
    try:
        result = (
            create(args.directory)
            if args.action == "create"
            else restore_and_compare(load_bundle(args.directory))
        )
        print(json.dumps(result, sort_keys=True))
    except (
        Rejected,
        OSError,
        ValueError,
        KeyError,
        TypeError,
        IndexError,
        KeyboardInterrupt,
    ) as error:
        category = (
            str(error)
            if isinstance(error, Rejected)
            else "invalid_input_or_interrupted"
        )
        print(json.dumps({"result": "rejected", "category": category}), file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
