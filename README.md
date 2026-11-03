# Star Histories

RPG solo **space opera** en 3D à la première personne, dans la galaxie d'Astréa en 425at. Un monde qui évolue tout seul, où le joueur monte en grade, change de métier, rejoint des groupes, s'allie… et trahit.

Fait avec [Bevy](https://bevyengine.org/) 0.20 (Rust) ; modèles 3D générés dans Blender.

**État** : jalon M0 (fondations) terminé, M1 en cours (voir [`docs/ROADMAP.md`](docs/ROADMAP.md)).

## Lancer le jeu (développement)

Prérequis Windows : Rust (rustup, cible MSVC) et Visual Studio Build Tools avec la charge « Développement Desktop en C++ ». Prérequis Linux (Arch, CachyOS) : `tools/setup-linux.sh` installe tout.

```
cargo dev
```

La première compilation prend plusieurs minutes. Vérification complète avant un commit :

```powershell
powershell -ExecutionPolicy Bypass -File tools/check.ps1
```

```bash
tools/check.sh
```

Raccourcis de développement : F1 inspecteur, F2 caméra libre, F3 FPS. Démarrage direct dans une vue : `$env:SH_START = "galaxy"; cargo dev` sous Windows, `SH_START=galaxy cargo dev` sous Linux (voir `AGENTS.md` §3.3).

## Documentation

- [`AGENTS.md`](AGENTS.md) : règles et architecture (pour les contributeurs humains comme IA)
- [`docs/GDD.md`](docs/GDD.md) : conception du jeu
- [`docs/ROADMAP.md`](docs/ROADMAP.md) : feuille de route
- [`_context/`](_context/) : l'univers (canon) de Star Histories
