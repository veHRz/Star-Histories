#!/usr/bin/env bash
# Captures de référence à points de vue fixes (AGENTS.md §3.3, M8 : comparaison avant / après).
#   tools/captures.sh [préfixe] [préréglage] [fenêtre] [vue,vue…]
#   ex. tools/captures.sh m8-avant medium 1600x900
# Écrit target/shots/<préfixe>-<vue>.png pour chaque point de vue. Même liste dans
# captures.ps1 : modifie les deux scripts ensemble.

set -u
cd "$(dirname "$0")/.."
export NO_COLOR=1
prefix="${1:-m8}"
preset="${2:-medium}"
window="${3:-1600x900}"
only="${4:-}"
mkdir -p target/shots

# Vue|variables de l'autopilote (séparées par des espaces). Positions SH_SPAWN_AT : pieds du
# joueur, repère du jeu (Y-haut, −Z = avant du vaisseau), cap et tangage en degrés.
views=(
    "cockpit|SH_START=sector:player_ship SH_SPAWN=cockpit SH_SCREENSHOT_FRAME=300"
    "cockpit-poste|SH_START=sector:player_ship SH_SPAWN_AT=2.6,0,-15.0,30,-10 SH_SCREENSHOT_FRAME=300"
    "couloir|SH_START=sector:player_ship SH_SPAWN_AT=0,0,-1.0,0,0 SH_SCREENSHOT_FRAME=300"
    "soute|SH_START=sector:player_ship SH_SPAWN_AT=9.4,0,11.6,35,-8 SH_SCREENSHOT_FRAME=300"
    "sas|SH_START=sector:player_ship SH_SPAWN_AT=-2.0,0,3.0,90,0 SH_SCREENSHOT_FRAME=300"
    "machines|SH_START=sector:player_ship SH_SPAWN_AT=0,0,14.8,180,-5 SH_SCREENSHOT_FRAME=300"
    "hall|SH_START=sector:hexis_spaceport SH_SPAWN_AT=-13.0,0,6.5,45,0 SH_SCREENSHOT_FRAME=300"
    "bureau|SH_START=sector:hexis_spaceport SH_SPAWN_AT=-20.0,0,-7.9,0,-5 SH_SCREENSHOT_FRAME=300"
    "global-hexis|SH_START=global:hexis SH_RAID=0 SH_SCREENSHOT_FRAME=150"
    "global-enclume|SH_START=global:hexis SH_RAID=0 SH_ARRIVAL=l_enclume,3000 SH_SCREENSHOT_FRAME=150"
    "global-hub|SH_START=global:hexis SH_RAID=0 SH_ARRIVAL=hexis_portal_hub,1500 SH_SCREENSHOT_FRAME=150"
    "global-chantiers|SH_START=global:hexis SH_RAID=0 SH_ARRIVAL=hexis_shipyards,1500 SH_SCREENSHOT_FRAME=150"
    "raid-cygnara|SH_START=global:cygnara SH_RAID=1 SH_SCREENSHOT_FRAME=420"
    "raid-proche|SH_START=global:cygnara SH_RAID=1 SH_SCREENSHOT_FRAME=900"
    "abordage|SH_START=sector:player_ship SH_SPAWN=soute SH_BOARDING=2 SH_SCREENSHOT_FRAME=300"
)

failed=()
for entry in "${views[@]}"; do
    name="${entry%%|*}"
    vars="${entry#*|}"
    if [[ -n "$only" && ",$only," != *",$name,"* ]]; then
        continue
    fi
    shot="target/shots/$prefix-$name.png"
    echo "== $shot"
    # shellcheck disable=SC2086
    env SH_SCREENSHOT="$shot" SH_PRESET="$preset" SH_WINDOW="$window" $vars cargo dev 2>&1 \
        | grep -E "ERROR|panicked" | sed 's/^/   /'
    code=${PIPESTATUS[0]}
    if [[ $code -ne 0 || ! -f "$shot" ]]; then
        echo "ÉCHEC : $name (code $code)"
        failed+=("$name")
    fi
done
if [[ ${#failed[@]} -gt 0 ]]; then
    echo "Captures manquées : ${failed[*]}"
    exit 1
fi
echo "Captures écrites dans target/shots/$prefix-*.png"
