"""Build the MSFS Community package with the PyInstaller exe inside.

Usage: python build_package.py

Expects msfs-gyro-sender.exe to already exist in dist/ (from PyInstaller).
Produces: dist/esp32-gyro-display/ ready to copy into the MSFS Community folder.
"""

import json
import os
import shutil
import time


def filetime_from_unix(unix_ts):
    """Convert Unix timestamp to Windows FILETIME (100-nanosecond intervals since 1601)."""
    return int((unix_ts + 11644473600) * 10_000_000)


def build():
    script_dir = os.path.dirname(os.path.abspath(__file__))
    exe_path = os.path.join(script_dir, "dist", "msfs-gyro-sender.exe")

    if not os.path.exists(exe_path):
        print("ERROR: dist/msfs-gyro-sender.exe not found.")
        print("Run 'pyinstaller msfs_sender.spec' first.")
        return

    # Output package directory
    pkg_dir = os.path.join(script_dir, "dist", "esp32-gyro-display")
    if os.path.exists(pkg_dir):
        shutil.rmtree(pkg_dir)
    os.makedirs(pkg_dir)

    # Copy manifest.json
    manifest_src = os.path.join(script_dir, "msfs-package", "manifest.json")
    shutil.copy2(manifest_src, os.path.join(pkg_dir, "manifest.json"))

    # Copy exe
    shutil.copy2(exe_path, os.path.join(pkg_dir, "msfs-gyro-sender.exe"))

    # Generate layout.json
    layout = {"content": []}
    for root, _dirs, files in os.walk(pkg_dir):
        for fname in files:
            if fname == "layout.json":
                continue
            fpath = os.path.join(root, fname)
            relpath = os.path.relpath(fpath, pkg_dir).replace("\\", "/")
            stat = os.stat(fpath)
            layout["content"].append({
                "path": relpath,
                "size": stat.st_size,
                "date": filetime_from_unix(stat.st_mtime),
            })

    with open(os.path.join(pkg_dir, "layout.json"), "w") as f:
        json.dump(layout, f, indent=2)

    # Update total_package_size in manifest
    total_size = sum(e["size"] for e in layout["content"])
    manifest_path = os.path.join(pkg_dir, "manifest.json")
    with open(manifest_path) as f:
        manifest = json.load(f)
    manifest["total_package_size"] = str(total_size)
    with open(manifest_path, "w") as f:
        json.dump(manifest, f, indent=2)

    print(f"Package built: {pkg_dir}")
    print(f"  Files: {len(layout['content'])}")
    print(f"  Total size: {total_size:,} bytes")
    print()
    print("To install:")
    print(f"  Copy '{os.path.basename(pkg_dir)}' folder into your MSFS Community folder")
    print("  Then run msfs-gyro-sender.exe from there, or add to exe.xml for auto-start")


if __name__ == "__main__":
    build()
