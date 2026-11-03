# Reprendre le développement sur un autre PC

> Procédure pour continuer Star Histories sur une autre machine Windows, là où la roadmap s'est arrêtée.
> Règle d'or : **un seul poste travaille à la fois sur `main`.** Avant de changer de poste, tout est commité et poussé ; en revenant sur un poste, on commence par `git pull`.

## 1. Sur l'ancien PC, avant de partir

1. Laisse l'agent en cours finir son jalon, puis envoie-lui :

   > Quand le jalon en cours sera terminé (tous ses critères « Fini quand » vérifiés), commite tout, y compris `.gitignore` et `docs/reprise-autre-pc.md`, pousse sur `origin/main`, vérifie que `git status` est propre et que les fichiers LFS (`.glb`, `.blend`, polices) sont bien partis, puis **arrête-toi sans commencer le jalon suivant** : la suite se fera sur un autre PC.

2. Vérifie sur https://github.com/veHRz/Star-Histories que le dernier commit est là et que les cases du jalon sont cochées dans `docs/ROADMAP.md`.
3. Ferme les sessions Claude de ce projet sur l'ancien PC.

## 2. Sur le nouveau PC

Prérequis : l'application Claude (onglet Code) installée et connectée. Le reste est installé par l'agent.

1. Crée le dossier parent du projet (ex. `D:\Projets`), ouvre une session Claude Code dessus (modèle Opus 5.5) et colle le **prompt 1**.
2. Quand l'agent a fini : ferme complètement Claude (y compris depuis la zone de notification), relance-le, ouvre une **nouvelle** session sur le dossier `Star Histories` cloné, approuve le serveur MCP `blender` quand il te le demande, et colle le **prompt 2**.

### Prompt 1 — installation du poste

```text
Tu prépares ce PC Windows pour reprendre le développement de **Star Histories**, un RPG space opera en Rust (Bevy 0.20.1) dont les modèles 3D sont produits dans Blender via le MCP « MCP for Blender ». Le projet a commencé sur un autre PC ; le dépôt est https://github.com/veHRz/Star-Histories (public). Pendant cette session, tu ne codes rien : tu installes, tu vérifies, puis tu me passes la main.

Règles :
- Installe en mode utilisateur quand c'est possible. Si une installation exige les droits administrateur, préviens-moi avant de la lancer : une fenêtre UAC s'ouvrira et je la validerai.
- Vérifie chaque installation (version, et empreinte SHA-256 quand je te la donne). Ne touche pas à ma configuration Git globale sans me demander.
- Lance les commandes longues en arrière-plan. Si un outil fraîchement installé est introuvable dans un shell, ajoute son dossier au PATH de la session (`%USERPROFILE%\.cargo\bin`, `%LOCALAPPDATA%\Microsoft\WinGet\Links`).
- Si une action est refusée par le système de permissions, ne la contourne pas : dis-moi ce que je dois faire à la main.

Étapes :
1. État des lieux : version de Windows, droits administrateur, GPU, espace disque libre (prévoir ≈ 25 Go), outils présents (git, git lfs, winget, rustup/cargo, uv, Blender, MSVC via vswhere). Montre-moi un tableau, puis enchaîne.
2. Git : installe Git for Windows s'il manque (winget `Git.Git`, il inclut Git LFS), puis `git lfs install`. Si `git config --global user.name` ou `user.email` est vide, demande-moi quoi mettre (identité utilisée jusqu'ici : `veHRz <122048918+veHRz@users.noreply.github.com>`).
3. Clone le dépôt dans le sous-dossier `Star Histories` du dossier courant, puis `git lfs pull`. Vérifie que `assets/fonts/*.ttf` et les `.glb` de `assets/models/` sont de vrais fichiers et pas des pointeurs LFS (plus de 1 Ko). Lis ensuite `AGENTS.md` en entier (les règles du projet), puis la dernière entrée de `docs/JOURNAL.md` et l'état de `docs/ROADMAP.md`.
4. Compilateur : Visual Studio Build Tools 2026 avec la charge C++ (winget `Microsoft.VisualStudio.BuildTools`, `--override "--wait --passive --norestart --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"`). Admin requis : préviens-moi. Vérifie la présence de `link.exe` (MSVC) et du SDK Windows (`Windows Kits\10\Lib\<version>\um\x64\kernel32.lib`).
5. Rust : rustup-init officiel (https://static.rust-lang.org/rustup/dist/x86_64-pc-windows-msvc/rustup-init.exe) avec `-y --default-toolchain 1.99.1 --profile minimal -c rustfmt -c clippy` (version épinglée par `rust-toolchain.toml`).
6. uv : winget `astral-sh.uv` avec `--scope user`.
7. Blender 5.2.3 LTS portable, au même emplacement que sur l'autre PC (des scripts du dépôt en dépendent) : télécharge https://download.blender.org/release/Blender5.2/blender-5.2.3-windows-x64.zip, vérifie son empreinte avec https://download.blender.org/release/Blender5.2/blender-5.2.3.sha256, extrais-le pour que l'exécutable soit `%LOCALAPPDATA%\Programs\Blender Foundation\Blender 5.2\blender.exe`, et crée un raccourci « Blender 5.2 » dans le menu Démarrer de l'utilisateur.
8. Addon MCP for Blender, identique à celui de l'autre PC : télécharge https://raw.githubusercontent.com/ahujasid/mcp-for-blender/992233bb258975a05eec8b5691465c50c2831e34/addon.py, vérifie que son SHA-256 vaut `6c2917c157c26d7549adcbd480243db2154a9b06bc580664f090530c8ef3801f`, copie-le en `%APPDATA%\Blender Foundation\Blender\5.2\scripts\addons\blender_mcp.py`, puis active-le de façon persistante : `blender.exe --background --python-expr "import bpy, addon_utils; addon_utils.enable('blender_mcp', default_set=True, persistent=True); bpy.ops.wm.save_userpref()"`. Vérifie avec `addon_utils.check('blender_mcp')`. Le serveur MCP est déjà déclaré dans `.mcp.json` du dépôt : rien d'autre à faire.
9. Sources de référence de Bevy (ignorées par Git, indispensables à la méthode d'AGENTS.md §4) : à la racine du dépôt, `git clone --depth 1 --branch v0.20.1 https://github.com/bevyengine/bevy.git .reference/bevy-0.20.1`.
10. Première compilation, en arrière-plan (≈ 15 min) : `cargo build --features dev` à la racine du dépôt. Ensuite, `powershell -ExecutionPolicy Bypass -File tools/check.ps1` doit passer en entier. Fais une capture d'écran avec l'autopilote (AGENTS.md §3.3) sur le point d'entrée le plus récent de `tools/check.ps1`, et regarde-la.
11. GPU : si ce PC n'a pas d'Intel UHD 770, dis-le-moi. Les budgets de performance restent ceux de l'UHD 770 (AGENTS.md §0 et §10), et les mesures faites ici ne valent pas pour la machine de référence.
12. Bilan : tableau des outils installés (versions), résultat de `tools/check.ps1`, ce qui a échoué ou reste à faire. Ajoute une entrée « Installation d'un second poste » en haut de `docs/JOURNAL.md` et commite-la (`docs(journal): installation d'un second poste`), sans pousser. Termine en me disant de redémarrer complètement Claude, d'ouvrir une nouvelle session sur le dossier du dépôt, d'approuver le serveur MCP `blender` et de coller le prompt 2 de `docs/reprise-autre-pc.md`.
```

### Prompt 2 — reprise du développement

```text
Tu reprends le développement de **Star Histories** sur ce poste, installé lors de la session précédente (voir la dernière entrée de `docs/JOURNAL.md`). Tu es l'agent de développement ; je suis l'auteur de l'univers.

1. Lis `AGENTS.md` en entier (tes règles, elles priment sur tes habitudes et sur ta mémoire de Bevy), `docs/ROADMAP.md`, `docs/GDD.md`, les dernières entrées de `docs/JOURNAL.md`, `docs/QUESTIONS.md` et `docs/propositions.md`, puis les sections 0, 2, 3, 9, 10 et 11 de `_context/STAR-HISTORIES-export-IA.md` (le reste à la demande, par recherche).
2. Vérifie l'état : `git pull`, `git log --oneline -10`, `git status` propre ; ouvre Blender (`Start-Process "$env:LOCALAPPDATA\Programs\Blender Foundation\Blender 5.2\blender.exe"`) et appelle l'outil MCP `get_addon_status` ; lance `tools/check.ps1`. Si quelque chose ne va pas, dis-le-moi en une ligne avant d'aller plus loin.
3. Reprends à la première tâche non cochée de `docs/ROADMAP.md` (normalement le début de M2 : vue Globale, système d'Hexis et pilotage). Si des cases du jalon précédent sont encore ouvertes, signale-le-moi d'abord.
4. Même méthode que jusqu'ici (AGENTS.md §12) : petits incréments, `cargo fmt` + `cargo lint` + `cargo t` verts, captures d'écran que tu regardes, roadmap cochée, journal, commits. À la fin de chaque jalon : vérifie ses critères « Fini quand », pousse sur `origin/main`, fais-moi un bilan court (ce qui marche avec le chemin des captures, les déviations, les nouvelles propositions et questions), puis enchaîne le jalon suivant. Ne t'arrête pour me demander que si une installation exige les droits administrateur, si une dépense serait nécessaire, ou si une décision d'univers ou de direction artistique bloque vraiment la suite.
```
