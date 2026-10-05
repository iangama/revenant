#!/usr/bin/env python3
"""Verify every archived payload against its local SHA-256 manifest."""

import hashlib
import re
import sys
from pathlib import Path


def digest(path):
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def verify(root):
    root = root.resolve()
    expected = set()
    for line in (root / "SHA256SUMS").read_text().splitlines():
        match = re.fullmatch(r"([0-9a-f]{64})  (.+)", line)
        if match is None:
            raise ValueError("malformed checksum manifest")
        checksum, name = match.groups()
        path = root / name
        if (
            name in expected
            or Path(name).is_absolute()
            or ".." in Path(name).parts
            or "\\" in name
            or path.is_symlink()
            or not path.resolve().is_relative_to(root)
            or not path.is_file()
        ):
            raise ValueError("invalid, duplicate, or missing payload: " + name)
        if digest(path) != checksum:
            raise ValueError("checksum mismatch: " + name)
        expected.add(name)
    actual = {
        p.relative_to(root).as_posix()
        for p in root.rglob("*")
        if p.is_file() and p != root / "SHA256SUMS"
    }
    if not expected or expected != actual:
        raise ValueError("empty manifest or unsealed extra files")
    return len(expected)


if __name__ == "__main__":
    try:
        count = verify(Path(sys.argv[1]) if len(sys.argv) > 1 else Path(__file__).resolve().parents[1])
        print(f"Archive verified: {count} payloads")
    except (OSError, ValueError) as error:
        sys.exit(str(error))
