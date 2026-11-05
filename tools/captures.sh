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
# Frame des vues dans un secteur : l'arrivée reste voilée tant que la cellule du joueur et ses
# voisines ne sont pas prêtes (M9, `CellStates::arrival_ready`) ; à plusieurs centaines d'i/s,
# 300 frames ne suffisent pas. captures.ps1 y met 300 (UHD 770, ≈ 60 i/s).
sector_frame=900

# Vue|variables de l'autopilote (séparées par des espaces). Positions SH_SPAWN_AT : pieds du
# joueur, repère du jeu (Y-haut, −Z = avant du vaisseau), cap et tangage en degrés.
views=(
    # M9 : l'intérieur type `corvette` (le vaisseau du joueur), salle par salle.
    "cockpit|SH_START=sector:corvette SH_SPAWN=cockpit SH_SCREENSHOT_FRAME=$sector_frame"
    "cockpit-poste|SH_START=sector:corvette SH_SPAWN_AT=2.6,0,-15.0,30,-10 SH_SCREENSHOT_FRAME=$sector_frame"
    "couloir|SH_START=sector:corvette SH_SPAWN_AT=0,0,-1.0,0,0 SH_SCREENSHOT_FRAME=$sector_frame"
    "soute|SH_START=sector:corvette SH_SPAWN_AT=4.0,0,11.0,15,-8 SH_SCREENSHOT_FRAME=$sector_frame"
    "sas|SH_START=sector:corvette SH_SPAWN_AT=-2.2,0,3.0,90,0 SH_SCREENSHOT_FRAME=$sector_frame"
    "machines|SH_START=sector:corvette SH_SPAWN_AT=0,0,14.8,180,-5 SH_SCREENSHOT_FRAME=$sector_frame"
    "quartiers|SH_START=sector:corvette SH_SPAWN_AT=-2.0,0,-4.6,90,-5 SH_SCREENSHOT_FRAME=$sector_frame"
    "carre|SH_START=sector:corvette SH_SPAWN_AT=2.0,0,-5.6,-90,-5 SH_SCREENSHOT_FRAME=$sector_frame"
    "chambre|SH_START=sector:corvette SH_SPAWN=chambre SH_SCREENSHOT_FRAME=$sector_frame"
    # La chambre aux trois niveaux de décor (le préréglage de la vue remplace celui du script).
    "chambre-bas|SH_START=sector:corvette SH_SPAWN=chambre SH_PRESET=low SH_SCREENSHOT_FRAME=$sector_frame"
    "chambre-moyen|SH_START=sector:corvette SH_SPAWN=chambre SH_PRESET=medium SH_SCREENSHOT_FRAME=$sector_frame"
    "chambre-haut|SH_START=sector:corvette SH_SPAWN=chambre SH_PRESET=high SH_SCREENSHOT_FRAME=$sector_frame"
    # Le départ d'une nouvelle partie : debout dans la chambre, en orbite d'Hexis.
    "chambre-depart|SH_START=new SH_RAID=0 SH_SCREENSHOT_FRAME=$sector_frame"
    # M9 : le spatioport par défaut, le vaisseau amarré dans son berceau.
    "hangar|SH_START=sector:spaceport SH_SPAWN_AT=-2.4,0,0.4,-55,7 SH_SCREENSHOT_FRAME=$sector_frame"
    "hangar-quai|SH_START=sector:spaceport SH_SPAWN_AT=3.0,0,2.0,-7,5 SH_SCREENSHOT_FRAME=$sector_frame"
    "passerelle-sas|SH_START=sector:spaceport SH_SPAWN_AT=8.3,0,0,93,-1 SH_SCREENSHOT_FRAME=$sector_frame"
    "hall|SH_START=sector:spaceport SH_SPAWN_AT=-13.0,0,6.5,45,0 SH_SCREENSHOT_FRAME=$sector_frame"
    "bureau|SH_START=sector:spaceport SH_SPAWN_AT=-20.0,0,-7.9,0,-5 SH_SCREENSHOT_FRAME=$sector_frame"
    "pnj-comptoir|SH_START=sector:spaceport SH_SPAWN_AT=-21.0,0,-9.9,12,-10 SH_SCREENSHOT_FRAME=$sector_frame"
    "global-hexis|SH_START=global:hexis SH_RAID=0 SH_SCREENSHOT_FRAME=150"
    "global-enclume|SH_START=global:hexis SH_RAID=0 SH_ARRIVAL=l_enclume,3000 SH_SCREENSHOT_FRAME=150"
    "global-hub|SH_START=global:hexis SH_RAID=0 SH_ARRIVAL=hexis_portal_hub,1500 SH_SCREENSHOT_FRAME=150"
    "global-chantiers|SH_START=global:hexis SH_RAID=0 SH_ARRIVAL=hexis_shipyards,1500 SH_SCREENSHOT_FRAME=150"
    "raid-cygnara|SH_START=global:cygnara SH_RAID=1 SH_SCREENSHOT_FRAME=420"
    "raid-proche|SH_START=global:cygnara SH_RAID=1 SH_SCREENSHOT_FRAME=900"
    "abordage|SH_START=sector:corvette SH_SPAWN=soute SH_BOARDING=2 SH_SCREENSHOT_FRAME=$sector_frame"
    # M9 : Hexis de loin et côté nuit (textures de planète calculées en tâche de fond), la carte.
    "hexis-loin|SH_START=global:hexis SH_RAID=0 SH_LOOK=-0.631,-0.003,-0.776 SH_SCREENSHOT_FRAME=400"
    "hexis-nuit|SH_START=global:hexis SH_RAID=0 SH_ARRIVAL=hexis,-2000000 SH_LOOK=-0.222,0.208,0.953 SH_SCREENSHOT_FRAME=400"
    "carte|SH_START=galaxy SH_SCREENSHOT_FRAME=200"
    "carte-karza|SH_START=galaxy SH_ROUTE=karza SH_HOVER=karza SH_SCREENSHOT_FRAME=200"
    # M9 : écran du personnage, rangement d'un casier, bornes et PNJ du spatioport, options.
    "inventaire|SH_START=sector:corvette SH_SPAWN=soute SH_SCREEN=inventory SH_GIVE=ration:13,spare_parts:25,medkit:2,datapad,flight_suit,data_chip:3 SH_SCREENSHOT_FRAME=$screen_frame"
    "competences|SH_START=sector:corvette SH_SPAWN=soute SH_SCREEN=skills SH_XP=piloting:650,navigation:260,shooting:120,commerce:1300,engineering:40 SH_SCREENSHOT_FRAME=$screen_frame"
    "casier|SH_START=sector:corvette SH_SPAWN=soute SH_CONTAINER=corvette_chambre_casier_1 SH_GIVE=ration:4,spare_parts:6,data_chip:2 SH_SCREENSHOT_FRAME=$screen_frame"
    "borne-contrats|SH_START=sector:spaceport SH_SPAWN=hall SH_TERMINAL=contrats SH_SCREENSHOT_FRAME=$screen_frame"
    "borne-chantier|SH_START=sector:spaceport SH_SPAWN=hall SH_TERMINAL=chantier SH_SCREENSHOT_FRAME=$screen_frame"
    "pnj-parler|SH_START=sector:spaceport SH_SPAWN_AT=-20.0,0,-7.9,0,-5 SH_TALK=contracts SH_SCREENSHOT_FRAME=$screen_frame"
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
