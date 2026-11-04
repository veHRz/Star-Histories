#!/usr/bin/env bash
# Installation d'un poste Linux (Arch / CachyOS) pour développer Star Histories.
#   tools/setup-linux.sh
# Rejouable : chaque étape vérifie d'abord ce qui est déjà en place. Demande le mot de passe sudo
# seulement s'il manque des paquets système. Ne touche pas à la configuration Git globale.
# Sous Windows, voir docs/reprise-autre-pc.md (prompt 1).

set -euo pipefail
cd "$(dirname "$0")/.."

ADDON_URL="https://raw.githubusercontent.com/ahujasid/mcp-for-blender/992233bb258975a05eec8b5691465c50c2831e34/addon.py"
ADDON_SHA256="6c2917c157c26d7549adcbd480243db2154a9b06bc580664f090530c8ef3801f"
ADDON_DIR="$HOME/.config/blender/5.2/scripts/addons"
BEVY_TAG="v0.20.1"

# git-lfs et uv pour le dépôt et le MCP ; le reste pour compiler et lancer Bevy (son, manettes,
# fenêtre X11/Wayland, Vulkan). Le pilote Vulkan de la carte graphique (vulkan-radeon, vulkan-intel,
# nvidia-utils) est à installer selon la machine.
PACKAGES=(git-lfs uv blender rustup clang pkgconf alsa-lib systemd-libs libxkbcommon wayland
    libx11 libxcursor libxrandr libxi vulkan-icd-loader)

say() { echo $'\e[36m'"== $*"$'\e[0m'; }

say "Paquets système"
missing=()
for p in "${PACKAGES[@]}"; do pacman -Q "$p" >/dev/null 2>&1 || missing+=("$p"); done
if [ ${#missing[@]} -gt 0 ]; then
    sudo pacman -S --needed "${missing[@]}"
else
    echo "   déjà installés"
fi
if ! ls /usr/share/vulkan/icd.d/*.json >/dev/null 2>&1; then
    echo "   ATTENTION : aucun pilote Vulkan trouvé (/usr/share/vulkan/icd.d) : le jeu ne démarrera pas"
fi

say "Rust (version épinglée par rust-toolchain.toml)"
rustup toolchain install   # lit rust-toolchain.toml : version et composants rustfmt, clippy
rustc --version

say "Git LFS (configuration locale au dépôt uniquement)"
git lfs install --local
git lfs pull
small=$(find assets/fonts assets/models -type f \( -name '*.ttf' -o -name '*.glb' -o -name '*.bin' \) -size -1k)
if [ -n "$small" ]; then
    echo "   ÉCHEC : fichiers encore à l'état de pointeurs LFS :"; echo "$small"; exit 1
fi
echo "   polices et modèles présents"

say "Sources de référence de Bevy ($BEVY_TAG)"
if [ -d .reference/bevy-0.20.1/.git ]; then
    echo "   déjà présentes ($(git -C .reference/bevy-0.20.1 describe --tags))"
else
    git clone --depth 1 --branch "$BEVY_TAG" https://github.com/bevyengine/bevy.git .reference/bevy-0.20.1
fi

say "Blender"
blender --version | head -1
blender --version | grep -q "Blender 5.2" || echo "   ATTENTION : le projet utilise Blender 5.2.3 LTS"

say "Addon MCP for Blender"
target="$ADDON_DIR/blender_mcp.py"
if [ -f "$target" ] && echo "$ADDON_SHA256  $target" | sha256sum -c --quiet 2>/dev/null; then
    echo "   déjà installé"
else
    tmp=$(mktemp)
    curl -fsSL -o "$tmp" "$ADDON_URL"
    echo "$ADDON_SHA256  $tmp" | sha256sum -c --quiet
    mkdir -p "$ADDON_DIR"
    mv "$tmp" "$target"
fi
blender --background --python-expr \
    "import bpy, addon_utils; addon_utils.enable('blender_mcp', default_set=True, persistent=True); bpy.ops.wm.save_userpref(); print('ADDON', addon_utils.check('blender_mcp'))" \
    2>&1 | grep ADDON

say "uvx (lance le serveur MCP déclaré dans .mcp.json)"
uvx --version

echo $'\e[32m'"Poste prêt. Première compilation : cargo build --features dev, puis tools/check.sh."$'\e[0m'
