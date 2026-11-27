#!/usr/bin/env python3
"""Keep ship_hud/*.h identical to lib/hud_proto/*.h and lib/hud_widgets/*.h.

  python scripts/sync_headers.py          copy lib headers to ship_hud/
  python scripts/sync_headers.py --check  exit 1 if any copy differs/is missing/stale
"""
import filecmp
import shutil
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SRC_DIRS = [ROOT / "lib" / "hud_proto", ROOT / "lib" / "hud_widgets"]
DST = ROOT / "ship_hud"


def main() -> int:
    check = "--check" in sys.argv[1:]
    sources = {}
    for d in SRC_DIRS:
        for f in sorted(d.glob("*.h")):
            if f.name in sources:
                print(f"error: duplicate basename {f.name}: {sources[f.name]} and {f}")
                return 2
            sources[f.name] = f
    bad = 0
    counts = {}

    def report(kind, name):
        counts[kind] = counts.get(kind, 0) + 1
        print(f"{kind:8} {name}")

    DST.mkdir(exist_ok=True)
    for name, src in sources.items():
        dst = DST / name
        if not dst.exists():
            if check:
                report("missing", name)
                bad += 1
            else:
                shutil.copyfile(src, dst)
                report("updated", name)
        elif filecmp.cmp(src, dst, shallow=False):
            report("ok", name)
        elif check:
            report("differs", name)
            bad += 1
        else:
            shutil.copyfile(src, dst)
            report("updated", name)
    for f in sorted(DST.glob("*.h")):
        if f.name not in sources:
            if check:
                report("stale", f.name)
                bad += 1
            else:
                f.unlink()
                report("removed", f.name)
    print("summary: " + ", ".join(f"{k}={v}" for k, v in sorted(counts.items())))
    if check and bad:
        print("run: python scripts/sync_headers.py")
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
