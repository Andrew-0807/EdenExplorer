"""Packs a VS Code icon theme into one JSON file Axis can embed.

Usage: python scripts/pack-icon-theme.py <theme.json> <out.json>

The output is the theme manifest with an extra "svgs" object that maps each
iconPath to the SVG text, so the bundled pack is a single file instead of
hundreds of tiny ones.
"""

import json
import sys
from pathlib import Path


def main() -> None:
    theme_path, out_path = Path(sys.argv[1]), Path(sys.argv[2])
    theme = json.loads(theme_path.read_text(encoding="utf-8"))
    svgs = {}
    for definition in theme.get("iconDefinitions", {}).values():
        icon_path = definition.get("iconPath")
        if icon_path and icon_path.endswith(".svg") and icon_path not in svgs:
            svgs[icon_path] = (theme_path.parent / icon_path).read_text(encoding="utf-8")
    theme["svgs"] = svgs
    out_path.write_text(json.dumps(theme, separators=(",", ":")), encoding="utf-8")


if __name__ == "__main__":
    main()
