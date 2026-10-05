"""Integrity checks for the local archive, including corrupt or incomplete installs."""

import importlib.util
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def module(name, path):
    spec = importlib.util.spec_from_file_location(name, ROOT / path)
    result = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(result)
    return result


pack = module("m32_pack", "scripts/package-m32-archive.py")
checker = module("m32_verify", "scripts/archive/verify.py")


class ArchiveIntegrity(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name) / "package"
        self.root.mkdir()
        (self.root / "game.pck").write_bytes(b"synthetic game fixture")
        (self.root / "SHA256SUMS").write_text(pack.manifest(self.root))

    def test_valid_archive_and_corrupt_payload(self):
        self.assertEqual(checker.verify(self.root), 1)
        (self.root / "game.pck").write_bytes(b"truncated")
        with self.assertRaisesRegex(ValueError, "checksum mismatch"):
            checker.verify(self.root)

    def test_missing_and_unsealed_files(self):
        (self.root / "unexpected.cfg").write_text("unsealed settings")
        with self.assertRaisesRegex(ValueError, "unsealed"):
            checker.verify(self.root)
        (self.root / "unexpected.cfg").unlink()
        (self.root / "game.pck").unlink()
        with self.assertRaisesRegex(ValueError, "missing"):
            checker.verify(self.root)

    def test_manifest_cannot_escape_installation(self):
        outside = self.root.parent / "outside"
        outside.write_text("outside fixture")
        (self.root / "SHA256SUMS").write_text(checker.digest(outside) + "  ../outside\n")
        with self.assertRaisesRegex(ValueError, "invalid"):
            checker.verify(self.root)

    def test_zip_is_independent_of_source_timestamps(self):
        import os

        first, second = self.root.parent / "first.zip", self.root.parent / "second.zip"
        pack.deterministic_zip(self.root, first)
        os.utime(self.root / "game.pck", (1234567890, 1234567890))
        pack.deterministic_zip(self.root, second)
        self.assertEqual(pack.digest(first), pack.digest(second))


if __name__ == "__main__":
    unittest.main()
