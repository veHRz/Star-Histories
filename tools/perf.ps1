# Mesures de performance (AGENTS.md §10) : les scénarios de l'autopilote en release, au préréglage
# et à la taille de fenêtre choisis. Seules les mesures du poste Windows (UHD 770) valent pour les
# budgets ; ailleurs, elles servent à comparer deux versions entre elles.
#   powershell -ExecutionPolicy Bypass -File tools/perf.ps1 [-Preset medium] [-Window fullscreen] [-Only raid_win,firefight]
# -Window : `fullscreen` (plein écran sans bordure, 1080p sur la machine de référence) ou `<L>x<H>`.
# Résumé dans target/perf/<date>-<préréglage>-<fenêtre>.txt. Pendant de tools/perf.sh : modifier les deux ensemble.

param(
    [string]$Preset = "medium",
    [string]$Window = "fullscreen",
    [string[]]$Only = @()
)

$ErrorActionPreference = "Continue"
$env:NO_COLOR = "1"
# Les journaux du jeu sont en UTF-8 : sans cela, PowerShell 5.1 les lit dans la page de code ANSI.
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8
Set-Location (Split-Path $PSScriptRoot -Parent)
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    $env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"
}

# Chaque critère de performance des jalons, avec les variables de son scénario.
$runs = @(
    @{ Name = "walk_ship"; Env = @{ SH_START = "sector:player_ship"; SH_SCENARIO = "walk_ship" } },
    @{ Name = "walk_ship_sans_lightmaps"; Env = @{ SH_START = "sector:player_ship"; SH_SCENARIO = "walk_ship"; SH_BAKED_LIGHTING = "0" } },
    @{ Name = "fly_ship"; Env = @{ SH_START = "global:hexis"; SH_SCENARIO = "fly_ship" } },
    @{ Name = "port_loop"; Env = @{ SH_START = "sector:hexis_spaceport"; SH_SCENARIO = "port_loop" } },
    @{ Name = "raid_win"; Env = @{ SH_START = "global:cygnara"; SH_SCENARIO = "raid_win"; SH_RAID = "1" } },
    @{ Name = "firefight"; Env = @{ SH_START = "sector:player_ship"; SH_SPAWN = "soute"; SH_BOARDING = "2"; SH_SCENARIO = "firefight" } },
    @{ Name = "firefight_lose"; Env = @{ SH_START = "sector:player_ship"; SH_SPAWN = "soute"; SH_BOARDING = "3"; SH_SCENARIO = "firefight_lose" } }
)
#   (`powershell -File` passe `-Only a,b` comme une seule chaîne.)
$Only = $Only -split "," | ForEach-Object { $_.Trim() } | Where-Object { $_ }
if ($Only.Count -gt 0) {
    $runs = $runs | Where-Object { $Only -contains $_.Name }
}

Write-Host "== cargo build --release" -ForegroundColor Cyan
cargo build --release
if ($LASTEXITCODE -ne 0) {
    Write-Host "ÉCHEC : cargo build --release" -ForegroundColor Red
    exit 1
}

$gpu = (Get-CimInstance Win32_VideoController | ForEach-Object { "$($_.Name) (pilote $($_.DriverVersion))" }) -join ", "
$commit = git log -1 --format="%h %s"
$report = @(
    "Mesures du $(Get-Date -Format 'dd/MM/yyyy HH:mm') : $env:COMPUTERNAME, $gpu",
    "Commit $commit ; release, préréglage $Preset, fenêtre $Window",
    ""
)
$failed = $false
foreach ($run in $runs) {
    Write-Host "== $($run.Name)" -ForegroundColor Cyan
    $vars = $run.Env + @{ SH_PRESET = $Preset; SH_WINDOW = $Window }
    foreach ($key in $vars.Keys) { Set-Item "Env:$key" $vars[$key] }
    $log = & cargo run --release -q 2>&1 | ForEach-Object { "$_" }
    $code = $LASTEXITCODE
    foreach ($key in $vars.Keys) { Remove-Item "Env:$key" -ErrorAction SilentlyContinue }
    $lines = $log | Select-String -Pattern "ERROR|panicked|sc.{1,3}nario .*(termin|raid fini|combat . pied)|temps de frame :" |
        ForEach-Object { ($_.Line -replace '^.*?(sh_game|star_histories)\S*: ', '').Trim() }
    if ($code -ne 0) {
        $failed = $true
        $lines += "ÉCHEC (code $code)"
    }
    $report += "## $($run.Name)"
    $report += $lines
    $report += ""
    $lines | ForEach-Object { Write-Host "   $_" }
}

New-Item -ItemType Directory -Force target/perf | Out-Null
$file = "target/perf/$(Get-Date -Format 'yyyy-MM-dd-HHmm')-$Preset-$Window.txt"
$report | Out-File -Encoding utf8 $file
Write-Host "Résumé : $file" -ForegroundColor Green
if ($failed) { exit 1 }
