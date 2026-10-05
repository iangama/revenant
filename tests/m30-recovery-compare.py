#!/usr/bin/env python3
"""Compare two complete Gate 5 reports using the frozen observation allowlist."""

import argparse
import hashlib
import json
from pathlib import Path


def normalized(path):
    report = json.loads(path.read_bytes())
    if (
        report.get("schema") != "revenant.m30.recovery-gate.v1"
        or report.get("passed") is not True
        or report.get("working_data_unchanged") is not True
        or not 0 < report.get("recovery_duration_ms", 0) <= 600000
    ):
        raise ValueError("incomplete_recovery_report")
    rows = report["rows"]
    if [row["case_id"] for row in rows] != [f"R{case:02}" for case in range(1, 31)]:
        raise ValueError("wrong_recovery_case_inventory")
    if not all(row["passed"] and row["redaction_result"] == "pass" for row in rows):
        raise ValueError("failed_recovery_case")
    # Row digests already normalize only declared generated session IDs and
    # wall/observed elapsed timestamps. Rewards, sequence state and gameplay
    # constants remain protected. Raw equality was required by each live case.
    result = [
        {
            key: value
            for key, value in row.items()
            if key not in {"duration_ms", "resource_sample"}
        }
        for row in rows
    ]
    return json.dumps(result, sort_keys=True, separators=(",", ":")).encode() + b"\n"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("first", type=Path)
    parser.add_argument("second", type=Path)
    args = parser.parse_args()
    first, second = normalized(args.first), normalized(args.second)
    if first != second:
        left, right = json.loads(first), json.loads(second)
        differing = [a["case_id"] for a, b in zip(left, right, strict=True) if a != b]
        print(json.dumps({"passed": False, "differing_cases": differing}))
        return 1
    print(
        json.dumps(
            {
                "passed": True,
                "cases_per_run": 30,
                "normalized_byte_identical": True,
                "normalized_sha256": hashlib.sha256(first).hexdigest(),
            },
            sort_keys=True,
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
