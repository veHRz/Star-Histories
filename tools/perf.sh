#!/usr/bin/env bash
# Mesures de performance (AGENTS.md §10) : les scénarios de l'autopilote en release, au préréglage
# et à la taille de fenêtre choisis. Seules les mesures du poste Windows (UHD 770) valent pour les
# budgets ; ici, elles servent à comparer deux versions entre elles.
#   tools/perf.sh [préréglage=medium] [fenêtre=fullscreen] [scénarios séparés par des virgules]
# Fenêtre : `fullscreen` (plein écran sans bordure) ou `<L>x<H>`.
# Résumé dans target/perf/<date>-<préréglage>-<fenêtre>.txt. Pendant de tools/perf.ps1 : modifier les deux ensemble.

set -u
export NO_COLOR=1
cd "$(dirname "$0")/.." || exit 1
if ! command -v cargo >/dev/null 2>&1; then
    export PATH="$HOME/.cargo/bin:$PATH"
fi

preset=${1:-medium}
window=${2:-fullscreen}
only=${3:-}

cyan=$'\e[36m'; red=$'\e[31m'; green=$'\e[32m'; reset=$'\e[0m'

# Chaque critère de performance des jalons, avec les variables de son scénario.
runs=(
    "walk_ship|SH_START=sector:player_ship SH_SCENARIO=walk_ship"
    "walk_ship_sans_lightmaps|SH_START=sector:player_ship SH_SCENARIO=walk_ship SH_BAKED_LIGHTING=0"
    "fly_ship|SH_START=global:hexis SH_SCENARIO=fly_ship"
    "port_loop|SH_START=sector:hexis_spaceport SH_SCENARIO=port_loop"
    "raid_win|SH_START=global:cygnara SH_SCENARIO=raid_win SH_RAID=1"
    "firefight|SH_START=sector:player_ship SH_SPAWN=soute SH_BOARDING=2 SH_SCENARIO=firefight"
    "firefight_lose|SH_START=sector:player_ship SH_SPAWN=soute SH_BOARDING=3 SH_SCENARIO=firefight_lose"
)

echo "${cyan}== cargo build --release${reset}"
if ! cargo build --release; then
    echo "${red}ÉCHEC : cargo build --release${reset}"
    exit 1
fi

gpu=$(lspci 2>/dev/null | grep -E "VGA|Display|3D" | sed 's/^[^:]*: //' | paste -sd ',' -)
report="Mesures du $(date '+%d/%m/%Y %H:%M') : $(hostname), ${gpu:-GPU inconnu}
Commit $(git log -1 --format='%h %s') ; release, préréglage ${preset}, fenêtre ${window}
"
failed=0
for run in "${runs[@]}"; do
    name=${run%%|*}
    vars=${run#*|}
    if [ -n "$only" ] && [[ ",${only}," != *",${name},"* ]]; then
        continue
    fi
    echo "${cyan}== ${name}${reset}"
    # shellcheck disable=SC2086 # les variables du scénario sont séparées par des espaces, voulu.
    log=$(env $vars SH_PRESET="$preset" SH_WINDOW="$window" cargo run --release -q 2>&1)
    code=$?
    lines=$(grep -E "ERROR|panicked|sc.{1,3}nario .*(termin|raid fini|combat . pied)|temps de frame :" <<<"$log" \
        | sed -E 's/^.*(sh_game|star_histories)[^ ]*: //')
    if [ "$code" -ne 0 ]; then
        failed=1
        lines="${lines}
ÉCHEC (code ${code})"
    fi
    sed 's/^/   /' <<<"$lines"
    report="${report}
## ${name}
${lines}
"
done

mkdir -p target/perf
file="target/perf/$(date '+%Y-%m-%d-%H%M')-${preset}-${window}.txt"
printf '%s\n' "$report" >"$file"
echo "${green}Résumé : ${file}${reset}"
exit "$failed"
