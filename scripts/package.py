#!/usr/bin/env python3
"""Package rpi-hello for the v1 prebuilt extension distribution protocol.

Two subcommands, matching the two stages of a release:

  fragment  run on each build runner: archive the target library and write a
            per-target manifest fragment (``rpi-hello.<target>.json``).
  manifest  run once after all artifacts are collected: verify every fragment
            and archive, then emit the final ``rpi-extension.json``.

The manifest is uploaded *after* the archives, as the installer requires.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import pathlib
import sys
import tarfile
import zipfile

NAME = "rpi-hello"
PLUGIN_ABI = 4
TARGETS = (
    "x86_64-unknown-linux-gnu",
    "aarch64-unknown-linux-gnu",
    "x86_64-unknown-linux-musl",
    "x86_64-apple-darwin",
    "aarch64-apple-darwin",
    "x86_64-pc-windows-msvc",
)


def die(message: str) -> "NoReturn":
    print(f"error: {message}", file=sys.stderr)
    raise SystemExit(1)


def digest(path: pathlib.Path) -> str:
    with path.open("rb") as source:
        return hashlib.file_digest(source, "sha256").hexdigest()


def library_name(target: str) -> str:
    if target.endswith("windows-msvc"):
        return "rpi_hello.dll"
    if target.endswith("apple-darwin"):
        return "librpi_hello.dylib"
    return "librpi_hello.so"


def archive_suffix(target: str) -> str:
    return ".zip" if target.endswith("windows-msvc") else ".tar.gz"


def write_json(path: pathlib.Path, value: dict) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")


def package_version() -> str:
    """Read the crate version without invoking cargo."""
    for line in pathlib.Path("Cargo.toml").read_text(encoding="utf-8").splitlines():
        if line.startswith("version") and "=" in line:
            return line.split("=", 1)[1].strip().strip('"')
    die("Cargo.toml has no package version")


def sdk_version() -> str:
    """Read the rpi-plugin-sdk version pinned in Cargo.lock."""
    try:
        import tomllib
    except ModuleNotFoundError:  # pragma: no cover - Python < 3.11
        die("Python 3.11+ is required (tomllib)")
    lock = tomllib.loads(pathlib.Path("Cargo.lock").read_text(encoding="utf-8"))
    for package in lock.get("package", []):
        if package.get("name") == "rpi-plugin-sdk":
            return package["version"]
    die("Cargo.lock does not contain rpi-plugin-sdk; run `cargo build` first")


def build_archive(source: pathlib.Path, destination: pathlib.Path) -> None:
    if destination.suffix == ".zip":
        with zipfile.ZipFile(destination, "w", compression=zipfile.ZIP_DEFLATED) as archive:
            archive.write(source, source.name)
    else:
        with tarfile.open(destination, "w:gz") as archive:
            archive.add(source, arcname=source.name)


def fragment(args: argparse.Namespace) -> None:
    if args.target not in TARGETS:
        die(f"unsupported target {args.target!r}; expected one of {', '.join(TARGETS)}")

    version = package_version()
    tag = f"{NAME}-v{version}"
    library = pathlib.Path("target") / args.target / "release" / library_name(args.target)
    if not library.is_file():
        die(f"missing build artifact {library}; run `cargo build --release --target {args.target}`")

    out = pathlib.Path(args.out)
    out.mkdir(parents=True, exist_ok=True)
    archive = out / f"{NAME}-{version}-{args.target}{archive_suffix(args.target)}"
    build_archive(library, archive)

    distribution = {
        "url": f"https://github.com/{args.repository}/releases/download/{tag}/{archive.name}",
        "sha256": digest(archive),
        "size": archive.stat().st_size,
        "format": "zip" if archive.suffix == ".zip" else "tar.gz",
        "library": {"path": library.name, "sha256": digest(library)},
    }
    record = {
        "schema_version": 1,
        "name": NAME,
        "version": version,
        "repository": f"https://github.com/{args.repository}",
        "tag": tag,
        "source_commit": args.source_commit,
        "sdk": {"version": sdk_version(), "plugin_abi": PLUGIN_ABI},
        "target": args.target,
        "distribution": distribution,
    }
    write_json(out / f"{NAME}.{args.target}.json", record)
    print(f"packaged {archive} ({distribution['size']} bytes)")


def manifest(args: argparse.Namespace) -> None:
    root = pathlib.Path(args.fragments)
    records = []
    for path in sorted(root.glob(f"{NAME}.*.json")):
        record = json.loads(path.read_text(encoding="utf-8"))
        if record.get("name") != NAME:
            die(f"{path} is not a {NAME} fragment")
        records.append(record)

    seen = {record["target"] for record in records}
    if seen != set(TARGETS) or len(records) != len(TARGETS):
        missing = sorted(set(TARGETS) - seen)
        extra = sorted(seen - set(TARGETS))
        die(f"target mismatch: missing={missing} unexpected={extra}")

    first = records[0]
    for key in ("name", "version", "repository", "tag", "source_commit", "sdk"):
        if any(record[key] != first[key] for record in records):
            die(f"fragments disagree on {key}; rebuild every target from one commit")

    for record in records:
        distribution = record["distribution"]
        archive = root / distribution["url"].rsplit("/", 1)[-1]
        if not archive.is_file():
            die(f"missing archive {archive}; collect every build artifact first")
        if archive.stat().st_size != distribution["size"] or digest(archive) != distribution["sha256"]:
            die(f"archive {archive} does not match its fragment")

    output = {
        key: first[key]
        for key in ("schema_version", "name", "version", "repository", "tag", "source_commit", "sdk")
    }
    output["targets"] = {record["target"]: record["distribution"] for record in records}
    path = pathlib.Path(args.out) / "rpi-extension.json"
    write_json(path, output)
    print(f"wrote {path} for {len(output['targets'])} targets")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)

    build = commands.add_parser("fragment", help="archive one target and write its manifest fragment")
    build.add_argument("--target", required=True)
    build.add_argument("--repository", required=True, help="owner/repo on GitHub")
    build.add_argument("--source-commit", required=True)
    build.add_argument("--out", default="dist/release-assets")
    build.set_defaults(func=fragment)

    merge = commands.add_parser("manifest", help="merge fragments into rpi-extension.json")
    merge.add_argument("--fragments", default="dist/release-assets")
    merge.add_argument("--out", default="dist/manifest")
    merge.set_defaults(func=manifest)

    args = parser.parse_args()
    args.func(args)


if __name__ == "__main__":
    main()
