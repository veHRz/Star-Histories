"""Convertit la police Inter fournie avec Blender (WOFF2 variable) en TTF statiques pour Bevy.

Bevy 0.20 ne charge que les formats TTF et OTF. Inter est sous licence SIL OFL 1.1
(Copyright 2020 The Inter Project Authors, https://github.com/rsms/inter) : la conversion de
format et l'instanciation de graisses sont permises ; la licence accompagne les fichiers
(assets/fonts/OFL-Inter.txt) et l'origine est consignée dans assets/CREDITS.md.

Usage (depuis la racine du dépôt) :
    uv run --python 3.11 --with fonttools --with brotli tools/fonts/convert_inter.py
"""

import os
import shutil
import sys
from pathlib import Path

from fontTools.ttLib import TTFont
from fontTools.varLib import instancer

BLENDER = Path(os.environ["LOCALAPPDATA"]) / "Programs" / "Blender Foundation" / "Blender 5.2"
SOURCE = BLENDER / "5.2" / "datafiles" / "fonts" / "Inter.woff2"
LICENSE = BLENDER / "license" / "spdx" / "OFL-1.1.txt"
OUT = Path(__file__).resolve().parents[2] / "assets" / "fonts"

# Graisses utilisées par l'interface : texte courant et titres.
WEIGHTS = {"Regular": 400, "SemiBold": 600}


def main() -> int:
    if not SOURCE.exists():
        print(f"police introuvable : {SOURCE}", file=sys.stderr)
        return 1
    OUT.mkdir(parents=True, exist_ok=True)
    for style, weight in WEIGHTS.items():
        font = TTFont(SOURCE)
        font.flavor = None  # WOFF2 -> TTF
        axes = {axis.axisTag for axis in font["fvar"].axes} if "fvar" in font else set()
        limits = {"wght": weight}
        if "opsz" in axes:
            limits["opsz"] = 14  # taille optique « texte »
        static = instancer.instantiateVariableFont(font, limits) if axes else font
        target = OUT / f"Inter-{style}.ttf"
        static.save(target)
        print(f"écrit {target} ({target.stat().st_size} octets)")
    header = (
        "Inter — Copyright 2020 The Inter Project Authors (https://github.com/rsms/inter)\n"
        "Fichiers convertis depuis la police fournie avec Blender 5.2 (tools/fonts/convert_inter.py).\n\n"
    )
    (OUT / "OFL-Inter.txt").write_text(header + LICENSE.read_text(encoding="utf-8"), encoding="utf-8")
    return 0


if __name__ == "__main__":
    sys.exit(main())
