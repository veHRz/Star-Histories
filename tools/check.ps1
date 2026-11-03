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
$starts = @("menu", "galaxy", "global:hexis", "sector:test")
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

Write-Host "Tout est vert." -ForegroundColor Green
