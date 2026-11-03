# Star Histories

RPG solo **space opera** en 3D à la première personne, dans la galaxie d'Astréa en 425at. Un monde qui évolue tout seul, où le joueur monte en grade, change de métier, rejoint des groupes, s'allie… et trahit.

Fait avec [Bevy](https://bevyengine.org/) 0.20 (Rust) ; modèles 3D générés dans Blender.

**État** : mise en place terminée, développement du jalon M0 (voir [`docs/ROADMAP.md`](docs/ROADMAP.md)).

## Lancer le jeu (développement)

Prérequis Windows : Rust (rustup, cible MSVC) et Visual Studio Build Tools avec la charge « Développement Desktop en C++ ».

```powershell
cargo dev
```

La première compilation prend plusieurs minutes.

## Documentation

- [`AGENTS.md`](AGENTS.md) : règles et architecture (pour les contributeurs humains comme IA)
- [`docs/GDD.md`](docs/GDD.md) : conception du jeu
- [`docs/ROADMAP.md`](docs/ROADMAP.md) : feuille de route
- [`_context/`](_context/) : l'univers (canon) de Star Histories
