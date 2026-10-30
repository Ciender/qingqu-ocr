"""Build a Windows ZIP from the current binary, public docs and license texts."""
from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import tempfile
import tomllib
import zipfile


def main() -> None:
    root = Path(__file__).resolve().parent.parent
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source-exe", type=Path, help="Override the release executable path")
    options = parser.parse_args()
    manifest = tomllib.loads((root / "Cargo.toml").read_text(encoding="utf-8"))
    version = manifest["package"]["version"]
    source = (options.source_exe or root / "target/release/qingqu-ocr-native.exe").resolve()
    if not source.is_file():
        raise SystemExit("Release executable is missing. Run cargo build --release --locked first.")
    metadata = json.loads(subprocess.check_output(
        ["cargo", "metadata", "--locked", "--offline", "--format-version", "1",
         "--filter-platform", "x86_64-pc-windows-msvc"], cwd=root, encoding="utf-8"))
    package_by_id = {item["id"]: item for item in metadata["packages"]}
    resolved = {node["id"] for node in metadata["resolve"]["nodes"]}
    components = []
    for package_id in sorted(resolved):
        package = package_by_id[package_id]
        if package["source"] is None:
            continue
        name, package_version = package["name"], package["version"]
        notices = root / "licenses" / f"{name}-{package_version}"
        # A normal checkout already carries notices. Refresh from the local
        # Cargo cache when the dependency version changes, without downloads.
        if not notices.is_dir():
            crate = Path(package["manifest_path"]).parent
            for file in crate.rglob("*"):
                if not file.is_file() or not file.name.upper().startswith(
                    ("LICENSE", "LICENCE", "COPYING", "NOTICE", "OFL", "UFL")
                ):
                    continue
                if file.suffix.lower() not in ("", ".txt", ".md", ".rst", ".html"):
                    continue
                destination = notices / file.relative_to(crate)
                if destination.suffix.lower() == ".md":
                    destination = destination.with_suffix(".txt")
                destination.parent.mkdir(parents=True, exist_ok=True)
                if destination.exists() and destination.read_bytes() != file.read_bytes():
                    raise SystemExit(f"Conflicting license filenames: {name}")
                destination.write_bytes(file.read_bytes())
        files = [p for p in notices.rglob("*") if p.is_file()] if notices.is_dir() else []
        if not files:
            raise SystemExit(f"Missing third-party license text: {name} {package_version}")
        components.append({"name": name, "version": package_version,
                           "license": package.get("license"),
                           "repository": package.get("repository"),
                           "notice_files": len(files)})
    components.extend([
        {"name": "Scintilla", "version": "5.6.9", "license": "LicenseRef-Scintilla",
         "repository": "https://www.scintilla.org/"},
        {"name": "Umi-OCR layout adaptation", "version": "83173efc4f4453b41223ea96d63d44534bf0505c",
         "license": "MIT", "repository": "https://github.com/hiroi-sora/Umi-OCR"},
        {"name": "Microsoft Fluent UI System Icons", "version": "1.1.345", "license": "MIT",
         "repository": "https://github.com/microsoft/fluentui-system-icons"},
        {"name": "VS Code / Catppuccin palettes", "license": "MIT"},
    ])
    (root / "licenses/components.json").write_text(
        json.dumps(components, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")

    dist = root / "dist"
    dist.mkdir(exist_ok=True)
    if dist.is_symlink() or not dist.resolve().is_relative_to(root):
        raise SystemExit("Refusing an output directory outside the project.")
    stem = f"qingqu-ocr-native-{version}-windows-x64"
    output = dist / f"{stem}.zip"
    if output.exists():
        raise SystemExit(f"Release already exists; choose a new version or move the existing ZIP: {output}")
    with tempfile.TemporaryDirectory(prefix="package-", dir=dist) as scratch:
        package_root = Path(scratch) / stem
        package_root.mkdir()
        shutil.copy2(source, package_root / "qingqu-ocr-native.exe")
        for filename in ("README.md", "README.en.md", "LICENSE", "NOTICE.txt", "THIRD_PARTY_NOTICES.txt"):
            shutil.copy2(root / filename, package_root / filename)
        shutil.copytree(root / "licenses", package_root / "licenses")
        shutil.copytree(root / "docs/screenshots", package_root / "docs/screenshots")
        (package_root / "assets").mkdir()
        shutil.copy2(root / "assets/app.svg", package_root / "assets/app.svg")
        with zipfile.ZipFile(output, "w", compression=zipfile.ZIP_DEFLATED, compresslevel=9) as archive:
            for file in sorted(package_root.rglob("*")):
                if file.is_file():
                    archive.write(file, file.relative_to(Path(scratch)))
    digest = hashlib.sha256(output.read_bytes()).hexdigest()
    output.with_suffix(".zip.sha256").write_text(f"{digest}  {output.name}\n", encoding="ascii")
    print(json.dumps({"archive": str(output), "bytes": output.stat().st_size,
                      "sha256": digest, "third_party_components": len(components)}, ensure_ascii=False))


if __name__ == "__main__":
    main()
