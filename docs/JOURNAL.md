# Journal de bord

> Une entrée courte par session (2 à 5 lignes) : ce qui a été fait, ce qui reste en suspens, les décisions prises. Le plus récent en haut.

## 03/11/2026 — M0 Fondations

- Environnement vérifié : jeu OK (fumée 120 frames), MCP Blender OK (Blender 5.2.3, addon 1.8). Réponses de l'auteur intégrées : **interface en anglais** (textes dans `assets/data/lang/en.ron`), temps ×30, année de 365 jours, DA « réaliste stylisé ».
- Crates `sh_core` (dates, grille, monnaies, identifiants typés, RNG ChaCha8 à sous-flux), `sh_data` (schémas, chargement, validation complète dont fuite de 🔒), `sh_sim` (socle), outil `sh_import` (carte JSON → `galaxy.ron`, rejouable, testé). Factions, classes, formations, monnaies transcrites à la main.
- `sh_game` : états + sous-états, chargement asynchrone validé, menu, entrées par contextes (BSN), vues de test Galaxie / Globale / Sectorielle, préréglages + échelle de rendu, autopilote `SH_START` + temps de frame, outils `dev` (F1 inspecteur, F2 caméra libre, F3 FPS), `tools/check.ps1`.
- **Pièges Bevy 0.20 trouvés** (détails AGENTS.md §4) : clustering GPU des lumières inopérant sur UHD 770 (forcé en CPU) ; caméra HDR + caméra d'interface à composer par mélange ; exposition intérieure.
- Déviation : les tâches de M0 sont regroupées en trois commits (interdépendantes) au lieu d'un par tâche. Police Inter (OFL) tirée de Blender et convertie en TTF : la police par défaut de Bevy n'a pas d'accents.

## 03/11/2026 — Mise en place

- Installé : Visual Studio Build Tools 2026 (MSVC 14.51 + SDK 10.0.26100), Rust 1.99.1, uv 0.13.7, Blender 5.2.3 LTS (portable, utilisateur), addon MCP for Blender activé, serveur MCP déclaré dans `.mcp.json` (version 2.2.6 épinglée, télémétrie désactivée).
- Clone de référence `.reference/bevy-0.20.1` (exemples, sources, notes de migration), ignoré par Git.
- Squelette : workspace Cargo, binaire `star_histories`, crate `sh_game` (scène de test + autopilote `SH_EXIT_AFTER_FRAMES` / `SH_SCREENSHOT`), liaison dynamique en dev, linker `rust-lld`.
- Docs : `AGENTS.md`, `docs/GDD.md`, `docs/ROADMAP.md`, registres. Prochaine étape : **M0**.
