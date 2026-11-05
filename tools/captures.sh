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

# Frame des captures d'un écran ouvert 3 s après l'arrivée (SH_SCREEN, SH_CONTAINER,
# SH_TERMINAL, SH_TALK) : le poste Linux tourne à plusieurs centaines d'i/s, il faut dépasser
# ≈ 2 000 frames (AGENTS.md §4, captures des écrans) ; captures.ps1 y met 700 (UHD 770).
screen_frame=2400

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
    # M9 : Hexis de loin et côté nuit (textures de planète calculées en tâche de fond), la carte.
    "hexis-loin|SH_START=global:hexis SH_RAID=0 SH_LOOK=-0.631,-0.003,-0.776 SH_SCREENSHOT_FRAME=400"
    "hexis-nuit|SH_START=global:hexis SH_RAID=0 SH_ARRIVAL=hexis,-2000000 SH_LOOK=-0.222,0.208,0.953 SH_SCREENSHOT_FRAME=400"
    "carte|SH_START=galaxy SH_SCREENSHOT_FRAME=200"
    "carte-karza|SH_START=galaxy SH_ROUTE=karza SH_HOVER=karza SH_SCREENSHOT_FRAME=200"
    # M9 : écran du personnage, rangement d'un casier, bornes et PNJ du spatioport, options.
    "inventaire|SH_START=sector:player_ship SH_SPAWN=soute SH_SCREEN=inventory SH_GIVE=ration:13,spare_parts:25,medkit:2,datapad,flight_suit,data_chip:3 SH_SCREENSHOT_FRAME=$screen_frame"
    "competences|SH_START=sector:player_ship SH_SPAWN=soute SH_SCREEN=skills SH_XP=piloting:650,navigation:260,shooting:120,commerce:1300,engineering:40 SH_SCREENSHOT_FRAME=$screen_frame"
    "casier|SH_START=sector:player_ship SH_SPAWN=soute SH_CONTAINER=corvette_chambre_casier_1 SH_GIVE=ration:4,spare_parts:6,data_chip:2 SH_SCREENSHOT_FRAME=$screen_frame"
    "borne-contrats|SH_START=sector:hexis_spaceport SH_SPAWN=hall SH_TERMINAL=contrats SH_SCREENSHOT_FRAME=$screen_frame"
    "borne-chantier|SH_START=sector:hexis_spaceport SH_SPAWN=hall SH_TERMINAL=chantier SH_SCREENSHOT_FRAME=$screen_frame"
    "pnj-parler|SH_START=sector:hexis_spaceport SH_SPAWN_AT=-20.0,0,-7.9,0,-5 SH_TALK=contracts SH_SCREENSHOT_FRAME=$screen_frame"
    "options-touches|SH_START=menu SH_OPTIONS=keys:piloting SH_SCREENSHOT_FRAME=200"
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
