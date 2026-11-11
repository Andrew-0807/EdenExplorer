"""Builds src/assets/icon.ico from src/assets/icons/axis/icon-app-axis-tile.svg.

Renders each size with Inkscape (must be on PATH) and packs the PNGs into one
.ico (PNG-compressed entries, supported since Windows Vista).

Usage: python scripts/make-app-icon.py
"""

import pathlib
import struct
import subprocess
import tempfile

ROOT = pathlib.Path(__file__).resolve().parent.parent
SVG = ROOT / "src/assets/icons/axis/icon-app-axis-tile.svg"
OUT = ROOT / "src/assets/icon.ico"
SIZES = [16, 24, 32, 48, 64, 128, 256]


def render(size: int, dest: pathlib.Path) -> bytes:
    subprocess.run(
        ["inkscape", str(SVG), "--export-type=png",
         f"--export-filename={dest}", "-w", str(size), "-h", str(size)],
        check=True, capture_output=True,
    )
    return dest.read_bytes()


def main() -> None:
    with tempfile.TemporaryDirectory() as tmp:
        images = [render(s, pathlib.Path(tmp) / f"{s}.png") for s in SIZES]

    header = struct.pack("<HHH", 0, 1, len(SIZES))
    offset = len(header) + 16 * len(SIZES)
    entries, data = b"", b""
    for size, png in zip(SIZES, images):
        dim = 0 if size == 256 else size  # 0 means 256 in the ICO format
        entries += struct.pack("<BBBBHHII", dim, dim, 0, 0, 1, 32, len(png), offset + len(data))
        data += png
    OUT.write_bytes(header + entries + data)
    print(f"wrote {OUT} ({len(SIZES)} sizes)")


if __name__ == "__main__":
    main()
