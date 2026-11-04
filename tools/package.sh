#!/usr/bin/env bash
# Empaquetage (M7) : build release distribuable, assets et notice, dans une archive.
#   tools/package.sh
# Résultat : target/dist/StarHistories-<version>-linux.tar.gz. Pendant de tools/package.ps1.

set -eu
cd "$(dirname "$0")/.."
if ! command -v cargo >/dev/null 2>&1; then
    export PATH="$HOME/.cargo/bin:$PATH"
fi

version=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)

# Les fichiers Git LFS doivent être là, pas leurs pointeurs (quelques octets).
pointers=$(find assets -type f \( -name '*.glb' -o -name '*.png' -o -name '*.wav' -o -name '*.ttf' \) -size -200c)
if [ -n "$pointers" ]; then
    echo "Fichiers LFS absents (lancer git lfs pull) :"
    echo "$pointers" | sed 's/^/   /'
    exit 1
fi

echo "== cargo build --release --features dist"
cargo build --release --features dist

name="StarHistories-${version}-linux"
out="target/dist/${name}"
rm -rf "$out"
mkdir -p "$out"
cp target/release/star_histories "$out/star_histories"
cp -r assets "$out/assets"
cp tools/package/LISEZMOI.txt "$out/LISEZMOI.txt"

tar -C target/dist -czf "target/dist/${name}.tar.gz" "$name"
echo "Archive : target/dist/${name}.tar.gz ($(du -h "target/dist/${name}.tar.gz" | cut -f1))"
