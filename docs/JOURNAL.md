# Journal de bord

> Une entrée courte par session (2 à 5 lignes) : ce qui a été fait, ce qui reste en suspens, les décisions prises. Le plus récent en haut.

## 03/11/2026 — Mise en place

- Installé : Visual Studio Build Tools 2026 (MSVC 14.51 + SDK 10.0.26100), Rust 1.99.1, uv 0.13.7, Blender 5.2.3 LTS (portable, utilisateur), addon MCP for Blender activé, serveur MCP déclaré dans `.mcp.json` (version 2.2.6 épinglée, télémétrie désactivée).
- Clone de référence `.reference/bevy-0.20.1` (exemples, sources, notes de migration), ignoré par Git.
- Squelette : workspace Cargo, binaire `star_histories`, crate `sh_game` (scène de test + autopilote `SH_EXIT_AFTER_FRAMES` / `SH_SCREENSHOT`), liaison dynamique en dev, linker `rust-lld`.
- Docs : `AGENTS.md`, `docs/GDD.md`, `docs/ROADMAP.md`, registres. Prochaine étape : **M0**.
