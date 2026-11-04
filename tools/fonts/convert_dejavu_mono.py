"""Convertit DejaVu Sans Mono fournie avec Blender (WOFF2) en TTF pour Bevy (M8, lot U).

La police à chasse fixe sert aux relevés de l'interface (vitesse, distances, munitions, argent,
numéros de choix) : des chiffres qui ne dansent pas quand la valeur change. Bevy 0.20 ne charge
que TTF et OTF.

Licence : Bitstream Vera Fonts License + Arev Fonts License, modifications DejaVu dans le domaine
public (« 2003 Bitstream, Inc. (Bitstream font glyphs). 2006 Tavmjong Bah (Arev font glyphs).
DejaVu changes are in public domain »). La redistribution est permise ; seule une police dont les
glyphes sont modifiés doit changer de nom (ni « Bitstream » ni « Vera »). Ici, simple changement
de format, glyphes intacts. Le texte des licences accompagne le fichier
(assets/fonts/LICENSE-DejaVuSansMono.txt) ; l'origine est à consigner dans assets/CREDITS.md.

Usage (depuis la racine du dépôt) :
    uv run --python 3.11 --with fonttools --with brotli tools/fonts/convert_dejavu_mono.py
"""

import os
import shutil
import sys
from pathlib import Path

from fontTools.ttLib import TTFont

OUT = Path(__file__).resolve().parents[2] / "assets" / "fonts"


def blender_dirs() -> list[Path]:
    """Dossiers d'installation de Blender 5.2 possibles (mêmes règles que convert_inter.py)."""
    dirs = []
    if "SH_BLENDER" in os.environ:
        dirs.append(Path(os.environ["SH_BLENDER"]).resolve().parent)
    if "LOCALAPPDATA" in os.environ:  # Windows : version portable (AGENTS.md §3.1)
        dirs.append(Path(os.environ["LOCALAPPDATA"]) / "Programs" / "Blender Foundation" / "Blender 5.2")
    if exe := shutil.which("blender"):
        dirs.append(Path(exe).resolve().parent)
    dirs.append(Path("/usr/share/blender"))  # Linux : paquet de la distribution
    dirs.append(Path("/usr/share/doc/blender"))  # Linux : licences du paquet (Arch, CachyOS)
    return dirs


def find(rel: str) -> Path | None:
    return next((d / rel for d in blender_dirs() if (d / rel).exists()), None)


SOURCE = find("5.2/datafiles/fonts/DejaVuSansMono.woff2")
# Licences fournies avec Blender ; absentes de certains paquets : on garde alors le fichier versionné.
LICENSES = [find("license/spdx/Bitstream-Vera.txt"), find("license/others/Arev-Fonts.txt")]


def main() -> int:
    if SOURCE is None:
        searched = ", ".join(str(d) for d in blender_dirs())
        print(f"DejaVuSansMono.woff2 introuvable dans : {searched} (SH_BLENDER=<blender> pour préciser)", file=sys.stderr)
        return 1
    OUT.mkdir(parents=True, exist_ok=True)
    font = TTFont(SOURCE)
    font.flavor = None  # WOFF2 -> TTF, glyphes inchangés
    target = OUT / "DejaVuSansMono.ttf"
    font.save(target)
    print(f"écrit {target} ({target.stat().st_size} octets)")
    if any(path is None for path in LICENSES):
        print("licences absentes de cette installation de Blender : LICENSE-DejaVuSansMono.txt inchangé")
        return 0
    header = (
        "DejaVu Sans Mono — 2003 Bitstream, Inc. (Bitstream font glyphs). 2006 Tavmjong Bah (Arev font\n"
        "glyphs). DejaVu changes are in public domain. https://github.com/dejavu-fonts/dejavu-fonts\n"
        "Fichier converti (WOFF2 -> TTF, glyphes intacts) depuis la police fournie avec Blender 5.2\n"
        "(tools/fonts/convert_dejavu_mono.py).\n\n"
    )
    body = "\n\n".join(path.read_text(encoding="utf-8") for path in LICENSES if path is not None)
    (OUT / "LICENSE-DejaVuSansMono.txt").write_text(header + body, encoding="utf-8")
    return 0


if __name__ == "__main__":
    sys.exit(main())
