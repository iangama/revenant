"""Safety boundaries for the owner backup command; no live database mutations."""

import copy
import importlib.util
import json
import os
import subprocess
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

SPEC = importlib.util.spec_from_file_location(
    "m30_backup", Path(__file__).resolve().parents[1] / "scripts/m30-backup.py"
)
backup = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(backup)


class BundleTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.directory = Path(self.temporary.name)
        self.payload = {
            "archive.pgdump": b"PGDMP-synthetic-fixture",
            "schema.sql": b"schema",
            "state.json": b"{}",
        }
        self.payload["manifest.json"] = backup.encoded(
            {
                "format": "revenant.m30.backup.v1",
                "postgres_image": backup.IMAGE,
                "migrations": {
                    name: backup.digest(data)
                    for name, data in backup.migrations().items()
                },
                "state_sha256": backup.digest(self.payload["state.json"]),
                "schema_sha256": backup.digest(self.payload["schema.sql"]),
            }
        )
        for name, data in self.payload.items():
            backup.write_new(self.directory / name, data)
        self.seal()

    def seal(self):
        path = self.directory / "SHA256SUMS"
        path.unlink(missing_ok=True)
        backup.write_new(
            path,
            b"".join(
                f"{backup.digest((self.directory / name).read_bytes())}  {name}\n".encode()
                for name in backup.PAYLOADS
            ),
        )

    def test_checked_buffers_do_not_follow_later_file_changes(self):
        files = backup.load_bundle(self.directory)
        (self.directory / "archive.pgdump").write_bytes(b"replaced")
        self.assertEqual(files["archive.pgdump"], self.payload["archive.pgdump"])

    def test_tampered_archive_rejected_before_restore(self):
        (self.directory / "archive.pgdump").write_bytes(b"PGDMP-corrupt")
        with self.assertRaisesRegex(backup.Rejected, "checksum_mismatch"):
            backup.load_bundle(self.directory)

    def test_checksum_traversal_is_never_interpreted(self):
        (self.directory / "SHA256SUMS").write_bytes(b"0" * 64 + b"  ../outside\n")
        with self.assertRaisesRegex(backup.Rejected, "checksum_mismatch"):
            backup.load_bundle(self.directory)

    def test_resealed_wrong_archive_format_rejected(self):
        (self.directory / "archive.pgdump").write_bytes(b"SQL is not a custom archive")
        self.seal()
        with self.assertRaisesRegex(backup.Rejected, "custom_archive_required"):
            backup.load_bundle(self.directory)

    def test_manifest_does_not_allow_another_image(self):
        path = self.directory / "manifest.json"
        manifest = json.loads(path.read_bytes())
        manifest["postgres_image"] = "postgres:latest"
        path.write_bytes(backup.encoded(manifest))
        self.seal()
        with self.assertRaisesRegex(backup.Rejected, "backup_format_mismatch"):
            backup.load_bundle(self.directory)

    def test_migration_revision_mismatch_rejects_before_ddl(self):
        with (
            patch.object(
                backup, "migrations", return_value={"changed.sql": b"candidate"}
            ),
            self.assertRaisesRegex(
                backup.Rejected, "backup_migration_revision_mismatch"
            ),
        ):
            backup.load_bundle(self.directory)

    def test_symlink_payload_rejected(self):
        path = self.directory / "schema.sql"
        path.unlink()
        path.symlink_to(self.directory / "state.json")
        with self.assertRaisesRegex(backup.Rejected, "nonsymlink"):
            backup.load_bundle(self.directory)

    def test_permissive_directory_and_file_rejected(self):
        for path, is_directory in (
            (self.directory, True),
            (self.directory / "state.json", False),
        ):
            with self.subTest(directory=is_directory):
                path.chmod(0o755 if is_directory else 0o644)
                with self.assertRaisesRegex(backup.Rejected, "private_mode_required"):
                    backup.load_bundle(self.directory)
                path.chmod(0o700 if is_directory else 0o600)

    def test_unexpected_extra_file_and_incomplete_seal_rejected(self):
        extra = self.directory / "unexpected"
        extra.touch()
        with self.assertRaisesRegex(backup.Rejected, "bundle_inventory_mismatch"):
            backup.load_bundle(self.directory)
        extra.unlink()
        (self.directory / "SHA256SUMS").unlink()
        with self.assertRaisesRegex(backup.Rejected, "bundle_inventory_mismatch"):
            backup.load_bundle(self.directory)

    def test_existing_backup_is_not_overwritten(self):
        with patch.object(backup, "source_identity") as source:
            with self.assertRaisesRegex(backup.Rejected, "backup_directory_exists"):
                backup.create(self.directory)
            source.assert_not_called()

    def test_file_size_limit(self):
        with patch.object(backup, "MAX_FILE_BYTES", 2):
            self.assertEqual(backup.read_private(self.directory / "state.json"), b"{}")
            (self.directory / "state.json").write_bytes(b"{}\n")
            with self.assertRaisesRegex(backup.Rejected, "file_size_limit"):
                backup.read_private(self.directory / "state.json")

    def test_known_secret_scan_does_not_echo_matching_value(self):
        for name in ("current", "custom-generation"):
            current = self.directory / name
            current.mkdir(mode=0o700)
            value = os.urandom(24).hex().encode()
            backup.write_new(current / "postgres_admin_password", value)
            with patch.dict(os.environ, {"REVENANT_SECRETS_DIR": str(current)}):
                with self.assertRaisesRegex(
                    backup.Rejected, "known_secret_detected"
                ) as caught:
                    backup.scan_material(b"row " + value)
                self.assertNotIn(value.decode(), str(caught.exception))


class TargetTests(unittest.TestCase):
    def test_source_identity_rejects_wrong_volume_labels_image_health_and_port(self):
        source = {
            "Id": "a" * 64,
            "Config": {
                "Image": backup.IMAGE,
                "Labels": {
                    "com.docker.compose.project": "infra",
                    "com.docker.compose.service": "postgres",
                },
            },
            "State": {"Health": {"Status": "healthy"}},
            "HostConfig": {"PortBindings": {}},
            "Mounts": [
                {
                    "Destination": "/var/lib/postgresql/data",
                    "Type": "volume",
                    "Name": "infra_revenant-postgres",
                }
            ],
        }
        variants = []
        wrong = copy.deepcopy(source)
        wrong["Mounts"][0]["Name"] = "unrelated-data"
        variants.append(wrong)
        wrong = copy.deepcopy(source)
        wrong["Config"]["Labels"]["com.docker.compose.project"] = "another-project"
        variants.append(wrong)
        wrong = copy.deepcopy(source)
        wrong["Config"]["Image"] = "postgres:latest"
        variants.append(wrong)
        wrong = copy.deepcopy(source)
        wrong["State"]["Health"]["Status"] = "unhealthy"
        variants.append(wrong)
        wrong = copy.deepcopy(source)
        wrong["HostConfig"]["PortBindings"] = {"5432/tcp": []}
        variants.append(wrong)
        for wrong in variants:
            with (
                self.subTest(source=wrong),
                patch.object(backup, "run", return_value=backup.encoded([wrong])),
            ):
                with (
                    self.assertRaises(backup.Rejected),
                    patch.object(backup, "sql") as sql,
                ):
                    backup.source_identity()
                sql.assert_not_called()

    def test_name_collision_does_not_adopt_or_delete_container(self):
        target = backup.Disposable()
        with patch.object(
            backup, "run", return_value=(target.name + "\n").encode()
        ) as run:
            with self.assertRaisesRegex(backup.Rejected, "disposable_container_exists"):
                target.__enter__()
            self.assertEqual(run.call_count, 1)
            self.assertIsNone(target.container)

    def test_cleanup_does_not_remove_foreign_container_or_data_mount(self):
        for info in (
            {"Config": {"Labels": {"revenant.m30.restore": "another-run"}}},
            {
                "Config": {"Labels": {}},
                "HostConfig": {"NetworkMode": "none"},
                "Mounts": [{"Name": "infra_revenant-postgres"}],
            },
        ):
            target = backup.Disposable()
            target.container = "a" * 64
            if not info["Config"]["Labels"]:
                info["Config"]["Labels"]["revenant.m30.restore"] = target.token
            with patch.object(
                backup, "run", return_value=backup.encoded([info])
            ) as run:
                with self.assertRaises(backup.Rejected):
                    target.close()
                self.assertEqual(run.call_count, 1)

    def test_restore_failure_cleans_up_only_created_container(self):
        target = backup.Disposable()
        target.container = "a" * 64
        info = {
            "Config": {"Labels": {"revenant.m30.restore": target.token}},
            "HostConfig": {"NetworkMode": "none"},
            "Mounts": [],
        }
        with patch.object(
            backup, "run", side_effect=[backup.encoded([info]), b""]
        ) as run:
            target.__exit__(RuntimeError, RuntimeError("restore failed"), None)
            self.assertEqual(
                run.call_args_list[-1].args[0], ["docker", "rm", "-f", "a" * 64]
            )
            self.assertIsNone(target.container)

    def test_interrupted_create_recovers_only_its_ownership_label(self):
        target = backup.Disposable()
        info = {
            "Config": {"Labels": {"revenant.m30.restore": target.token}},
            "HostConfig": {"NetworkMode": "none"},
            "Mounts": [],
        }
        with patch.object(
            backup,
            "run",
            side_effect=[
                b"",
                KeyboardInterrupt(),
                ("a" * 64 + "\n").encode(),
                backup.encoded([info]),
                b"",
            ],
        ) as run:
            with self.assertRaises(KeyboardInterrupt):
                target.__enter__()
            self.assertIn(
                "label=revenant.m30.restore=" + target.token,
                run.call_args_list[2].args[0],
            )
            self.assertEqual(
                run.call_args_list[-1].args[0], ["docker", "rm", "-f", "a" * 64]
            )
            self.assertIsNone(target.container)

    def test_subprocess_diagnostics_never_disclose_stderr(self):
        result = subprocess.CompletedProcess(
            ["tool"], 1, b"", b"sensitive internal error"
        )
        with (
            patch.object(subprocess, "run", return_value=result),
            self.assertRaisesRegex(backup.Rejected, "^command_failed$"),
        ):
            backup.run(["tool"])


if __name__ == "__main__":
    unittest.main()
