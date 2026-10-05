#!/usr/bin/env python3
"""Build a private local archive from the current source and built Docker images."""

import argparse
import gzip
import hashlib
import json
import os
import shutil
import subprocess
import tarfile
import tempfile
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
TEMPLATE_SHA = "86409db6200b6f8fd3230989c2d2002851f3dd18acf11d7bdbafddf5a0dd0f72"
SOURCE_PATHS = (
    "VERSION", "Cargo.toml", "Cargo.lock", "Makefile", "README.md", "AGENTS.md",
    "rustfmt.toml", ".gitignore", ".dockerignore", ".env.example", ".github",
    "archive", "client", "infra", "runtime", "scripts", "tests", "tools", "web", "docs",
)


def run(args, **kwargs):
    return subprocess.run(args, check=True, cwd=ROOT, **kwargs)


def digest(path):
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def json_file(path, value):
    path.write_text(json.dumps(value, sort_keys=True, indent=2) + "\n")


def manifest(root):
    return "".join(
        f"{digest(p)}  {p.relative_to(root).as_posix()}\n"
        for p in sorted(root.rglob("*")) if p.is_file() and p != root / "SHA256SUMS"
    )


def deterministic_zip(root, output):
    with zipfile.ZipFile(output, "x", compression=zipfile.ZIP_DEFLATED, compresslevel=6) as archive:
        for path in sorted(root.rglob("*")):
            if not path.is_file():
                continue
            info = zipfile.ZipInfo(root.name + "/" + path.relative_to(root).as_posix(), (1980, 1, 1, 0, 0, 0))
            info.create_system = 3
            info.external_attr = (0o100755 if path.suffix == ".sh" or path.suffix == ".x86_64" else 0o100644) << 16
            info.compress_type = zipfile.ZIP_DEFLATED
            with path.open("rb") as source, archive.open(info, "w", force_zip64=True) as dest:
                shutil.copyfileobj(source, dest)


def source_snapshot(destination):
    names = run(["git", "ls-files", "--cached", "--others", "--exclude-standard", "-z", "--", *SOURCE_PATHS], capture_output=True).stdout.decode().split("\0")
    for name in sorted(set(names) - {""}):
        source = ROOT / name
        if source.is_symlink() or not source.is_file() or not source.resolve().is_relative_to(ROOT):
            raise ValueError("source must be a regular file: " + name)
        if source.suffix in {".pgdump", ".pem", ".key", ".log"} or source.name == ".env":
            raise ValueError("private file cannot enter the archive")
        target = destination / name
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(source, target)
        os.utime(target, (0, 0))


def source_tar(root, output):
    with output.open("xb") as raw, gzip.GzipFile(filename="", mode="wb", fileobj=raw, mtime=0) as compressed:
        with tarfile.open(fileobj=compressed, mode="w|") as archive:
            for path in sorted(root.rglob("*")):
                if not path.is_file():
                    continue
                info = archive.gettarinfo(str(path), arcname="source/" + path.relative_to(root).as_posix())
                info.uid = info.gid = info.mtime = 0
                info.uname = info.gname = ""
                info.mode = 0o755 if path.suffix == ".sh" else 0o644
                with path.open("rb") as stream:
                    archive.addfile(info, stream)


def package(output, milestone="m32"):
    if milestone not in {"m32", "m38"}:
        raise ValueError("unsupported archival checkpoint")
    output = output.resolve()
    if output.is_relative_to(ROOT):
        raise ValueError("archive output must be outside the repository")
    output.mkdir(parents=True, exist_ok=True)
    editor = Path(os.environ.get("GODOT_BIN", ROOT / ".tooling/godot/Godot_v4.7.1-stable_linux.x86_64"))
    templates = Path(os.environ.get("GODOT_TEMPLATE_ARCHIVE", ROOT / ".tooling/godot/export-templates/Godot_v4.7.1-stable_export_templates.tpz"))
    if digest(templates) != TEMPLATE_SHA:
        raise ValueError("Godot template checksum mismatch")
    if (ROOT / "VERSION").read_text().strip() != "0.2.0":
        raise ValueError("local archives preserve baseline version 0.2.0")
    # Snapshot once so source identity and exported scripts describe the same bytes.
    with tempfile.TemporaryDirectory(prefix=f"revenant-{milestone}-") as temporary:
        stage = Path(temporary)
        source = stage / "source"
        source.mkdir()
        source_snapshot(source)
        source_manifest = manifest(source)
        source_sha = hashlib.sha256(source_manifest.encode()).hexdigest()
        name = f"revenant-{milestone}-local-0.2.0-" + source_sha[:12]
        for target in (output / name, output / (name + ".zip"), output / (name + ".zip.sha256")):
            if target.exists():
                raise ValueError("archive target already exists: " + target.name)
        dest = stage / name
        for folder in ("client/windows", "client/linux", "backend", "operator", "source", "evidence"):
            (dest / folder).mkdir(parents=True)
        (dest / "evidence/SOURCE-SHA256SUMS").write_text(source_manifest)
        source_tar(source, dest / "source/source.tar.gz")
        env = dict(os.environ, XDG_DATA_HOME=str(stage / "data"), XDG_CONFIG_HOME=str(stage / "config"), XDG_CACHE_HOME=str(stage / "cache"))
        template_dir = stage / "data/godot/export_templates/4.7.1.stable"
        template_dir.mkdir(parents=True)
        with zipfile.ZipFile(templates) as archive:
            for file in ("windows_release_x86_64.exe", "linux_release.x86_64"):
                with archive.open("templates/" + file) as src, (template_dir / file).open("wb") as dst:
                    shutil.copyfileobj(src, dst)
        game = source / "client/game"
        for preset, file in (("Windows Desktop", "windows/Revenant.exe"), ("Linux", "linux/Revenant.x86_64")):
            run([str(editor), "--headless", "--path", str(game), "--export-release", preset, str(dest / "client" / file)], env=env)
        config = json.loads(run(["docker", "compose", "-f", "infra/docker-compose.yml", "config", "--format", "json"], capture_output=True).stdout)
        config.pop("name", None)
        images = {}
        for service, image in (("gateway", "infra-gateway"), ("migrate", "infra-gateway"), ("inspector", "infra-inspector"), ("postgres", config["services"]["postgres"]["image"]), ("db-provision", config["services"]["postgres"]["image"])):
            image_id = run(["docker", "image", "inspect", "--format", "{{.Id}}", image], capture_output=True).stdout.decode().strip()
            images[service] = image_id
            config["services"][service].pop("build", None)
            config["services"][service]["image"] = image_id
            config["services"][service]["pull_policy"] = "never"
        for value in config.get("volumes", {}).values():
            value.pop("name", None)
        for value in config.get("networks", {}).values():
            value.pop("name", None)
        for value in config["secrets"].values():
            value["file"] = "${REVENANT_SECRETS_DIR:?set private credentials directory}/" + Path(value["file"]).name
        for service in config["services"].values():
            for volume in service.get("volumes", []):
                if volume["type"] == "bind":
                    volume["source"] = "./" + Path(volume["source"]).name
            for port in service.get("ports", []):
                if port.get("host_ip") != "127.0.0.1":
                    raise ValueError("archive services must remain loopback-only")
                variable = {7000: "REVENANT_GAME_PORT", 8080: "REVENANT_HEALTH_PORT", 80: "INSPECTOR_PORT"}[port["target"]]
                port["published"] = "${" + variable + ":-" + str(port["published"]) + "}"
        config["services"]["gateway"]["environment"]["REVENANT_INSPECTOR_ORIGIN"] = "http://127.0.0.1:${INSPECTOR_PORT:-4173}"
        json_file(dest / "backend/compose.json", config)
        run(["docker", "save", "-o", str(dest / "backend/images.tar"), *sorted(set(images.values()))])
        for file in ("m30-provision-database.sh", "m30-generate-secrets.sh"):
            shutil.copyfile(source / "scripts" / file, dest / "backend" / file)
        for file in (source / "scripts/archive").iterdir():
            if file.is_file():
                shutil.copyfile(file, dest / "operator" / file.name)
        shutil.copyfile(source / f"docs/operations/{milestone}-local-archive.md", dest / "START-HERE.md")
        json_file(dest / "evidence/BUILD-IDENTITY.json", {
            "package": name, "kind": "private-local-archival-checkpoint", "version": "0.2.0", "milestone": milestone,
            "protocol": "v2 with frozen v1 compatibility", "public_distribution": False,
            "baseline_commit": run(["git", "rev-parse", "HEAD"], capture_output=True).stdout.decode().strip(),
            "source_state": "uncommitted working snapshot", "source_manifest_sha256": source_sha,
            "godot": run([str(editor), "--version"], capture_output=True).stdout.decode().strip(),
            "godot_editor_sha256": digest(editor), "godot_templates_sha256": TEMPLATE_SHA,
            "docker_image_ids": images,
            "reproducibility": "same payloads produce byte-identical ZIP; compiler and Docker rebuild reproducibility not claimed",
        })
        for script in dest.rglob("*.sh"):
            script.chmod(0o755)
        (dest / "client/linux/Revenant.x86_64").chmod(0o755)
        (dest / "SHA256SUMS").write_text(manifest(dest))
        run(["python3", "-B", str(dest / "operator/verify.py"), str(dest)])
        archive_path = output / (name + ".zip")
        deterministic_zip(dest, archive_path)
        repeat = stage / "repeat.zip"
        deterministic_zip(dest, repeat)
        checksum = digest(archive_path)
        if digest(repeat) != checksum:
            raise ValueError("deterministic ZIP comparison failed")
        (output / (name + ".zip.sha256")).write_text(checksum + "  " + archive_path.name + "\n")
        shutil.copytree(dest, output / name)
        # copyfile on Windows-mounted workspaces does not preserve executable modes.
        (output / name / "client/linux/Revenant.x86_64").chmod(0o755)
        print(json.dumps({"archive": str(archive_path), "sha256": checksum, "identical_zip_repack": True}))


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("output", type=Path)
    parser.add_argument("--milestone", choices=("m32", "m38"), default="m32")
    args = parser.parse_args()
    package(args.output, args.milestone)
