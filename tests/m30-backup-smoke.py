#!/usr/bin/env python3
"""Real disposable restore and corruption checks for an existing private backup.

This is the Gate 5 first-slice smoke, not the R01-R30 acceptance matrix.
"""

import argparse
import importlib.util
import json
import os
import tempfile
from pathlib import Path

SPEC = importlib.util.spec_from_file_location(
    "m30_backup", Path(__file__).resolve().parents[1] / "scripts/m30-backup.py"
)
backup = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(backup)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("directory", type=Path)
    args = parser.parse_args()
    os.umask(0o077)
    source = backup.source_identity()
    before = backup.snapshot(source, "revenant")
    before_schema = backup.schema(source, "revenant")
    files = backup.load_bundle(args.directory)
    rows = [
        {
            "case": "valid_restore_and_two_migrations",
            **backup.restore_and_compare(files),
        }
    ]
    cases = (
        (
            "wrong_checksum",
            "archive.pgdump",
            b"PGDMP-modified",
            "checksum_mismatch",
            False,
        ),
        (
            "truncated_archive_with_matching_checksum",
            "archive.pgdump",
            files["archive.pgdump"][: len(files["archive.pgdump"]) // 2],
            "command_failed",
            True,
        ),
        (
            "schema_mismatch_with_matching_checksum",
            "schema.sql",
            b"-- incorrect schema\n",
            "restored_schema_differs",
            True,
        ),
        (
            "row_mismatch_with_matching_checksum",
            "state.json",
            b"{}\n",
            "restored_rows_or_sequences_differ",
            True,
        ),
    )
    for label, name, corrupted, expected, reseal in cases:
        with tempfile.TemporaryDirectory(
            prefix="revenant-backup-negative-"
        ) as temporary:
            directory = Path(temporary)
            changed = dict(files)
            changed[name] = corrupted
            if reseal:
                manifest = json.loads(changed["manifest.json"])
                manifest["state_sha256"] = backup.digest(changed["state.json"])
                manifest["schema_sha256"] = backup.digest(changed["schema.sql"])
                changed["manifest.json"] = backup.encoded(manifest)
            for filename, data in changed.items():
                backup.write_new(directory / filename, data)
            hashed = changed if reseal else files
            backup.write_new(
                directory / "SHA256SUMS",
                b"".join(
                    f"{backup.digest(hashed[filename])}  {filename}\n".encode()
                    for filename in backup.PAYLOADS
                ),
            )
            try:
                backup.restore_and_compare(backup.load_bundle(directory))
            except backup.Rejected as error:
                backup.require(str(error) == expected, "unexpected_rejection_category")
                rows.append(
                    {
                        "case": label,
                        "expected": expected,
                        "observed": str(error),
                        "passed": True,
                    }
                )
            else:
                raise backup.Rejected("corruption_was_accepted")
    backup.require(
        source == backup.source_identity()
        and before == backup.snapshot(source, "revenant")
        and before_schema == backup.schema(source, "revenant"),
        "working_source_changed",
    )
    backup.require(
        not backup.run(
            ["docker", "ps", "-aq", "--filter", "name=m30g5restore-"]
        ).strip(),
        "disposable_container_leftover",
    )
    print(
        json.dumps(
            {
                "format": "revenant.m30.backup-first-slice-smoke.v1",
                "rows": rows,
                "working_source_unchanged": True,
                "working_state_sha256": backup.digest(before),
                "disposable_containers_remaining": 0,
            },
            sort_keys=True,
        )
    )


if __name__ == "__main__":
    main()
