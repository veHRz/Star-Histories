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
$starts = @("menu", "galaxy", "global:hexis", "sector:test", "sector:player_ship", "sector:hexis_spaceport")
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

# Parcours scripté du vaisseau du joueur, du sas au cockpit, toutes portes franchies.
Step "scénario walk_ship" {
    $env:SH_START = "sector:player_ship"
    $env:SH_SCENARIO = "walk_ship"
    $log = & cargo dev 2>&1 | ForEach-Object { "$_" }
    $code = $LASTEXITCODE
    Remove-Item Env:SH_START, Env:SH_SCENARIO
    $log | Select-String -Pattern "ERROR|panicked|sc.{1,3}nario" | ForEach-Object { "   $_" }
    $global:LASTEXITCODE = $code
}

# Vol aux commandes (assisté, croisière, découplé), puis marche dans le vaisseau en plein vol.
Step "scénario fly_ship" {
    $env:SH_START = "global:hexis"
    $env:SH_SCENARIO = "fly_ship"
    $log = & cargo dev 2>&1 | ForEach-Object { "$_" }
    $code = $LASTEXITCODE
    Remove-Item Env:SH_START, Env:SH_SCENARIO
    $log | Select-String -Pattern "ERROR|panicked|sc.{1,3}nario" | ForEach-Object { "   $_" }
    $global:LASTEXITCODE = $code
}

# Critère de M2 : quai → cockpit → départ → vol → amarrage → quai.
Step "scénario port_loop" {
    $env:SH_START = "sector:hexis_spaceport"
    $env:SH_SCENARIO = "port_loop"
    $log = & cargo dev 2>&1 | ForEach-Object { "$_" }
    $code = $LASTEXITCODE
    Remove-Item Env:SH_START, Env:SH_SCENARIO
    $log | Select-String -Pattern "ERROR|panicked|sc.{1,3}nario .*termin" | ForEach-Object { "   $_" }
    $global:LASTEXITCODE = $code
}

# Critère de M3 : Hexis → Cygnara par Portail, retour par l'hyperespace, durées vérifiées.
Step "scénario voyage" {
    $env:SH_START = "global:hexis"
    $env:SH_SCENARIO = "voyage"
    $log = & cargo dev 2>&1 | ForEach-Object { "$_" }
    $code = $LASTEXITCODE
    Remove-Item Env:SH_START, Env:SH_SCENARIO
    $log | Select-String -Pattern "ERROR|panicked|sc.{1,3}nario voyage : sys_|sc.{1,3}nario .*termin" | ForEach-Object { "   $_" }
    $global:LASTEXITCODE = $code
}

# Critère de M5 : contrat Hexis → Cygnara livré et payé, échéance, trahison et ses conséquences.
Step "scénario contract_loop" {
    $env:SH_START = "global:hexis"
    $env:SH_SCENARIO = "contract_loop"
    $log = & cargo dev 2>&1 | ForEach-Object { "$_" }
    $code = $LASTEXITCODE
    Remove-Item Env:SH_START, Env:SH_SCENARIO
    $log | Select-String -Pattern "ERROR|panicked|v.{1,3}rifi|sc.{1,3}nario .*termin" | ForEach-Object { "   $_" }
    $global:LASTEXITCODE = $code
}

Write-Host "Tout est vert." -ForegroundColor Green
