# Captures de référence à points de vue fixes (AGENTS.md §3.3, M8 : comparaison avant / après).
#   powershell -ExecutionPolicy Bypass -File tools/captures.ps1 -Prefix m8-avant
# Écrit target/shots/<Prefix>-<vue>.png pour chaque point de vue. Même liste dans captures.sh :
# modifie les deux scripts ensemble. -Only <vue>,<vue> pour n'en refaire que quelques-unes.

param(
    [string]$Prefix = "m8",
    [string]$Preset = "medium",
    [string]$Window = "1600x900",
    [string[]]$Only = @()
)

$ErrorActionPreference = "Continue"
$env:NO_COLOR = "1"
# Avec `powershell -File`, « -Only a,b » arrive en une seule chaîne : on la découpe.
$Only = @($Only | ForEach-Object { $_ -split "," } | Where-Object { $_ })
Set-Location (Split-Path $PSScriptRoot -Parent)
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    $env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"
}
New-Item -ItemType Directory -Force target/shots | Out-Null

# Frame des captures d'un écran ouvert 3 s après l'arrivée (SH_SCREEN, SH_CONTAINER,
# SH_TERMINAL, SH_TALK) : 700 frames sur le poste Windows (UHD 770). Le poste Linux tourne à
# plusieurs centaines d'i/s : captures.sh y met 2400 (AGENTS.md §4, captures des écrans).
$screenFrame = "700"

# Vue → variables de l'autopilote. Positions SH_SPAWN_AT : pieds du joueur, repère du jeu
# (Y-haut, −Z = avant du vaisseau), cap et tangage en degrés.
$views = [ordered]@{
    "cockpit"          = @{ SH_START = "sector:player_ship"; SH_SPAWN = "cockpit"; SH_SCREENSHOT_FRAME = "300" }
    "cockpit-poste"    = @{ SH_START = "sector:player_ship"; SH_SPAWN_AT = "2.6,0,-15.0,30,-10"; SH_SCREENSHOT_FRAME = "300" }
    "couloir"          = @{ SH_START = "sector:player_ship"; SH_SPAWN_AT = "0,0,-1.0,0,0"; SH_SCREENSHOT_FRAME = "300" }
    "soute"            = @{ SH_START = "sector:player_ship"; SH_SPAWN_AT = "9.4,0,11.6,35,-8"; SH_SCREENSHOT_FRAME = "300" }
    "sas"              = @{ SH_START = "sector:player_ship"; SH_SPAWN_AT = "-2.0,0,3.0,90,0"; SH_SCREENSHOT_FRAME = "300" }
    "machines"         = @{ SH_START = "sector:player_ship"; SH_SPAWN_AT = "0,0,14.8,180,-5"; SH_SCREENSHOT_FRAME = "300" }
    "hall"             = @{ SH_START = "sector:hexis_spaceport"; SH_SPAWN_AT = "-13.0,0,6.5,45,0"; SH_SCREENSHOT_FRAME = "300" }
    "bureau"           = @{ SH_START = "sector:hexis_spaceport"; SH_SPAWN_AT = "-20.0,0,-7.9,0,-5"; SH_SCREENSHOT_FRAME = "300" }
    "global-hexis"     = @{ SH_START = "global:hexis"; SH_RAID = "0"; SH_SCREENSHOT_FRAME = "150" }
    "global-enclume"   = @{ SH_START = "global:hexis"; SH_RAID = "0"; SH_ARRIVAL = "l_enclume,3000"; SH_SCREENSHOT_FRAME = "150" }
    "global-hub"       = @{ SH_START = "global:hexis"; SH_RAID = "0"; SH_ARRIVAL = "hexis_portal_hub,1500"; SH_SCREENSHOT_FRAME = "150" }
    "global-chantiers" = @{ SH_START = "global:hexis"; SH_RAID = "0"; SH_ARRIVAL = "hexis_shipyards,1500"; SH_SCREENSHOT_FRAME = "150" }
    "raid-cygnara"     = @{ SH_START = "global:cygnara"; SH_RAID = "1"; SH_SCREENSHOT_FRAME = "420" }
    "raid-proche"      = @{ SH_START = "global:cygnara"; SH_RAID = "1"; SH_SCREENSHOT_FRAME = "900" }
    "abordage"         = @{ SH_START = "sector:player_ship"; SH_SPAWN = "soute"; SH_BOARDING = "2"; SH_SCREENSHOT_FRAME = "300" }
    # M9 : Hexis de loin et côté nuit (textures de planète calculées en tâche de fond), la carte.
    "hexis-loin"       = @{ SH_START = "global:hexis"; SH_RAID = "0"; SH_LOOK = "-0.631,-0.003,-0.776"; SH_SCREENSHOT_FRAME = "400" }
    "hexis-nuit"       = @{ SH_START = "global:hexis"; SH_RAID = "0"; SH_ARRIVAL = "hexis,-2000000"; SH_LOOK = "-0.222,0.208,0.953"; SH_SCREENSHOT_FRAME = "400" }
    "carte"            = @{ SH_START = "galaxy"; SH_SCREENSHOT_FRAME = "200" }
    "carte-karza"      = @{ SH_START = "galaxy"; SH_ROUTE = "karza"; SH_HOVER = "karza"; SH_SCREENSHOT_FRAME = "200" }
    # M9 : écran du personnage, rangement d'un casier, bornes et PNJ du spatioport, options.
    "inventaire"       = @{ SH_START = "sector:player_ship"; SH_SPAWN = "soute"; SH_SCREEN = "inventory"; SH_GIVE = "ration:13,spare_parts:25,medkit:2,datapad,flight_suit,data_chip:3"; SH_SCREENSHOT_FRAME = $screenFrame }
    "competences"      = @{ SH_START = "sector:player_ship"; SH_SPAWN = "soute"; SH_SCREEN = "skills"; SH_XP = "piloting:650,navigation:260,shooting:120,commerce:1300,engineering:40"; SH_SCREENSHOT_FRAME = $screenFrame }
    "casier"           = @{ SH_START = "sector:player_ship"; SH_SPAWN = "soute"; SH_CONTAINER = "corvette_chambre_casier_1"; SH_GIVE = "ration:4,spare_parts:6,data_chip:2"; SH_SCREENSHOT_FRAME = $screenFrame }
    "borne-contrats"   = @{ SH_START = "sector:hexis_spaceport"; SH_SPAWN = "hall"; SH_TERMINAL = "contrats"; SH_SCREENSHOT_FRAME = $screenFrame }
    "borne-chantier"   = @{ SH_START = "sector:hexis_spaceport"; SH_SPAWN = "hall"; SH_TERMINAL = "chantier"; SH_SCREENSHOT_FRAME = $screenFrame }
    "pnj-parler"       = @{ SH_START = "sector:hexis_spaceport"; SH_SPAWN_AT = "-20.0,0,-7.9,0,-5"; SH_TALK = "contracts"; SH_SCREENSHOT_FRAME = $screenFrame }
    "options-touches"  = @{ SH_START = "menu"; SH_OPTIONS = "keys:piloting"; SH_SCREENSHOT_FRAME = "200" }
}

$failed = @()
foreach ($name in $views.Keys) {
    if ($Only.Count -gt 0 -and $Only -notcontains $name) { continue }
    $vars = $views[$name]
    $shot = "target/shots/$Prefix-$name.png"
    Write-Host "== $shot" -ForegroundColor Cyan
    $env:SH_SCREENSHOT = $shot
    $env:SH_PRESET = $Preset
    $env:SH_WINDOW = $Window
    foreach ($k in $vars.Keys) { Set-Item "Env:$k" $vars[$k] }
    $log = & cargo dev 2>&1 | ForEach-Object { "$_" }
    $code = $LASTEXITCODE
    foreach ($k in @($vars.Keys) + @("SH_SCREENSHOT", "SH_PRESET", "SH_WINDOW")) {
        Remove-Item "Env:$k" -ErrorAction SilentlyContinue
    }
    $log | Select-String -Pattern "ERROR|panicked" | ForEach-Object { "   $_" }
    if ($code -ne 0 -or -not (Test-Path $shot)) {
        Write-Host "ÉCHEC : $name (code $code)" -ForegroundColor Red
        $failed += $name
    }
}
if ($failed.Count -gt 0) {
    Write-Host "Captures manquées : $($failed -join ', ')" -ForegroundColor Red
    exit 1
}
Write-Host "Captures écrites dans target/shots/$Prefix-*.png" -ForegroundColor Green
