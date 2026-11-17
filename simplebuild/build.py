#!/usr/bin/env python3
"""Artifact-first TRION image builder; the traditional full build is never implicit."""
import argparse
import hashlib
import json
import os
import shutil
import subprocess
import sys
import tarfile
import tempfile
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
BUILD = ROOT / "build"
CACHE = BUILD / "simplebuild" / "cache" / "manifest.json"
DOWNLOADS = ROOT / "simplebuild" / "downloads.json"


def log(message):
    print("[SimpleBuild] " + message, flush=True)


def fail(message):
    raise SystemExit("[SimpleBuild] ERROR: " + message)


def file_digest(path):
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def tree_digest(directory):
    digest = hashlib.sha256()
    for path in sorted(p for p in directory.rglob("*") if p.is_file()):
        relative = path.relative_to(directory).as_posix().encode()
        digest.update(len(relative).to_bytes(4, "big"))
        digest.update(relative)
        digest.update(bytes.fromhex(file_digest(path)))
    return digest.hexdigest()


def git_output(*args):
    git = shutil.which("git")
    if not git or not (ROOT / ".git").exists():
        return ""
    result = subprocess.run(
        [git, "-C", str(ROOT), *args], capture_output=True, text=True, check=False
    )
    return result.stdout.strip() if result.returncode == 0 else ""


def config_files(arch):
    root = BUILD / "config"
    return sorted(p for p in root.rglob("*.toml") if p.is_file()
                  and (p.relative_to(root).parts[0] == arch
                       or len(p.relative_to(root).parts) == 1))


def config_file(arch, name, board):
    root = BUILD / "config"
    candidates = []
    if board:
        candidates.append(root / arch / board / (name + ".toml"))
    candidates.extend([root / arch / (name + ".toml"), root / (name + ".toml")])
    for path in candidates:
        if path.is_file():
            return path
    available = [p.relative_to(root).as_posix() for p in config_files(arch)]
    fail("No filesystem config found for " + arch + ("/" + board if board else "")
         + "/" + name + ". Available config files: " + ", ".join(available))


def load_downloads():
    if not DOWNLOADS.is_file():
        return {}
    try:
        return json.loads(DOWNLOADS.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        fail("Cannot read simplebuild/downloads.json: " + str(error))


def download_verified(name, entry, destination):
    url = entry.get("url", "")
    expected = entry.get("sha256", "").lower()
    if not url.startswith("https://") or len(expected) != 64:
        fail(name + " download requires an HTTPS URL and a 64-character SHA-256.")
    destination.parent.mkdir(parents=True, exist_ok=True)
    temporary = destination.with_name(destination.name + ".download")
    log("Downloading missing " + name + " from the configured HTTPS source.")
    try:
        request = urllib.request.Request(url, headers={"User-Agent": "TRION-SimpleBuild/1"})
        with urllib.request.urlopen(request, timeout=120) as response, temporary.open("wb") as output:
            shutil.copyfileobj(response, output)
        actual = file_digest(temporary)
        if actual != expected:
            fail(name + " SHA-256 mismatch: expected " + expected + ", received " + actual)
        return temporary
    except Exception:
        temporary.unlink(missing_ok=True)
        raise


def acquire_installer(installer, downloads):
    if installer.is_file():
        return
    entry = downloads.get("redox_installer")
    if not entry:
        fail("Missing " + str(installer.relative_to(ROOT))
             + "; add a trusted redox_installer HTTPS URL and SHA-256 to simplebuild/downloads.json.")
    temporary = download_verified("redox_installer", entry, installer)
    temporary.replace(installer)
    installer.chmod(installer.stat().st_mode | 0o111)


def safe_extract_repo(archive, destination):
    temporary = destination.with_name(destination.name + ".partial")
    shutil.rmtree(temporary, ignore_errors=True)
    temporary.mkdir(parents=True)
    try:
        with tarfile.open(archive, "r:*") as tar:
            for member in tar.getmembers():
                target = (temporary / member.name).resolve()
                if not target.is_relative_to(temporary.resolve()):
                    fail("Repository archive contains an unsafe path: " + member.name)
                if not (member.isfile() or member.isdir()):
                    fail("Repository archive contains unsupported link/device entry: " + member.name)
            tar.extractall(temporary)
        # Accept either a repo-root archive or one containing a single repo/ directory.
        children = list(temporary.iterdir())
        if len(children) == 1 and children[0].is_dir() and children[0].name == "repo":
            nested = children[0]
            staging = destination.with_name(destination.name + ".unpack")
            shutil.rmtree(staging, ignore_errors=True)
            nested.rename(staging)
            shutil.rmtree(temporary)
            temporary = staging
        shutil.rmtree(destination, ignore_errors=True)
        temporary.replace(destination)
    except Exception:
        shutil.rmtree(temporary, ignore_errors=True)
        raise


def acquire_repo(repository, downloads):
    if repository.is_dir() and any(repository.iterdir()):
        return
    entry = downloads.get("repo_archive")
    if not entry:
        fail("Missing compiled package repository at " + str(repository.relative_to(ROOT))
             + "; add repo_archive with a trusted HTTPS URL and SHA-256 to downloads.json. "
             "SimpleBuild will not run Cookbook or compile packages.")
    archive = repository.with_name(repository.name + ".archive")
    temporary = download_verified("repo_archive", entry, archive)
    try:
        safe_extract_repo(temporary, repository)
    finally:
        temporary.unlink(missing_ok=True)


def changed_recipes(previous_head):
    changed = set()
    if previous_head:
        names = git_output("diff", "--name-only", previous_head + "..HEAD",
                           "--", "build/recipes").splitlines()
        for name in names:
            parts = name.split("/")
            if len(parts) > 3:
                changed.add(parts[3])
    for line in git_output("status", "--porcelain", "--untracked-files=all").splitlines():
        name = line[3:].strip().replace("\\", "/")
        if name.startswith("build/recipes/"):
            parts = name.split("/")
            if len(parts) > 3:
                changed.add(parts[3])
    return sorted(changed)


def rebuild_changed(recipes, arch, name, board):
    if not recipes:
        return
    make = shutil.which("make")
    if not make:
        fail("Recipe sources changed (" + ", ".join(recipes) + ") but GNU make is unavailable.")
    for recipe in recipes:
        log("Rebuilding changed recipe only: " + recipe)
        command = [make, "-C", str(BUILD), "ARCH=" + arch, "CONFIG_NAME=" + name,
                   "PODMAN_BUILD=0", "r." + recipe]
        if board:
            command.append("BOARD=" + board)
        subprocess.run(command, check=True)


def build_one(installer, config, output, arch, name, board, live):
    # Match build/mk/disk.mk: installer paths are relative to the build/ directory.
    config_arg = config.relative_to(BUILD).as_posix()
    size_result = subprocess.run(
        [str(installer), "--filesystem-size", "-c", config_arg],
        cwd=BUILD, text=True, capture_output=True, check=True
    )
    size = size_result.stdout.strip()
    if not size.isdigit() or int(size) < 1:
        fail("Installer returned an invalid filesystem size in MiB: " + repr(size))
    output.parent.mkdir(parents=True, exist_ok=True)
    partial = output.with_name(output.name + ".partial")
    partial.unlink(missing_ok=True)
    with partial.open("wb") as stream:
        stream.truncate(int(size) * 1024 * 1024)
    command = [str(installer), "--cookbook=.", "--config-name=" + name,
               "-c", config_arg]
    if live:
        bootloader = BUILD / "build" / arch / name / "bootloader-live.efi"
        command.extend(["--write-bootloader=" + str(bootloader), "--live"])
    command.append(str(partial))
    try:
        subprocess.run(command, cwd=BUILD, check=True)
        if not partial.is_file() or partial.stat().st_size == 0:
            fail("Installer produced an empty image: " + str(partial))
        partial.replace(output)
        if live:
            ipxe = BUILD / "redox.ipxe"
            if not ipxe.is_file():
                fail("Live ISO build requires build/redox.ipxe, matching build/mk/disk.mk.")
            shutil.copy2(ipxe, output.parent / "redox.ipxe")
    finally:
        partial.unlink(missing_ok=True)


def input_fingerprint(config, installer, repository, ipxe, arch, name, board):
    return {
        "config_sha256": file_digest(config),
        "installer_sha256": file_digest(installer),
        "repository_sha256": tree_digest(repository),
        "ipxe_sha256": file_digest(ipxe),
        "arch": arch,
        "config": name,
        "board": board or "",
    }


def cache_valid(state, inputs, output):
    if not state or state.get("inputs") != inputs:
        return False
    for name in ("harddrive.img", "redox-live.iso"):
        artifact = output / name
        expected = state.get("artifacts", {}).get(name, {}).get("sha256")
        if not artifact.is_file() or not artifact.stat().st_size or file_digest(artifact) != expected:
            return False
    return True


def qemu_boot(arch, image, memory):
    if arch != "x86_64":
        fail("QEMU boot testing is currently configured for x86_64 only; generated image checks are complete.")
    qemu = shutil.which("qemu-system-x86_64")
    if not qemu:
        fail("QEMU executable qemu-system-x86_64 not found.")
    bios = next((p for p in (
        Path("/usr/share/ovmf/OVMF.fd"),
        Path("/usr/share/OVMF/OVMF_CODE.fd"),
    ) if p.is_file()), None)
    pflash = next((p for p in (
        Path("/usr/share/qemu/edk2-x86_64-code.fd"),
        Path("/run/libvirt/nix-ovmf/edk2-x86_64-code.fd"),
    ) if p.is_file()), None)
    if not bios and not pflash:
        fail("UEFI firmware not found. Install OVMF (OVMF.fd or edk2-x86_64-code.fd).")
    command = [qemu, "-machine", "q35", "-smp", "4", "-m", memory]
    if bios:
        command.extend(["-bios", str(bios)])
    else:
        command.extend(["-drive", "if=pflash,format=raw,unit=0,file=" + str(pflash) + ",readonly=on"])
    command.extend([
        "-drive", "file=" + str(image) + ",format=raw,if=none,id=drv0",
        "-device", "nvme,drive=drv0", "-serial", "stdio"
    ])
    log("Launching the live image as a raw NVMe disk with UEFI, matching the traditional QEMU setup.")
    log("Close the QEMU window or terminate the process to finish the boot attempt.")
    subprocess.run(command, check=False)


def main():
    parser = argparse.ArgumentParser(description="Artifact-first TRION image builder")
    parser.add_argument("--arch", default=os.getenv("ARCH", "x86_64"))
    parser.add_argument("--config", default=os.getenv("CONFIG_NAME", "desktop"))
    parser.add_argument("--board", default=os.getenv("BOARD", ""))
    parser.add_argument("--list-configs", action="store_true")
    parser.add_argument("--check", action="store_true", help="Check prerequisites without building images.")
    parser.add_argument("--no-rebuild-changed", action="store_true")
    parser.add_argument("--qemu", action="store_true")
    parser.add_argument("--memory", default="2048")
    args = parser.parse_args()

    if args.list_configs:
        for path in config_files(args.arch):
            print(path.relative_to(BUILD / "config").as_posix())
        return

    config = config_file(args.arch, args.config, args.board)
    installer = BUILD / "fstools" / "bin" / "redox_installer"
    repository = BUILD / "repo"
    ipxe = BUILD / "redox.ipxe"
    downloads = load_downloads()

    if args.check:
        missing = []
        if not installer.is_file():
            missing.append(str(installer.relative_to(ROOT)))
        if not repository.is_dir() or not any(repository.iterdir()):
            missing.append(str(repository.relative_to(ROOT)))
        if not ipxe.is_file():
            missing.append(str(ipxe.relative_to(ROOT)))
        log("Configuration found: " + str(config.relative_to(ROOT)))
        if missing:
            fail("Prerequisites missing: " + ", ".join(missing)
                 + ". Missing installer/repository can be supplied using checksum-verified downloads.json entries.")
        log("All image-generation prerequisites are present.")
        return

    acquire_installer(installer, downloads)
    acquire_repo(repository, downloads)
    if not ipxe.is_file():
        fail("Missing build/redox.ipxe; this file is required by the existing live-image recipe.")

    head = git_output("rev-parse", "HEAD")
    state = json.loads(CACHE.read_text(encoding="utf-8")) if CACHE.is_file() else None
    previous_head = state.get("source_revision", "") if state else ""
    changed = changed_recipes(previous_head)
    if changed and args.no_rebuild_changed:
        fail("Changed recipe sources detected: " + ", ".join(changed))
    rebuild_changed(changed, args.arch, args.config, args.board)

    output = BUILD / "build" / args.arch / args.config
    if args.board:
        output = BUILD / "build" / args.arch / args.config
    inputs = input_fingerprint(config, installer, repository, ipxe, args.arch, args.config, args.board)
    log("Using existing compiled repository and host installer; full Cookbook and Podman builds are bypassed.")
    log("Backend C selected: native Redox installer. Independent assembler B remains a future replaceable backend.")
    if not cache_valid(state, inputs, output):
        build_one(installer, config, output / "harddrive.img", args.arch, args.config, args.board, False)
        build_one(installer, config, output / "redox-live.iso", args.arch, args.config, args.board, True)
    else:
        log("Input fingerprints and output SHA-256 checksums match; reusing cached images.")

    artifacts = {}
    for filename in ("harddrive.img", "redox-live.iso"):
        path = output / filename
        if not path.is_file() or not path.stat().st_size:
            fail("Output missing or empty: " + str(path))
        artifacts[filename] = {"size": path.stat().st_size, "sha256": file_digest(path)}
    (output / "SHA256SUMS").write_text(
        "".join(value["sha256"] + "  " + name + "\n" for name, value in artifacts.items()),
        encoding="utf-8"
    )
    CACHE.parent.mkdir(parents=True, exist_ok=True)
    CACHE.write_text(json.dumps({
        "source_revision": head,
        "inputs": inputs,
        "artifacts": artifacts,
        "backend": "redox_installer",
    }, indent=2) + "\n", encoding="utf-8")
    log("Image creation and SHA-256 integrity checks succeeded.")
    for name, value in artifacts.items():
        log(name + ": " + str(value["size"]) + " bytes; sha256=" + value["sha256"])
    if args.qemu:
        qemu_boot(args.arch, output / "redox-live.iso", args.memory)


if __name__ == "__main__":
    try:
        main()
    except subprocess.CalledProcessError as error:
        fail("Command failed with exit code " + str(error.returncode) + ": " + str(error.cmd))
