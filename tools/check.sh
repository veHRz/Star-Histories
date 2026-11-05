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

# Arbre vérifié : le contenu du dossier de travail (fichiers ignorés exclus), pris au départ et noté
# dans target/check-ok.txt si tout est vert. L'orchestrateur (/orchestrateur) le compare à HEAD pour
# ne pas relancer ce script sur un état déjà vérifié sur ce poste. Index temporaire copié du vrai :
# seuls les fichiers modifiés sont relus.
checked_tree=""
if index=$(git rev-parse --git-path index 2>/dev/null) && [ -f "$index" ]; then
    tmp_index="$index.check"
    cp "$index" "$tmp_index"
    if GIT_INDEX_FILE="$tmp_index" git add -A 2>/dev/null; then
        checked_tree=$(GIT_INDEX_FILE="$tmp_index" git write-tree)
    fi
    rm -f "$tmp_index"
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
for start in menu new galaxy global:hexis sector:test sector:corvette sector:spaceport sector:salle_temoin; do
    step "fumée SH_START=${start}" \
        run_game "ERROR|panicked|donn.{1,3}es charg|temps de frame" SH_START="$start" SH_EXIT_AFTER_FRAMES=150
done

# La voie d'éclairage entièrement dynamique (sans lightmaps) reste disponible pour comparer.
step "fumée sector:corvette sans lightmaps" \
    run_game "ERROR|panicked|temps de frame" SH_START=sector:corvette SH_BAKED_LIGHTING=0 SH_EXIT_AFTER_FRAMES=150

# Nouvelle partie, Échap vers le menu principal, puis nouvelle partie repartie de zéro.
step "scénario menu_return" \
    run_game "ERROR|panicked|menu_return" SH_START=new SH_SCENARIO=menu_return

# Menu de pause et options, au vrai clavier (Échap) et aux vrais boutons : ni plantage ni écran fantôme.
step "scénario pause_escape" \
    run_game "ERROR|panicked|pause_escape" SH_START=new SH_SCENARIO=pause_escape

# La carte à la vraie touche, tenue comme un joueur : elle reste ouverte, puis se referme.
step "scénario map_key" \
    run_game "ERROR|panicked|map_key" SH_START=new SH_RAID=0 SH_SCENARIO=map_key

# L'encart des touches à la vraie touche (F1 tenue) : il bascule une fois par appui.
step "scénario key_help" \
    run_game "ERROR|panicked|key_help" SH_START=sector:spaceport SH_SPAWN=hall SH_SCENARIO=key_help

# L'écran du personnage aux vraies touches (I, O, Échap sans pause derrière), puis un casier.
step "scénario character" \
    run_game "ERROR|panicked|sc.{1,3}nario character" SH_START=sector:corvette SH_SPAWN=soute SH_SCENARIO=character

# M10 : le frigo de la cuisine compte ses rations, refuse ce qui ne se mange pas, reprend une ration.
step "scénario fridge" \
    run_game "ERROR|panicked|sc.{1,3}nario fridge" SH_START=sector:corvette SH_GIVE=spare_parts:5 SH_CONTAINER=corvette_carre_frigo SH_SCENARIO=fridge

# M10 : un casier nommé (sa plaque l'affiche), sauvegardé à quai et rechargé : le nom est resté.
step "scénario locker_name" \
    run_game "ERROR|panicked|sc.{1,3}nario locker_name" SH_USER_DIR=target/locker-user SH_START=sector:spaceport SH_SCENARIO=locker_name

# Sauvegarde puis chargement, en vol et à quai : argent, date, vue et pose du vaisseau retrouvés.
for start in new sector:spaceport; do
    step "scénario save_load (${start})" \
        run_game "ERROR|panicked|save_load termin" SH_USER_DIR=target/test-user SH_START="$start" SH_SCENARIO=save_load
done

# Un objet ramassé l'est pour de bon : la tablette de la chambre, prise à quai, ne revient pas après
# une sauvegarde et un chargement (et son modèle du catalogue est posé quand il existe).
step "scénario pickup_persist" \
    run_game "ERROR|panicked|pickup_persist" SH_USER_DIR=target/pickup-user SH_START=sector:spaceport SH_SCENARIO=pickup_persist

# Parcours scripté du vaisseau du joueur, du sas au cockpit, toutes portes franchies, puis assis.
step "scénario walk_ship" \
    run_game "ERROR|panicked|sc.{1,3}nario" SH_START=sector:corvette SH_SCENARIO=walk_ship

# Vol aux commandes (assisté, croisière, découplé), puis marche dans le vaisseau en plein vol.
step "scénario fly_ship" \
    run_game "ERROR|panicked|sc.{1,3}nario" SH_START=global:hexis SH_SCENARIO=fly_ship

# M10 : en vol, la porte extérieure du sas annonce le vide et ne bouge pas (ni attente ni son).
step "scénario airlock_space" \
    run_game "ERROR|panicked|sc.{1,3}nario airlock_space" SH_START=global:hexis SH_RAID=0 SH_SCENARIO=airlock_space

# Critère de M2 : quai du hangar → sas → cockpit → départ → vol → amarrage → hangar.
step "scénario port_loop" \
    run_game "ERROR|panicked|sc.{1,3}nario .*termin" SH_START=sector:spaceport SH_SCENARIO=port_loop

# M9 : chaque PNJ de comptoir du spatioport, le service qu'il ouvre, puis la tablette.
step "scénario terminal_tour" \
    run_game "ERROR|panicked|sc.{1,3}nario .*termin" SH_START=sector:spaceport SH_RAID=0 SH_SCENARIO=terminal_tour

# Critère de M3 : Hexis → Cygnara par Portail, retour par l'hyperespace, durées vérifiées.
step "scénario voyage" \
    run_game "ERROR|panicked|sc.{1,3}nario voyage : sys_|sc.{1,3}nario .*termin" SH_START=global:hexis SH_SCENARIO=voyage SH_RAID=0
# M9 : un monde isolé (Calderis, sans Portail) par l'hyperespace, aller et retour, durées vérifiées.
step "scénario voyage_isolated" \
    run_game "ERROR|panicked|sc.{1,3}nario voyage_isolated" SH_START=global:hexis SH_SCENARIO=voyage_isolated SH_RAID=0
step "scénario contract_loop" \
    run_game "ERROR|panicked|v.{1,3}rifi|sc.{1,3}nario .*termin" SH_START=global:hexis SH_SCENARIO=contract_loop SH_RAID=0

# M10 : un contrat vers Vorrh (système sans scène avant M10), Portail, autopilote, livraison.
step "scénario deliver_vorrh" \
    run_game "ERROR|panicked|v.{1,3}rifi|sc.{1,3}nario .*termin" SH_START=global:hexis SH_SCENARIO=deliver_vorrh SH_RAID=0

# Critère de M8 : le tutoriel guidé joué en entier, de la nouvelle partie à la sauvegarde.
step "scénario tutorial" \
    run_game "ERROR|panicked|sc.{1,3}nario tutorial" SH_USER_DIR=target/tuto-user SH_START=new SH_TUTORIAL=1 SH_RAID=0 SH_SCENARIO=tutorial
# Critères de M6 : un raid gagné (puis dégâts gardés et réparés à Hexis), fui, perdu.
step "scénario raid_win" \
    run_game "ERROR|panicked|raid fini|v.{1,3}rifi|sc.{1,3}nario .*termin" SH_START=global:cygnara SH_SCENARIO=raid_win SH_RAID=1
step "scénario raid_flee" \
    run_game "ERROR|panicked|raid fini" SH_START=global:cygnara SH_SCENARIO=raid_flee SH_RAID=1
step "scénario raid_lose" \
    run_game "ERROR|panicked|raid fini" SH_START=global:cygnara SH_SCENARIO=raid_lose SH_RAID=alone
step "scénario firefight" \
    run_game "ERROR|panicked|combat . pied" SH_START=sector:corvette SH_SPAWN=soute SH_BOARDING=2 SH_SCENARIO=firefight
step "scénario firefight_lose" \
    run_game "ERROR|panicked|combat . pied" SH_START=sector:corvette SH_SPAWN=soute SH_BOARDING=3 SH_SCENARIO=firefight_lose

if [ -n "$checked_tree" ]; then
    printf 'tree=%s\ncommit=%s\ndate=%s\n' "$checked_tree" "$(git rev-parse HEAD)" \
        "$(date '+%Y-%m-%d %H:%M')" > target/check-ok.txt
fi
echo "${green}Tout est vert.${reset}"
