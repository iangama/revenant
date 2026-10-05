#!/usr/bin/env python3
"""M30 R01-R30: bounded backup, process-fault and application-recovery proof."""

import argparse
import importlib.util
import json
import os
import re
import secrets
import signal
import socket
import subprocess
import tempfile
import time
from collections import Counter
from pathlib import Path
from urllib.parse import urlsplit, urlunsplit
from urllib.request import Request, urlopen

ROOT = Path(__file__).resolve().parents[1]
SPEC = importlib.util.spec_from_file_location(
    "m30_backup", ROOT / "scripts/m30-backup.py"
)
backup = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(backup)
require = backup.require
OWNER_LABEL = "revenant.m30.recovery"
SERVICES = {"postgres", "migrate", "db-provision", "gateway", "inspector"}
CAPS = {
    "postgres": (256 * 1024**2, 64),
    "gateway": (128 * 1024**2, 192),
    "inspector": (64 * 1024**2, 32),
}
WALL_FIELDS = {
    "accepted_at",
    "terminal_at",
    "occurred_at",
    "granted_at",
    "completed_at",
}
ELAPSED_FIELDS = {
    "expires_elapsed_ms",
    "elapsed_ms",
    "anchor_elapsed_ms",
    "ping_elapsed_ms",
    "runner_elapsed_ms",
    "downed_elapsed_ms",
    "revive_started_elapsed_ms",
    "revive_completed_elapsed_ms",
    "warden_elapsed_ms",
    "terminal_elapsed_ms",
}


def run(args, *, env=None, data=None, timeout=120, check=True):
    result = subprocess.run(
        args,
        input=data,
        capture_output=True,
        cwd=ROOT,
        env=env,
        timeout=timeout,
        check=False,
    )
    if check:
        require(result.returncode == 0, "subprocess_failed")
    return result


def clean_env():
    return {
        key: value
        for key, value in os.environ.items()
        if not key.startswith(("REVENANT_", "M30_", "POSTGRES_", "DATABASE_"))
    }


def wait_until(predicate, label, seconds=30):
    deadline = time.monotonic() + seconds
    while time.monotonic() < deadline:
        if predicate():
            return
        time.sleep(0.1)
    raise backup.Rejected(label)


def canonical(raw):
    sessions = sorted(
        set(re.findall(r"session-[0-9]+", raw.decode())),
        key=lambda value: int(value.split("-")[1]),
    )
    mapping = {value: f"session-{index + 1}" for index, value in enumerate(sessions)}

    def visit(value, key=""):
        if value is not None and key in WALL_FIELDS | ELAPSED_FIELDS:
            return "OBSERVED_TIME"
        if isinstance(value, dict):
            return {name: visit(item, name) for name, item in value.items()}
        if isinstance(value, list):
            items = [visit(item) for item in value]
            # SQL exports are multisets. Timestamp normalization can change their order.
            return sorted(items, key=backup.encoded) if key in backup.TABLES else items
        if isinstance(value, str):
            if key == "payload" and value.startswith("{"):
                return visit(json.loads(value))
            return re.sub(r"session-[0-9]+", lambda match: mapping[match[0]], value)
        return value

    return backup.encoded(visit(json.loads(raw)))


def changed_entries(before, after):
    def entries(raw):
        result = Counter()
        for table, rows in json.loads(raw).items():
            for row in rows if isinstance(rows, list) else [rows]:
                result[(table, backup.encoded(row))] += 1
        return result

    first, second = entries(before), entries(after)
    return sum((first - second).values()) + sum((second - first).values())


def http_json(port, path, origin):
    request = Request(f"http://127.0.0.1:{port}{path}", headers={"Origin": origin})
    with urlopen(request, timeout=5) as response:
        require(response.status == 200, "http_status")
        if path.startswith("/api/"):
            require(
                response.headers.get("Cache-Control") == "no-store", "inspector_cache"
            )
        return json.load(response)


class Stack:
    def __init__(self, parent, restored=False):
        self.token = secrets.token_hex(6)
        self.project = "m30g5" + self.token
        self.database = ("m30r_" if restored else "m30f_") + self.token
        self.directory = parent / self.project
        self.directory.mkdir(mode=0o700)
        self.secret_dir = self.directory / "secrets/current"
        self.env = clean_env()
        self.env["REVENANT_SECRETS_ROOT"] = str(self.secret_dir.parent)
        run(["bash", "scripts/m30-generate-secrets.sh", "current"], env=self.env)
        self.env["REVENANT_SECRETS_DIR"] = str(self.secret_dir)
        for name in (
            "postgres_admin_database_url",
            "gateway_database_url",
            "operator_database_url",
        ):
            path = self.secret_dir / name
            url = urlsplit(path.read_text().strip())
            if name == "operator_database_url":
                url = url._replace(
                    netloc=url.netloc.rsplit("@", 1)[0] + "@127.0.0.1:15451"
                )
            path.write_text(urlunsplit(url._replace(path="/" + self.database)) + "\n")
        self.secrets = [p.read_bytes().strip() for p in self.secret_dir.iterdir()]
        self.game_port, self.health_port, self.inspector_port = (
            (17452, 18452, 41452) if restored else (17451, 18451, 41451)
        )
        self.origin = f"http://127.0.0.1:{self.inspector_port}"
        config = json.loads(
            run(
                [
                    "docker",
                    "compose",
                    "-p",
                    self.project,
                    "-f",
                    "infra/docker-compose.yml",
                    "config",
                    "--format",
                    "json",
                ],
                env=self.env,
            ).stdout
        )
        require(set(config["services"]) == SERVICES, "unexpected_compose_services")
        for service, value in config["services"].items():
            value.pop("build", None)
            if service in {"migrate", "gateway", "inspector"}:
                value["image"] = f"infra-{service}:latest"
            value.setdefault("labels", {})[OWNER_LABEL] = self.token
            value["restart"] = "no"
            for field in ("POSTGRES_DB",):
                if field in value.get("environment", {}):
                    value["environment"][field] = self.database
        config["services"]["postgres"]["environment"]["POSTGRES_DB"] = (
            "postgres" if restored else self.database
        )
        config["services"]["gateway"]["environment"]["REVENANT_EXPECTED_PLAYERS"] = "2"
        config["services"]["gateway"]["environment"]["REVENANT_INSPECTOR_ORIGIN"] = (
            self.origin
        )
        # The provisioning image declares PGDATA as a VOLUME even though this
        # one-shot client never starts a server. Suppress anonymous allocation.
        config["services"]["db-provision"].setdefault("tmpfs", []).append(
            "/var/lib/postgresql/data:rw,nosuid,nodev,noexec,size=1m"
        )

        def port(published, target):
            return {
                "host_ip": "127.0.0.1",
                "published": str(published),
                "target": target,
                "protocol": "tcp",
            }

        config["services"]["gateway"]["ports"] = [
            port(self.game_port, 7000),
            port(self.health_port, 8080),
        ]
        config["services"]["inspector"]["ports"] = [port(self.inspector_port, 80)]
        config["services"]["postgres"]["ports"] = (
            [] if restored else [port(15451, 5432)]
        )
        self.volumes = {self.project + "_" + name for name in config["volumes"]}
        for name, value in config["volumes"].items():
            value["name"] = self.project + "_" + name
            value["labels"] = {OWNER_LABEL: self.token}
        config["networks"]["default"]["name"] = self.project + "_default"
        config["networks"]["default"]["labels"] = {OWNER_LABEL: self.token}
        self.config = self.directory / "compose.json"
        backup.write_new(self.config, backup.encoded(config))
        self.compose = ["docker", "compose", "-p", self.project, "-f", str(self.config)]
        self.ids = {}
        self.closed = False
        for kind in ("container", "volume", "network"):
            require(not self.resources(kind), "preexisting_fixture_resources")

    def resources(self, kind):
        command = (
            ["docker", "ps", "-aq", "--no-trunc"]
            if kind == "container"
            else ["docker", kind, "ls", "-q"]
        )
        return (
            run(
                command
                + ["--filter", "label=com.docker.compose.project=" + self.project]
            )
            .stdout.decode()
            .splitlines()
        )

    def inspect_owned(self, kind, identifier):
        info = json.loads(run(["docker", kind, "inspect", identifier]).stdout)[0]
        labels = (
            info["Config"].get("Labels", {})
            if kind == "container"
            else info.get("Labels", {})
        )
        require(
            labels.get(OWNER_LABEL) == self.token
            and labels.get("com.docker.compose.project") == self.project,
            "fixture_ownership_mismatch",
        )
        if kind == "container":
            require(
                all(
                    m.get("Name") in self.volumes
                    for m in info["Mounts"]
                    if m["Type"] == "volume"
                ),
                "foreign_data_volume",
            )
            for bindings in (info["HostConfig"].get("PortBindings") or {}).values():
                require(
                    all(binding["HostIp"] == "127.0.0.1" for binding in bindings),
                    "non_loopback_fixture",
                )
        return info

    def refresh(self):
        for identifier in self.resources("container"):
            info = self.inspect_owned("container", identifier)
            self.ids[info["Config"]["Labels"]["com.docker.compose.service"]] = (
                identifier
            )

    def up(self, postgres_only=False):
        command = self.compose + [
            "up",
            "-d",
            "--no-build",
            "--wait",
            "--wait-timeout",
            "90",
        ]
        if postgres_only:
            command += ["postgres"]
        try:
            run(command, env=self.env)
        finally:
            self.refresh()

    def service(self, name):
        identifier = self.ids[name]
        self.inspect_owned("container", identifier)
        return identifier

    def sql(self, query, database=None):
        return backup.sql(self.service("postgres"), database or self.database, query)

    def capture(self):
        return backup.snapshot(self.service("postgres"), self.database)

    def schema(self):
        return backup.schema(self.service("postgres"), self.database)

    def stop(self, name, kill=False):
        identifier = self.service(name)
        run(
            ["docker", "kill", "--signal", "KILL", identifier]
            if kill
            else ["docker", "stop", "--time", "10", identifier]
        )
        require(
            not self.inspect_owned("container", identifier)["State"]["Running"],
            "service_did_not_stop",
        )

    def start(self, name):
        identifier = self.service(name)
        run(["docker", "start", identifier])
        wait_until(
            lambda: (
                self.inspect_owned("container", identifier)["State"]
                .get("Health", {})
                .get("Status")
                == "healthy"
            ),
            "service_restart_health",
            50,
        )

    def logs(self, name):
        result = run(["docker", "logs", self.service(name)])
        return result.stdout + result.stderr

    def sample(self):
        if self.closed:
            return {"disposable_resources_removed": True}
        result = {}
        for name in ("postgres", "gateway", "inspector"):
            if name not in self.ids:
                continue
            info = self.inspect_owned("container", self.ids[name])
            require(not info["State"].get("OOMKilled"), "fixture_oom")
            require(
                (info["HostConfig"]["Memory"], info["HostConfig"]["PidsLimit"])
                == CAPS[name],
                "fixture_caps_differ",
            )
            result[name] = {
                "running": info["State"]["Running"],
                "memory_limit": info["HostConfig"]["Memory"],
                "pids_limit": info["HostConfig"]["PidsLimit"],
            }
        return result

    def close(self):
        if self.closed:
            return
        for identifier in self.resources("container"):
            self.inspect_owned("container", identifier)
            run(["docker", "rm", "-f", identifier])
        for kind in ("network", "volume"):
            for identifier in self.resources(kind):
                info = self.inspect_owned(kind, identifier)
                if kind == "volume":
                    require(info["Name"] in self.volumes, "cleanup_volume_mismatch")
                run(["docker", kind, "rm", identifier])
        require(
            all(
                not self.resources(kind) for kind in ("container", "network", "volume")
            ),
            "fixture_teardown_incomplete",
        )
        self.closed = True

    def healthy(self, services):
        for name in services:
            info = self.inspect_owned("container", self.service(name))
            require(
                info["State"].get("Health", {}).get("Status") == "healthy",
                "fixture_not_healthy",
            )


class Matrix:
    def __init__(self, report):
        self.report = report
        self.rows, self.raw_states, self.stacks, self.processes = [], {}, [], []
        self.secret_values = []
        self.client_outputs = 0
        self.normalized_states = {}
        self.stage = "setup"
        self.working_unchanged = False
        self.working_id = backup.source_identity()
        self.working_before = backup.snapshot(self.working_id, "revenant")
        self.working_schema = backup.schema(self.working_id, "revenant")
        self.started = time.monotonic()

    def redact(self, data):
        backup.scan_material(data)
        require(
            all(value not in data for value in self.secret_values),
            "fixture_secret_disclosure",
        )

    def client(self, stack, username, role="driver", hold=None):
        env = clean_env()
        env.update(
            REVENANT_GAME_ADDR=f"127.0.0.1:{stack.game_port}",
            REVENANT_BOT_USERNAME=username,
            REVENANT_EXPECTED_PLAYERS="2",
            REVENANT_BOT_ROLE=role,
        )
        if hold:
            env.update(
                REVENANT_BOT_SECURITY_PROBE="recovery-hold",
                M30_PROBE_READY_FILE=str(hold[0]),
                M30_PROBE_GO_FILE=str(hold[1]),
            )
        process = subprocess.Popen(
            [str(ROOT / "target/debug/revenant-bot")],
            cwd=ROOT,
            env=env,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
        )
        self.processes.append(process)
        return process

    def finish_clients(self, processes, marker=None, success=True):
        for process in processes:
            output, _ = process.communicate(timeout=40)
            self.record_client(output)
            require((process.returncode == 0) == success, "client_exit_mismatch")
            if marker:
                require(marker in output, "client_marker_missing")
        time.sleep(0.2)

    def pair(self, stack, prefix):
        joins = self.join_count(stack, prefix + "-a")
        first = self.client(stack, prefix + "-a", "observer")
        wait_until(
            lambda: self.join_count(stack, prefix + "-a") > joins,
            "first_player_admission",
            8,
        )
        second = self.client(stack, prefix + "-b")
        self.finish_clients([first], b"shared completion for relay_awakening")
        self.finish_clients([second], b"activity relay_awakening completed")
        return self.session(stack, "local:" + prefix + "-a")

    def record_client(self, output):
        self.redact(output)
        self.client_outputs += 1
        with self.report.with_suffix(f".client{self.client_outputs:02}.log").open(
            "xb"
        ) as handle:
            handle.write(output)

    @staticmethod
    def join_count(stack, username):
        require(re.fullmatch(r"m30[a-z0-9-]+", username), "invalid_fixture_username")
        return int(
            stack.sql(
                f"SELECT count(*) FROM replay_events WHERE account_id='local:{username}' AND event_type='player_joined';"
            ).strip()
        )

    def cooperative(self, stack, prefix, success=True):
        env = clean_env()
        env.update(
            REVENANT_GAME_ADDR=f"127.0.0.1:{stack.game_port}",
            REVENANT_COOPERATION_ACCOUNT_PREFIX=prefix,
            REVENANT_COOPERATION_SCENARIO="success",
        )
        result = run(
            [str(ROOT / "target/debug/revenant-cooperation-bot")],
            env=env,
            check=False,
            timeout=45,
        )
        self.record_client(result.stdout + result.stderr)
        require((result.returncode == 0) == success, "cooperation_exit_mismatch")
        if success:
            require(
                b'"outcome":"succeeded"' in result.stdout, "cooperation_success_marker"
            )
        time.sleep(0.2)
        return self.session(stack, "local:" + prefix + "-a")

    @staticmethod
    def session(stack, account):
        require(
            re.fullmatch(r"local:m30[a-z0-9-]+", account), "invalid_fixture_account"
        )
        result = (
            stack.sql(
                f"SELECT session_id FROM replay_events WHERE account_id='{account}' "
                "AND event_type='player_joined' ORDER BY id DESC LIMIT 1;"
            )
            .decode()
            .strip()
        )
        require(re.fullmatch(r"session-[0-9]+", result), "fixture_session_missing")
        return result

    @staticmethod
    def rewards(stack, session, complete, participants=2):
        require(re.fullmatch(r"session-[0-9]+", session), "invalid_session")
        counts = json.loads(
            stack.sql(
                f"SELECT json_build_object("
                f"'complete',(SELECT count(*) FROM replay_events WHERE session_id='{session}' AND event_type='activity_completed'),"
                f"'loot',(SELECT count(*) FROM inventory_reward_grants WHERE session_id='{session}'),"
                f"'xp',(SELECT count(*) FROM progression_reward_grants WHERE session_id='{session}'),"
                f"'loot_replay',(SELECT count(*) FROM replay_events WHERE session_id='{session}' AND event_type='loot_granted'),"
                f"'xp_replay',(SELECT count(*) FROM replay_events WHERE session_id='{session}' AND event_type='progression_granted'));"
            ).strip()
        )
        expected = {
            "complete": int(complete),
            "loot": participants if complete else 0,
            "xp": participants if complete else 0,
            "loot_replay": participants if complete else 0,
            "xp_replay": participants if complete else 0,
        }
        require(counts == expected, "session_reward_reconciliation")

    def hold_pair(self, prefix):
        processes, go_files, ready_files = [], [], []
        for suffix in ("a", "b"):
            ready = self.private / (prefix + suffix + ".ready")
            go = self.private / (prefix + suffix + ".go")
            processes.append(
                self.client(self.source, prefix + "-" + suffix, hold=(ready, go))
            )
            if suffix == "a":
                wait_until(
                    lambda: self.join_count(self.source, prefix + "-a") == 1,
                    "held_first_player_admission",
                    8,
                )
            ready_files.append(ready)
            go_files.append(go)
        wait_until(
            lambda: all(path.is_file() for path in ready_files), "fault_rendezvous", 8
        )
        session = self.session(self.source, "local:" + prefix + "-a")
        self.rewards(self.source, session, False)
        return processes, go_files, session

    def release(self, held):
        for path in held[1]:
            path.touch(exist_ok=False)
        self.finish_clients(held[0], b'"disposition":"closed_without_success"')

    def case(
        self,
        number,
        label,
        action,
        *,
        reader=None,
        before=None,
        stable=True,
        deferred=False,
    ):
        self.stage = f"R{number:02}:{label}"
        reader = reader or self.source.capture
        before = reader() if before is None else before
        started = time.monotonic()
        row = {
            "case_id": f"R{number:02}",
            "layer": "local_recovery",
            "input_category": label,
            "expected_disposition": "verified",
            "observed_disposition": "pending",
            "initial_state_digest": backup.digest(canonical(before)),
            "post_state_digest": None,
            "protected_mutation_count": None,
            "redaction_result": "pass",
            "duration_ms": None,
            "resource_sample": None,
            "passed": False,
        }
        self.rows.append(row)
        action()
        after = None if deferred else reader()
        row.update(
            duration_ms=round((time.monotonic() - started) * 1000),
            resource_sample={
                "source": self.source.sample(),
                "restored": self.target.sample(),
            },
        )
        if after is not None:
            self.finish_row(row, before, after, stable)
        print(
            f"R{number:02} {'awaiting_restart_observation' if deferred else 'pass'}",
            flush=True,
        )
        return before

    def finish_row(self, row, before, after, stable):
        require(not stable or before == after, "undeclared_fixture_mutation")
        row.update(
            post_state_digest=backup.digest(canonical(after)),
            protected_mutation_count=changed_entries(before, after),
            passed=True,
            observed_disposition="verified",
        )
        for raw in (before, after):
            normalized = canonical(raw)
            self.normalized_states[backup.digest(normalized)] = json.loads(normalized)
        self.raw_states[row["case_id"]] = {
            "before": backup.digest(before),
            "after": backup.digest(after),
        }

    def reject_archive(self, data):
        before = self.target.capture()
        result = run(
            [
                "docker",
                "exec",
                "-i",
                self.target.service("postgres"),
                "pg_restore",
                "--no-owner",
                "--no-privileges",
                "--file=-",
            ],
            data=data,
            check=False,
        )
        require(result.returncode != 0, "corrupt_archive_accepted")
        require(self.target.capture() == before, "archive_rejection_changed_fixture")

    def reject_bundle(self, data, *, reseal, expected):
        files = backup.load_bundle(self.bundle)
        with tempfile.TemporaryDirectory(
            prefix="negative-", dir=self.private
        ) as temporary:
            directory = Path(temporary)
            changed = {**files, "archive.pgdump": data}
            for name, content in changed.items():
                backup.write_new(directory / name, content)
            hashes = changed if reseal else files
            backup.write_new(
                directory / "SHA256SUMS",
                b"".join(
                    f"{backup.digest(hashes[name])}  {name}\n".encode()
                    for name in backup.PAYLOADS
                ),
            )
            try:
                backup.restore_and_compare(backup.load_bundle(directory))
            except backup.Rejected as error:
                require(str(error) == expected, "unexpected_bundle_rejection")
            else:
                raise backup.Rejected("corrupt_bundle_accepted")

    def restore(self, database):
        require(
            re.fullmatch(r"m30r_[a-f0-9]{12}(?:_old)?", database),
            "invalid_restore_database",
        )
        require(
            not self.target.sql(
                f"SELECT 1 FROM pg_database WHERE datname='{database}';", "postgres"
            ).strip(),
            "restore_database_exists",
        )
        self.target.sql(f'CREATE DATABASE "{database}" TEMPLATE template0;', "postgres")
        run(
            [
                "docker",
                "exec",
                "-i",
                self.target.service("postgres"),
                "pg_restore",
                "-U",
                "revenant",
                "-d",
                database,
                "--single-transaction",
                "--exit-on-error",
                "--no-owner",
                "--no-privileges",
            ],
            data=self.archive,
        )

    def compare_target(self, database=None):
        database = database or self.target.database
        require(
            backup.snapshot(self.target.service("postgres"), database)
            == self.sealed_state,
            "restored_rows_differ",
        )
        require(
            backup.schema(self.target.service("postgres"), database)
            == self.sealed_schema,
            "restored_schema_differs",
        )

    def migrate_target(self):
        ddl = (
            b"BEGIN;\nSELECT pg_advisory_xact_lock(824180018);\n"
            + b"\n".join(backup.migrations().values())
            + b"\nCOMMIT;"
        )
        self.target.sql(ddl.decode())
        self.compare_target()

    def terminal_retry(self):
        env = clean_env()
        env.update(
            CARGO_HOME=str(ROOT / ".tooling/cargo"),
            RUSTUP_HOME=str(ROOT / ".tooling/rustup"),
            DATABASE_URL_FILE=str(self.source.secret_dir / "operator_database_url"),
            M30_RETRY_SESSION=self.seed_coop,
        )
        env["PATH"] = env["CARGO_HOME"] + "/bin:" + env["PATH"]
        output = run(
            [
                "cargo",
                "test",
                "-p",
                "revenant-persistence",
                "--test",
                "m30_terminal_retry",
                "committed_cooperation_retry_preserves_rewards_and_replay",
                "--",
                "--ignored",
                "--exact",
                "--nocapture",
            ],
            env=env,
        ).stdout
        self.redact(output)
        require(
            b"M30_TERMINAL_RETRY participants=2 attempts=2 disposition=replayed"
            in output,
            "terminal_retry_not_executed",
        )

    def inspect_session(self, stack, session, *, cooperative=False):
        api = http_json(
            stack.inspector_port,
            f"/api/inspector/sessions/{session}/summary",
            stack.origin,
        )
        summary = api["summary"]
        require(
            summary["completed"]
            and summary["participant_count"] == 2
            and summary["loot_grant_count"] == 2
            and summary["progression_grant_count"] == 2,
            "inspector_reward_summary",
        )
        if cooperative:
            require(
                summary["cooperation_terminal_outcome"] == "succeeded"
                and summary["cooperation_revive_count"] == 1,
                "inspector_cooperation_summary",
            )
        self.rewards(stack, session, True)
        events = http_json(
            stack.inspector_port,
            f"/api/inspector/sessions/{session}/events",
            stack.origin,
        )
        require(
            sum(
                event["event_type"] == "activity_completed"
                for event in events["events"]
            )
            == 1,
            "inspector_replay_completion",
        )

    def execute(self):
        with tempfile.TemporaryDirectory(prefix="revenant-m30-recovery-") as temporary:
            self.private = Path(temporary)
            try:
                self.source = Stack(self.private)
                self.stacks.append(self.source)
                self.secret_values += self.source.secrets
                self.target = Stack(self.private, restored=True)
                self.stacks.append(self.target)
                self.secret_values += self.target.secrets
                self.source.up()
                self.target.up(postgres_only=True)
                self.seed_plain = self.pair(self.source, "m30seed")
                self.seed_coop = self.cooperative(self.source, "m30coop")
                self.rewards(self.source, self.seed_plain, True)
                self.rewards(self.source, self.seed_coop, True)
                self.sealed_state, self.sealed_schema = (
                    self.source.capture(),
                    self.source.schema(),
                )
                self.case(
                    1,
                    "target_and_health_preflight",
                    lambda: (
                        self.source.healthy(("postgres", "gateway", "inspector")),
                        self.target.healthy(("postgres",)),
                    ),
                )

                def dump():
                    self.archive = run(
                        [
                            "docker",
                            "exec",
                            self.source.service("postgres"),
                            "pg_dump",
                            "-U",
                            "revenant",
                            "-d",
                            self.source.database,
                            "--format=custom",
                            "--no-privileges",
                        ]
                    ).stdout
                    require(self.archive.startswith(b"PGDMP"), "custom_dump_magic")
                    self.archive_hash = backup.digest(self.archive)
                    self.bundle = self.private / "sealed"
                    self.bundle.mkdir(mode=0o700)
                    payload = {
                        "archive.pgdump": self.archive,
                        "schema.sql": self.sealed_schema,
                        "state.json": self.sealed_state,
                        "manifest.json": backup.encoded(
                            {
                                "format": "revenant.m30.backup.v1",
                                "postgres_image": backup.IMAGE,
                                "migrations": {
                                    name: backup.digest(data)
                                    for name, data in backup.migrations().items()
                                },
                                "state_sha256": backup.digest(self.sealed_state),
                                "schema_sha256": backup.digest(self.sealed_schema),
                            }
                        ),
                    }
                    for name, data in payload.items():
                        backup.write_new(self.bundle / name, data)
                    backup.write_new(
                        self.bundle / "SHA256SUMS",
                        b"".join(
                            f"{backup.digest(payload[name])}  {name}\n".encode()
                            for name in backup.PAYLOADS
                        ),
                    )

                self.case(2, "custom_dump", dump)
                self.case(
                    3, "checksum_verification", lambda: backup.load_bundle(self.bundle)
                )

                self.case(
                    4,
                    "restore_list_verification",
                    lambda: require(
                        b"Format: CUSTOM"
                        in run(
                            [
                                "docker",
                                "exec",
                                "-i",
                                self.target.service("postgres"),
                                "pg_restore",
                                "--list",
                            ],
                            data=self.archive,
                        ).stdout,
                        "archive_list",
                    ),
                )
                self.case(
                    5,
                    "wrong_checksum_refused",
                    lambda: self.reject_bundle(
                        self.archive + b"corruption",
                        reseal=False,
                        expected="checksum_mismatch",
                    ),
                )
                self.case(
                    6,
                    "truncated_archive_refused",
                    lambda: self.reject_bundle(
                        self.archive[: len(self.archive) // 2],
                        reseal=True,
                        expected="command_failed",
                    ),
                )
                self.case(
                    7,
                    "checked_absent_restore_database",
                    lambda: require(
                        not self.target.sql(
                            f"SELECT 1 FROM pg_database WHERE datname='{self.target.database}';",
                            "postgres",
                        ).strip(),
                        "restore_target_exists",
                    ),
                )
                self.restore_started = time.monotonic()
                self.case(
                    8,
                    "transactional_restore",
                    lambda: self.restore(self.target.database),
                )
                self.case(
                    9,
                    "schema_and_object_cardinality",
                    lambda: require(
                        self.target.schema() == self.sealed_schema, "schema_cardinality"
                    ),
                )
                self.case(10, "canonical_rows_and_replay_equality", self.compare_target)
                self.case(
                    11,
                    "archive_secret_material_absent",
                    lambda: self.redact(
                        run(
                            [
                                "docker",
                                "exec",
                                "-i",
                                self.target.service("postgres"),
                                "pg_restore",
                                "--file=-",
                                "--no-owner",
                                "--no-privileges",
                            ],
                            data=self.archive,
                        ).stdout
                    ),
                )
                self.case(12, "working_database_untouched", self.check_working)

                held = self.hold_pair("m30gateway")

                def kill_gateway():
                    self.source.stop("gateway", kill=True)
                    self.release(held)
                    self.rewards(self.source, held[2], False)

                self.case(13, "gateway_killed_during_incomplete_activity", kill_gateway)

                def restart_gateway():
                    self.source.start("gateway")
                    fresh = self.pair(self.source, "m30gateway")
                    require(fresh != held[2], "gateway_retry_reused_session")
                    self.rewards(self.source, held[2], False)
                    self.rewards(self.source, fresh, True)

                self.case(
                    14, "gateway_restart_and_fresh_retry", restart_gateway, stable=False
                )

                held_pg = self.hold_pair("m30postgres")
                gateway_id = self.source.service("gateway")
                gateway_started = self.source.inspect_owned("container", gateway_id)[
                    "State"
                ]["StartedAt"]

                def stop_postgres():
                    self.source.stop("postgres")
                    self.release(held_pg)

                pg_before = self.case(
                    15,
                    "postgres_stopped_during_active_activity",
                    stop_postgres,
                    deferred=True,
                )

                def restart_postgres():
                    self.source.start("postgres")
                    after = self.source.capture()
                    self.finish_row(self.rows[-2], pg_before, after, True)
                    self.rows[-2]["post_state_observation"] = (
                        "after_R16_postgres_restart"
                    )
                    self.rewards(self.source, held_pg[2], False)

                self.case(
                    16,
                    "postgres_restart_retains_committed_state",
                    restart_postgres,
                    before=pg_before,
                )

                def reconnect():
                    fresh = self.pair(self.source, "m30postgres")
                    require(fresh != held_pg[2], "postgres_retry_reused_session")
                    require(
                        self.source.inspect_owned("container", gateway_id)["State"][
                            "StartedAt"
                        ]
                        == gateway_started,
                        "gateway_restarted_during_persistence_recovery",
                    )
                    require(
                        b"session_persistence_reconnected"
                        in self.source.logs("gateway"),
                        "persistence_reconnect_not_observed",
                    )
                    self.rewards(self.source, fresh, True)
                    self.rewards(self.source, held_pg[2], False)

                self.case(
                    17, "same_gateway_persistence_reconnect", reconnect, stable=False
                )

                def inspector_restart():
                    self.source.stop("inspector", kill=True)
                    self.source.start("inspector")
                    self.inspect_session(self.source, self.seed_coop, cooperative=True)

                self.case(18, "inspector_kill_restart", inspector_restart)

                def rollback():
                    self.source.sql(
                        "CREATE FUNCTION m30_fail_second_reward() RETURNS trigger LANGUAGE plpgsql AS $$ "
                        "BEGIN IF NEW.account_id='local:m30rollback-r' AND NEW.event_type='progression_granted' "
                        "THEN RAISE EXCEPTION 'm30 disposable failure'; END IF; RETURN NEW; END; $$; "
                        "CREATE TRIGGER m30_fail_second_reward BEFORE INSERT ON replay_events FOR EACH ROW EXECUTE FUNCTION m30_fail_second_reward();"
                    )
                    try:
                        failed = self.cooperative(
                            self.source, "m30rollback", success=False
                        )
                        self.rewards(self.source, failed, False)
                        require(
                            b"m30 disposable failure" in self.source.logs("postgres"),
                            "rollback_fault_not_reached",
                        )
                        require(
                            self.source.sql(
                                "SELECT COALESCE(sum(quantity),0) FROM inventory WHERE item_id='relay_core_fragment' AND character_id IN ('local:m30rollback-a:operator','local:m30rollback-r:operator');"
                            ).strip()
                            == b"0",
                            "partial_inventory",
                        )
                        require(
                            self.source.sql(
                                "SELECT count(*) FROM activity_history WHERE account_id IN ('local:m30rollback-a','local:m30rollback-r');"
                            ).strip()
                            == b"0",
                            "partial_history",
                        )
                        require(
                            self.source.sql(
                                "SELECT sum(experience) FROM progression WHERE character_id IN ('local:m30rollback-a:operator','local:m30rollback-r:operator');"
                            ).strip()
                            == b"0",
                            "partial_xp",
                        )
                    finally:
                        self.source.sql(
                            "DROP TRIGGER m30_fail_second_reward ON replay_events; DROP FUNCTION m30_fail_second_reward();"
                        )
                    fresh = self.cooperative(self.source, "m30rollback")
                    require(fresh != failed, "rollback_retry_reused_session")
                    self.rewards(self.source, fresh, True)
                    self.rewards(self.source, failed, False)

                self.case(
                    19,
                    "second_participant_transaction_failure_and_retry",
                    rollback,
                    stable=False,
                )
                self.case(
                    20, "committed_terminal_retry_has_no_duplicate", self.terminal_retry
                )
                corrupt = b"PGDMP" + bytes(96)
                self.case(
                    21, "corrupt_archive_rejected", lambda: self.reject_archive(corrupt)
                )
                self.target.sql(
                    "UPDATE progression SET experience=experience+1 WHERE character_id='local:m30seed-a:operator';"
                )

                def detect_corruption():
                    try:
                        self.compare_target()
                    except backup.Rejected as error:
                        require(
                            str(error) == "restored_rows_differ",
                            "unexpected_database_rejection",
                        )
                    else:
                        raise backup.Rejected("corrupt_database_not_detected")

                self.case(
                    22,
                    "corrupt_disposable_database_detected",
                    detect_corruption,
                    reader=self.target.capture,
                )
                self.target.sql(
                    "UPDATE progression SET experience=experience-1 WHERE character_id='local:m30seed-a:operator';"
                )
                self.compare_target()
                self.case(
                    23,
                    "pre_migration_sealed_snapshot",
                    self.compare_target,
                    reader=self.target.capture,
                )
                self.case(
                    24,
                    "candidate_migrations_once",
                    self.migrate_target,
                    reader=self.target.capture,
                )
                self.case(
                    25,
                    "candidate_migrations_twice",
                    self.migrate_target,
                    reader=self.target.capture,
                )
                old_database = self.target.database + "_old"

                def restore_old():
                    self.restore(old_database)
                    self.compare_target(old_database)

                self.case(
                    26,
                    "pre_migration_disposable_restore",
                    restore_old,
                    reader=self.target.capture,
                )
                self.case(
                    27,
                    "legacy_exports_byte_identical",
                    self.compare_target,
                    reader=self.target.capture,
                )

                def reconcile_application():
                    self.target.up()
                    self.compare_target()
                    self.inspect_session(self.target, self.seed_plain)
                    self.inspect_session(self.target, self.seed_coop, cooperative=True)
                    plain = self.pair(self.target, "m30restored")
                    coop = self.cooperative(self.target, "m30restoredcoop")
                    self.inspect_session(self.target, plain)
                    self.inspect_session(self.target, coop, cooperative=True)
                    self.recovery_ms = round(
                        (time.monotonic() - self.restore_started) * 1000
                    )

                self.case(
                    28,
                    "restored_gateway_inspector_rewards_and_replay",
                    reconcile_application,
                    reader=self.target.capture,
                    stable=False,
                )
                self.case(
                    29,
                    "complete_restore_to_application_health_within_600_seconds",
                    lambda: require(
                        self.recovery_ms <= 600000, "recovery_objective_exceeded"
                    ),
                )
                self.capture_logs()
                before = self.source.capture()

                def teardown():
                    for stack in reversed(self.stacks):
                        stack.close()
                    self.check_working()

                self.case(
                    30,
                    "exact_disposable_teardown_working_volume_retained",
                    teardown,
                    before=before,
                    reader=lambda: b"{}",
                    stable=False,
                )
                require(
                    [row["case_id"] for row in self.rows]
                    == [f"R{i:02}" for i in range(1, 31)]
                    and all(row["passed"] for row in self.rows),
                    "matrix_not_complete",
                )
            finally:
                try:
                    self.capture_logs()
                finally:
                    for process in self.processes:
                        if process.poll() is None:
                            process.kill()
                            process.communicate(timeout=5)
                    for stack in reversed(self.stacks):
                        stack.close()
                    self.check_working()

    def capture_logs(self):
        for index, stack in enumerate(self.stacks):
            if stack.closed:
                continue
            for service in stack.ids:
                path = self.report.with_suffix(f".stack{index}.{service}.log")
                if path.exists():
                    continue
                logs = stack.logs(service)
                self.redact(logs)
                with path.open("xb") as handle:
                    handle.write(logs)

    def check_working(self):
        self.working_unchanged = False
        require(
            backup.source_identity() == self.working_id
            and backup.snapshot(self.working_id, "revenant") == self.working_before
            and backup.schema(self.working_id, "revenant") == self.working_schema,
            "working_database_changed",
        )
        self.working_unchanged = True

    def save(self, failure=None):
        payload = {
            "schema": "revenant.m30.recovery-gate.v1",
            "rows": self.rows,
            "raw_state_digests": self.raw_states,
            "working_state_digest": backup.digest(self.working_before),
            "working_data_unchanged": self.working_unchanged,
            "recovery_duration_ms": getattr(self, "recovery_ms", None),
            "duration_ms": round((time.monotonic() - self.started) * 1000),
            "passed": failure is None
            and len(self.rows) == 30
            and all(row["passed"] for row in self.rows),
            "failure": failure,
            "failure_stage": self.stage if failure else None,
        }
        data = backup.encoded(payload)
        self.redact(data)
        with self.report.open("xb") as handle:
            handle.write(data)
        states = backup.encoded(self.normalized_states)
        self.redact(states)
        with self.report.with_suffix(".states.json").open("xb") as handle:
            handle.write(states)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--report", type=Path, required=True)
    args = parser.parse_args()
    require(
        args.report.is_absolute()
        and args.report.parent.is_dir()
        and not args.report.exists(),
        "new_absolute_report_required",
    )
    for port in (15451, 17451, 18451, 41451, 17452, 18452, 41452):
        with socket.socket() as probe:
            probe.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
            probe.bind(("127.0.0.1", port))
    os.umask(0o077)
    matrix = Matrix(args.report)

    def interrupted(_number, _frame):
        raise backup.Rejected("interrupted")

    signal.signal(signal.SIGTERM, interrupted)
    try:
        matrix.execute()
    except (
        backup.Rejected,
        OSError,
        ValueError,
        KeyError,
        TypeError,
        IndexError,
        subprocess.TimeoutExpired,
        KeyboardInterrupt,
    ) as error:
        category = (
            str(error) if isinstance(error, backup.Rejected) else type(error).__name__
        )
        matrix.save(category)
        print(
            json.dumps(
                {"result": "failed", "category": category, "stage": matrix.stage}
            ),
            flush=True,
        )
        return 1
    matrix.save()
    print("M30 recovery matrix: 30/30 passed", flush=True)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
