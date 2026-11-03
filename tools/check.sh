#!/usr/bin/env bash
# Vérification complète du dépôt (AGENTS.md §1, règle 7) : à lancer avant chaque commit.
#   tools/check.sh
# Pendant Linux de tools/check.ps1 (Windows) : mêmes étapes, à garder synchronisés.
# Étapes : formatage, lint, tests, import du canon à jour, test de fumée de chaque SH_START.
# S'arrête à la première étape en échec (code de sortie 1).

set -u
# Journaux du jeu sans codes de couleur ANSI, lisibles et filtrables.
export NO_COLOR=1
cd "$(dirname "$0")/.." || exit 1
if ! command -v cargo >/dev/null 2>&1; then
    export PATH="$HOME/.cargo/bin:$PATH"
fi

cyan=$'\e[36m'; red=$'\e[31m'; green=$'\e[32m'; reset=$'\e[0m'

step() {
    local name=$1; shift
    echo "${cyan}== ${name}${reset}"
    "$@"
    local code=$?
    if [ "$code" -ne 0 ]; then
        echo "${red}ÉCHEC : ${name} (code ${code})${reset}"
        exit 1
    fi
}

# Lance le jeu, n'affiche que les lignes du journal qui correspondent au motif, rend le code du jeu.
run_game() {
    local pattern=$1; shift
    local log code
    log=$(env "$@" cargo dev 2>&1)
    code=$?
    grep -E "$pattern" <<<"$log" | sed 's/^/   /'
    return "$code"
}

step "cargo fmt --check" cargo fmt --all --check
step "cargo lint" cargo lint
step "cargo t" cargo t
step "import du canon à jour" cargo run -q -p sh_import -- --check

# Chaque point d'entrée de l'autopilote doit démarrer, charger les données et quitter sans panique.
for start in menu galaxy global:hexis sector:test sector:player_ship sector:hexis_spaceport; do
    step "fumée SH_START=${start}" \
        run_game "ERROR|panicked|donn.{1,3}es charg|temps de frame" SH_START="$start" SH_EXIT_AFTER_FRAMES=150
done

# La voie d'éclairage entièrement dynamique (sans lightmaps) reste disponible pour comparer.
step "fumée sector:player_ship sans lightmaps" \
    run_game "ERROR|panicked|temps de frame" SH_START=sector:player_ship SH_BAKED_LIGHTING=0 SH_EXIT_AFTER_FRAMES=150

# Parcours scripté du vaisseau du joueur, du sas au cockpit, toutes portes franchies.
step "scénario walk_ship" \
    run_game "ERROR|panicked|sc.{1,3}nario" SH_START=sector:player_ship SH_SCENARIO=walk_ship

# Vol aux commandes (assisté, croisière, découplé), puis marche dans le vaisseau en plein vol.
step "scénario fly_ship" \
    run_game "ERROR|panicked|sc.{1,3}nario" SH_START=global:hexis SH_SCENARIO=fly_ship

# Critère de M2 : quai → cockpit → départ → vol → amarrage → quai.
step "scénario port_loop" \
    run_game "ERROR|panicked|sc.{1,3}nario .*termin" SH_START=sector:hexis_spaceport SH_SCENARIO=port_loop

# Critère de M3 : Hexis → Cygnara par Portail, retour par l'hyperespace, durées vérifiées.
step "scénario voyage" \
    run_game "ERROR|panicked|sc.{1,3}nario voyage : sys_|sc.{1,3}nario .*termin" SH_START=global:hexis SH_SCENARIO=voyage
step "scénario contract_loop" \
    run_game "ERROR|panicked|v.{1,3}rifi|sc.{1,3}nario .*termin" SH_START=global:hexis SH_SCENARIO=contract_loop

echo "${green}Tout est vert.${reset}"
