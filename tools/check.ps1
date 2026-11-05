# Vérification complète du dépôt (AGENTS.md §1, règle 7) : à lancer avant chaque commit.
#   powershell -ExecutionPolicy Bypass -File tools/check.ps1
# Étapes : formatage, lint, tests, import du canon à jour, test de fumée de chaque SH_START.
# S'arrête à la première étape en échec (code de sortie 1).

$ErrorActionPreference = "Continue"
# Journaux du jeu sans codes de couleur ANSI, lisibles et filtrables.
$env:NO_COLOR = "1"
Set-Location (Split-Path $PSScriptRoot -Parent)
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    $env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"
}

# Arbre vérifié : le contenu du dossier de travail (fichiers ignorés exclus), pris au départ et noté
# dans target/check-ok.txt si tout est vert. L'orchestrateur (/orchestrateur) le compare à HEAD pour
# ne pas relancer ce script sur un état déjà vérifié sur ce poste. Index temporaire copié du vrai :
# seuls les fichiers modifiés sont relus.
$checkedTree = $null
$index = git rev-parse --git-path index
if ($LASTEXITCODE -eq 0 -and (Test-Path $index)) {
    $tmpIndex = "$index.check"
    Copy-Item $index $tmpIndex -Force
    $env:GIT_INDEX_FILE = $tmpIndex
    git add -A 2>$null
    if ($LASTEXITCODE -eq 0) { $checkedTree = git write-tree }
    Remove-Item Env:GIT_INDEX_FILE
    Remove-Item $tmpIndex -ErrorAction SilentlyContinue
}

function Step([string]$Name, [scriptblock]$Command) {
    Write-Host "== $Name" -ForegroundColor Cyan
    & $Command
    if ($LASTEXITCODE -ne 0) {
        Write-Host "ÉCHEC : $Name (code $LASTEXITCODE)" -ForegroundColor Red
        exit 1
    }
}

Step "cargo fmt --check" { cargo fmt --all --check }
Step "cargo lint" { cargo lint }
Step "cargo t" { cargo t }
Step "import du canon à jour" { cargo run -q -p sh_import -- --check }

# Chaque point d'entrée de l'autopilote doit démarrer, charger les données et quitter sans panique.
$starts = @("menu", "new", "galaxy", "global:hexis", "sector:test", "sector:player_ship", "sector:hexis_spaceport", "sector:salle_temoin")
foreach ($start in $starts) {
    Step "fumée SH_START=$start" {
        $env:SH_START = $start
        $env:SH_EXIT_AFTER_FRAMES = "150"
        $log = & cargo dev 2>&1 | ForEach-Object { "$_" }
        $code = $LASTEXITCODE
        Remove-Item Env:SH_START, Env:SH_EXIT_AFTER_FRAMES
        $log | Select-String -Pattern "ERROR|panicked|donn.{1,3}es charg|temps de frame" | ForEach-Object { "   $_" }
        $global:LASTEXITCODE = $code
    }
}

# La voie d'éclairage entièrement dynamique (sans lightmaps) reste disponible pour comparer.
Step "fumée sector:player_ship sans lightmaps" {
    $env:SH_START = "sector:player_ship"
    $env:SH_BAKED_LIGHTING = "0"
    $env:SH_EXIT_AFTER_FRAMES = "150"
    $log = & cargo dev 2>&1 | ForEach-Object { "$_" }
    $code = $LASTEXITCODE
    Remove-Item Env:SH_START, Env:SH_BAKED_LIGHTING, Env:SH_EXIT_AFTER_FRAMES
    $log | Select-String -Pattern "ERROR|panicked|temps de frame" | ForEach-Object { "   $_" }
    $global:LASTEXITCODE = $code
}

# Nouvelle partie, Échap vers le menu principal, puis nouvelle partie repartie de zéro.
Step "scénario menu_return" {
    $env:SH_START = "new"
    $env:SH_SCENARIO = "menu_return"
    $log = & cargo dev 2>&1 | ForEach-Object { "$_" }
    $code = $LASTEXITCODE
    Remove-Item Env:SH_START, Env:SH_SCENARIO -ErrorAction SilentlyContinue
    $log | Select-String -Pattern "ERROR|panicked|menu_return" | ForEach-Object { "   $_" }
    $global:LASTEXITCODE = $code
}

# Menu de pause et options, au vrai clavier (Échap) et aux vrais boutons : ni plantage ni écran fantôme.
Step "scénario pause_escape" {
    $env:SH_START = "new"
    $env:SH_SCENARIO = "pause_escape"
    $log = & cargo dev 2>&1 | ForEach-Object { "$_" }
    $code = $LASTEXITCODE
    Remove-Item Env:SH_START, Env:SH_SCENARIO -ErrorAction SilentlyContinue
    $log | Select-String -Pattern "ERROR|panicked|pause_escape" | ForEach-Object { "   $_" }
    $global:LASTEXITCODE = $code
}

# La carte à la vraie touche, tenue comme un joueur : elle reste ouverte, puis se referme.
Step "scénario map_key" {
    $env:SH_START = "new"
    $env:SH_RAID = "0"
    $env:SH_SCENARIO = "map_key"
    $log = & cargo dev 2>&1 | ForEach-Object { "$_" }
    $code = $LASTEXITCODE
    Remove-Item Env:SH_START, Env:SH_RAID, Env:SH_SCENARIO -ErrorAction SilentlyContinue
    $log | Select-String -Pattern "ERROR|panicked|map_key" | ForEach-Object { "   $_" }
    $global:LASTEXITCODE = $code
}

# L'encart des touches à la vraie touche (F1 tenue) : il bascule une fois par appui.
Step "scénario key_help" {
    $env:SH_START = "sector:hexis_spaceport"
    $env:SH_SPAWN = "hall"
    $env:SH_SCENARIO = "key_help"
    $log = & cargo dev 2>&1 | ForEach-Object { "$_" }
    $code = $LASTEXITCODE
    Remove-Item Env:SH_START, Env:SH_SPAWN, Env:SH_SCENARIO -ErrorAction SilentlyContinue
    $log | Select-String -Pattern "ERROR|panicked|key_help" | ForEach-Object { "   $_" }
    $global:LASTEXITCODE = $code
}

# L'écran du personnage aux vraies touches (I, O, Échap sans pause derrière), puis un casier.
Step "scénario character" {
    $env:SH_START = "sector:player_ship"
    $env:SH_SPAWN = "soute"
    $env:SH_SCENARIO = "character"
    $log = & cargo dev 2>&1 | ForEach-Object { "$_" }
    $code = $LASTEXITCODE
    Remove-Item Env:SH_START, Env:SH_SPAWN, Env:SH_SCENARIO -ErrorAction SilentlyContinue
    $log | Select-String -Pattern "ERROR|panicked|sc.{1,3}nario character" | ForEach-Object { "   $_" }
    $global:LASTEXITCODE = $code
}

# Sauvegarde puis chargement, en vol et à quai : argent, date, vue et pose du vaisseau retrouvés.
foreach ($start in @("new", "sector:hexis_spaceport")) {
    Step "scénario save_load ($start)" {
        $env:SH_USER_DIR = "target/test-user"
        $env:SH_START = $start
        $env:SH_SCENARIO = "save_load"
        $log = & cargo dev 2>&1 | ForEach-Object { "$_" }
        $code = $LASTEXITCODE
        Remove-Item Env:SH_USER_DIR, Env:SH_START, Env:SH_SCENARIO -ErrorAction SilentlyContinue
        $log | Select-String -Pattern "ERROR|panicked|save_load termin" | ForEach-Object { "   $_" }
        $global:LASTEXITCODE = $code
    }
}

# Un objet ramassé l'est pour de bon : la tablette des quartiers, prise à quai, ne revient pas après
# une sauvegarde et un chargement (et son modèle du catalogue est posé quand il existe).
Step "scénario pickup_persist" {
    $env:SH_USER_DIR = "target/pickup-user"
    $env:SH_START = "sector:hexis_spaceport"
    $env:SH_SCENARIO = "pickup_persist"
    $log = & cargo dev 2>&1 | ForEach-Object { "$_" }
    $code = $LASTEXITCODE
    Remove-Item Env:SH_USER_DIR, Env:SH_START, Env:SH_SCENARIO -ErrorAction SilentlyContinue
    $log | Select-String -Pattern "ERROR|panicked|pickup_persist" | ForEach-Object { "   $_" }
    $global:LASTEXITCODE = $code
}

# Parcours scripté du vaisseau du joueur, du sas au cockpit, toutes portes franchies.
Step "scénario walk_ship" {
    $env:SH_START = "sector:player_ship"
    $env:SH_SCENARIO = "walk_ship"
    $log = & cargo dev 2>&1 | ForEach-Object { "$_" }
    $code = $LASTEXITCODE
    Remove-Item Env:SH_START, Env:SH_SCENARIO, Env:SH_RAID -ErrorAction SilentlyContinue
    $log | Select-String -Pattern "ERROR|panicked|sc.{1,3}nario" | ForEach-Object { "   $_" }
    $global:LASTEXITCODE = $code
}

# Vol aux commandes (assisté, croisière, découplé), puis marche dans le vaisseau en plein vol.
Step "scénario fly_ship" {
    $env:SH_START = "global:hexis"
    $env:SH_SCENARIO = "fly_ship"
    $log = & cargo dev 2>&1 | ForEach-Object { "$_" }
    $code = $LASTEXITCODE
    Remove-Item Env:SH_START, Env:SH_SCENARIO, Env:SH_RAID -ErrorAction SilentlyContinue
    $log | Select-String -Pattern "ERROR|panicked|sc.{1,3}nario" | ForEach-Object { "   $_" }
    $global:LASTEXITCODE = $code
}

# Critère de M2 : quai → cockpit → départ → vol → amarrage → quai.
Step "scénario port_loop" {
    $env:SH_START = "sector:hexis_spaceport"
    $env:SH_SCENARIO = "port_loop"
    $log = & cargo dev 2>&1 | ForEach-Object { "$_" }
    $code = $LASTEXITCODE
    Remove-Item Env:SH_START, Env:SH_SCENARIO, Env:SH_RAID -ErrorAction SilentlyContinue
    $log | Select-String -Pattern "ERROR|panicked|sc.{1,3}nario .*termin" | ForEach-Object { "   $_" }
    $global:LASTEXITCODE = $code
}

# Critère de M3 : Hexis → Cygnara par Portail, retour par l'hyperespace, durées vérifiées.
Step "scénario voyage" {
    $env:SH_START = "global:hexis"
    $env:SH_SCENARIO = "voyage"
    $env:SH_RAID = "0"
    $log = & cargo dev 2>&1 | ForEach-Object { "$_" }
    $code = $LASTEXITCODE
    Remove-Item Env:SH_START, Env:SH_SCENARIO, Env:SH_RAID -ErrorAction SilentlyContinue
    $log | Select-String -Pattern "ERROR|panicked|sc.{1,3}nario voyage : sys_|sc.{1,3}nario .*termin" | ForEach-Object { "   $_" }
    $global:LASTEXITCODE = $code
}

# M9 : un monde isolé (Calderis, sans Portail) par l'hyperespace, aller et retour, durées vérifiées.
Step "scénario voyage_isolated" {
    $env:SH_START = "global:hexis"
    $env:SH_SCENARIO = "voyage_isolated"
    $env:SH_RAID = "0"
    $log = & cargo dev 2>&1 | ForEach-Object { "$_" }
    $code = $LASTEXITCODE
    Remove-Item Env:SH_START, Env:SH_SCENARIO, Env:SH_RAID -ErrorAction SilentlyContinue
    $log | Select-String -Pattern "ERROR|panicked|sc.{1,3}nario voyage_isolated" | ForEach-Object { "   $_" }
    $global:LASTEXITCODE = $code
}

# Critère de M5 : contrat Hexis → Cygnara livré et payé, échéance, trahison et ses conséquences.
Step "scénario contract_loop" {
    $env:SH_START = "global:hexis"
    $env:SH_SCENARIO = "contract_loop"
    $env:SH_RAID = "0"
    $log = & cargo dev 2>&1 | ForEach-Object { "$_" }
    $code = $LASTEXITCODE
    Remove-Item Env:SH_START, Env:SH_SCENARIO, Env:SH_RAID -ErrorAction SilentlyContinue
    $log | Select-String -Pattern "ERROR|panicked|v.{1,3}rifi|sc.{1,3}nario .*termin" | ForEach-Object { "   $_" }
    $global:LASTEXITCODE = $code
}

# Critère de M8 : le tutoriel guidé joué en entier, de la nouvelle partie à la sauvegarde.
Step "scénario tutorial" {
    $env:SH_USER_DIR = "target/tuto-user"
    $env:SH_START = "new"
    $env:SH_TUTORIAL = "1"
    $env:SH_RAID = "0"
    $env:SH_SCENARIO = "tutorial"
    $log = & cargo dev 2>&1 | ForEach-Object { "$_" }
    $code = $LASTEXITCODE
    Remove-Item Env:SH_USER_DIR, Env:SH_START, Env:SH_TUTORIAL, Env:SH_RAID, Env:SH_SCENARIO -ErrorAction SilentlyContinue
    $log | Select-String -Pattern "ERROR|panicked|sc.{1,3}nario tutorial" | ForEach-Object { "   $_" }
    $global:LASTEXITCODE = $code
}

# Critères de M6 : un raid gagné (puis dégâts gardés et réparés à Hexis), fui, perdu.
Step "scénario raid_win" {
    $env:SH_START = "global:cygnara"
    $env:SH_SCENARIO = "raid_win"
    $env:SH_RAID = "1"
    $log = & cargo dev 2>&1 | ForEach-Object { "$_" }
    $code = $LASTEXITCODE
    Remove-Item Env:SH_START, Env:SH_SCENARIO, Env:SH_RAID -ErrorAction SilentlyContinue
    $log | Select-String -Pattern "ERROR|panicked|raid fini|v.{1,3}rifi|sc.{1,3}nario .*termin" | ForEach-Object { "   $_" }
    $global:LASTEXITCODE = $code
}

Step "scénario raid_flee" {
    $env:SH_START = "global:cygnara"
    $env:SH_SCENARIO = "raid_flee"
    $env:SH_RAID = "1"
    $log = & cargo dev 2>&1 | ForEach-Object { "$_" }
    $code = $LASTEXITCODE
    Remove-Item Env:SH_START, Env:SH_SCENARIO, Env:SH_RAID -ErrorAction SilentlyContinue
    $log | Select-String -Pattern "ERROR|panicked|raid fini" | ForEach-Object { "   $_" }
    $global:LASTEXITCODE = $code
}

Step "scénario raid_lose" {
    $env:SH_START = "global:cygnara"
    $env:SH_SCENARIO = "raid_lose"
    $env:SH_RAID = "alone"
    $log = & cargo dev 2>&1 | ForEach-Object { "$_" }
    $code = $LASTEXITCODE
    Remove-Item Env:SH_START, Env:SH_SCENARIO, Env:SH_RAID -ErrorAction SilentlyContinue
    $log | Select-String -Pattern "ERROR|panicked|raid fini" | ForEach-Object { "   $_" }
    $global:LASTEXITCODE = $code
}

# Critère de M6 à pied : un abordage repoussé, un abordage perdu.
Step "scénario firefight" {
    $env:SH_START = "sector:player_ship"
    $env:SH_SPAWN = "soute"
    $env:SH_BOARDING = "2"
    $env:SH_SCENARIO = "firefight"
    $log = & cargo dev 2>&1 | ForEach-Object { "$_" }
    $code = $LASTEXITCODE
    Remove-Item Env:SH_START, Env:SH_SPAWN, Env:SH_BOARDING, Env:SH_SCENARIO -ErrorAction SilentlyContinue
    $log | Select-String -Pattern "ERROR|panicked|combat . pied" | ForEach-Object { "   $_" }
    $global:LASTEXITCODE = $code
}

Step "scénario firefight_lose" {
    $env:SH_START = "sector:player_ship"
    $env:SH_SPAWN = "soute"
    $env:SH_BOARDING = "3"
    $env:SH_SCENARIO = "firefight_lose"
    $log = & cargo dev 2>&1 | ForEach-Object { "$_" }
    $code = $LASTEXITCODE
    Remove-Item Env:SH_START, Env:SH_SPAWN, Env:SH_BOARDING, Env:SH_SCENARIO -ErrorAction SilentlyContinue
    $log | Select-String -Pattern "ERROR|panicked|combat . pied" | ForEach-Object { "   $_" }
    $global:LASTEXITCODE = $code
}

if ($checkedTree) {
    @("tree=$checkedTree", "commit=$(git rev-parse HEAD)", "date=$(Get-Date -Format 'yyyy-MM-dd HH:mm')") |
        Set-Content -Encoding ascii target/check-ok.txt
}
Write-Host "Tout est vert." -ForegroundColor Green
