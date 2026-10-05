"""Safety boundaries of the destructive disposable recovery harness."""

import importlib.util
import json
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest.mock import patch

SPEC = importlib.util.spec_from_file_location(
    "m30_recovery", Path(__file__).with_name("m30-recovery-matrix.py")
)
matrix = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(matrix)


class RecoverySafetyTests(unittest.TestCase):
    def test_normalization_only_removes_declared_observations(self):
        before = {
            "replay_events": [
                {
                    "session_id": "session-12345",
                    "occurred_at": "today",
                    "payload": '{"elapsed_ms":123,"quantity":2}',
                }
            ]
        }
        after = {
            "replay_events": [
                {
                    "session_id": "session-98765",
                    "occurred_at": "tomorrow",
                    "payload": '{"elapsed_ms":456,"quantity":2}',
                }
            ]
        }
        encode = matrix.backup.encoded
        self.assertEqual(
            matrix.canonical(encode(before)), matrix.canonical(encode(after))
        )
        after["replay_events"][0]["payload"] = '{"elapsed_ms":456,"quantity":3}'
        self.assertNotEqual(
            matrix.canonical(encode(before)), matrix.canonical(encode(after))
        )

    def test_payload_array_order_is_protected(self):
        before = b'{"replay_events":[{"payload":"{\\"route\\":[1,2]}"}]}'
        after = b'{"replay_events":[{"payload":"{\\"route\\":[2,1]}"}]}'
        self.assertNotEqual(matrix.canonical(before), matrix.canonical(after))

    def test_sequence_and_duplicate_changes_remain_visible(self):
        before = b'{"replay_events":[{"id":1}],"replay_events_id_seq":{"last_value":1}}'
        after = b'{"replay_events":[{"id":1},{"id":1}],"replay_events_id_seq":{"last_value":2}}'
        self.assertEqual(matrix.changed_entries(before, after), 3)
        self.assertNotEqual(matrix.canonical(before), matrix.canonical(after))

    def fixture(self):
        stack = matrix.Stack.__new__(matrix.Stack)
        stack.project, stack.token, stack.volumes = "m30g5abc", "abc", {"m30g5abc_data"}
        info = {
            "Config": {
                "Labels": {
                    matrix.OWNER_LABEL: "abc",
                    "com.docker.compose.project": "m30g5abc",
                }
            },
            "Mounts": [{"Type": "volume", "Name": "m30g5abc_data"}],
            "HostConfig": {"PortBindings": {"5432/tcp": [{"HostIp": "127.0.0.1"}]}},
        }
        return stack, info

    def assert_rejected(self, stack, info, category):
        with (
            patch.object(
                matrix,
                "run",
                return_value=SimpleNamespace(stdout=json.dumps([info]).encode()),
            ),
            self.assertRaisesRegex(matrix.backup.Rejected, category),
        ):
            stack.inspect_owned("container", "fixture")

    def test_foreign_owner_refused_before_destructive_action(self):
        stack, info = self.fixture()
        info["Config"]["Labels"][matrix.OWNER_LABEL] = "someone-else"
        self.assert_rejected(stack, info, "fixture_ownership_mismatch")

    def test_working_volume_refused_even_with_fixture_labels(self):
        stack, info = self.fixture()
        info["Mounts"][0]["Name"] = "infra_revenant-postgres"
        self.assert_rejected(stack, info, "foreign_data_volume")

    def test_public_port_refused(self):
        stack, info = self.fixture()
        info["HostConfig"]["PortBindings"]["5432/tcp"][0]["HostIp"] = "0.0.0.0"
        self.assert_rejected(stack, info, "non_loopback_fixture")

    def test_working_database_cannot_be_restore_target(self):
        recovery = matrix.Matrix.__new__(matrix.Matrix)
        with self.assertRaisesRegex(matrix.backup.Rejected, "invalid_restore_database"):
            recovery.restore("revenant")

    def test_normalized_match_cannot_hide_raw_mutation(self):
        recovery = matrix.Matrix.__new__(matrix.Matrix)
        with self.assertRaisesRegex(
            matrix.backup.Rejected, "undeclared_fixture_mutation"
        ):
            recovery.finish_row(
                {},
                b'{"replay_events":[{"elapsed_ms":1}]}',
                b'{"replay_events":[{"elapsed_ms":2}]}',
                True,
            )


if __name__ == "__main__":
    unittest.main()
