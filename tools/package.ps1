# Empaquetage (M7) : build release distribuable, assets et notice, dans une archive.
#   powershell -ExecutionPolicy Bypass -File tools/package.ps1
# Résultat : target/dist/StarHistories-<version>-windows.zip. Pendant de tools/package.sh.

$ErrorActionPreference = "Stop"
Set-Location (Split-Path $PSScriptRoot -Parent)
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    $env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"
}

$version = (Select-String -Path Cargo.toml -Pattern '^version = "(.+)"' | Select-Object -First 1).Matches[0].Groups[1].Value

# Les fichiers Git LFS doivent être là, pas leurs pointeurs (quelques octets).
$pointers = Get-ChildItem assets -Recurse -File -Include *.glb, *.png, *.wav, *.ttf | Where-Object { $_.Length -lt 200 }
if ($pointers) {
    Write-Host "Fichiers LFS absents (lancer git lfs pull) :" -ForegroundColor Red
    $pointers | ForEach-Object { Write-Host "   $($_.FullName)" }
    exit 1
}

Write-Host "== cargo build --release --features dist" -ForegroundColor Cyan
cargo build --release --features dist
if ($LASTEXITCODE -ne 0) {
    Write-Host "ÉCHEC : build release" -ForegroundColor Red
    exit 1
}

$name = "StarHistories-$version-windows"
$out = "target/dist/$name"
if (Test-Path $out) { Remove-Item -Recurse -Force $out }
New-Item -ItemType Directory -Force $out | Out-Null
Copy-Item target/release/star_histories.exe "$out/StarHistories.exe"
Copy-Item assets "$out/assets" -Recurse
Copy-Item tools/package/LISEZMOI.txt "$out/LISEZMOI.txt"

$zip = "target/dist/$name.zip"
if (Test-Path $zip) { Remove-Item -Force $zip }
Compress-Archive -Path "$out/*" -DestinationPath $zip
$size = [math]::Round((Get-Item $zip).Length / 1MB, 1)
Write-Host "Archive : $zip ($size Mo)" -ForegroundColor Green
