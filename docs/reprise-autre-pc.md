# Travailler sur plusieurs postes

> Le développement alterne entre deux postes : **Windows 11** (machine de référence, Intel UHD 770) et **Linux** (CachyOS, Radeon RX 9070 XT). Détails des postes : `AGENTS.md` §3.1.
> Règle d'or : **un seul poste travaille à la fois sur `main`.** Avant de changer de poste, tout est commité et poussé ; en arrivant sur un poste, on commence par `git pull`.
> Les mesures de performance ne valent que sur le poste Windows (`AGENTS.md` §10).

## 1. Avant de quitter un poste

1. Envoie à l'agent en cours :

   > Je change de poste. Termine la tâche en cours ou arrête-la à un état propre (`cargo fmt`, `cargo lint`, `cargo t` verts), mets à jour `docs/ROADMAP.md` et `docs/JOURNAL.md` (en précisant le poste et ce qui reste à faire, notamment les mesures de performance à refaire sous Windows), commite, pousse sur `origin/main`, vérifie que `git status` est propre et que les fichiers LFS sont partis, puis arrête-toi. Si le push échoue, dis-moi pourquoi et quoi faire.

2. Si le push échoue sous Linux avec `Permission denied (publickey)`, charge ta clé SSH dans un terminal (`ssh-add ~/.ssh/id_ed25519`), puis pousse toi-même (`git push`) ou demande à l'agent de réessayer.
3. Ferme les sessions Claude de ce projet sur ce poste.

## 2. Arriver sur un poste déjà installé

C'est le cas courant : les deux postes sont installés.

1. Ouvre une session Claude Code sur le dossier du dépôt (modèle Opus 5.5), approuve le serveur MCP `blender` si on te le demande.
2. Colle le **prompt de reprise** (section 4). Il commence par `git pull`.

## 3. Installer un nouveau poste

### Windows

Prérequis : l'application Claude (onglet Code) installée et connectée. Le reste est installé par l'agent.

1. Crée le dossier parent du projet (ex. `D:\Projets`), ouvre une session Claude Code dessus (modèle Opus 5.5) et colle le **prompt d'installation Windows**.
2. Quand l'agent a fini : ferme complètement Claude (y compris depuis la zone de notification), relance-le, ouvre une **nouvelle** session sur le dossier `Star Histories` cloné, approuve le serveur MCP `blender` et colle le **prompt de reprise**.

#### Prompt d'installation Windows

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
7. Blender 5.2.3 LTS portable, au même emplacement que sur l'autre PC Windows (des scripts du dépôt en dépendent) : télécharge https://download.blender.org/release/Blender5.2/blender-5.2.3-windows-x64.zip, vérifie son empreinte avec https://download.blender.org/release/Blender5.2/blender-5.2.3.sha256, extrais-le pour que l'exécutable soit `%LOCALAPPDATA%\Programs\Blender Foundation\Blender 5.2\blender.exe`, et crée un raccourci « Blender 5.2 » dans le menu Démarrer de l'utilisateur.
8. Addon MCP for Blender, identique à celui des autres postes : télécharge https://raw.githubusercontent.com/ahujasid/mcp-for-blender/992233bb258975a05eec8b5691465c50c2831e34/addon.py, vérifie que son SHA-256 vaut `6c2917c157c26d7549adcbd480243db2154a9b06bc580664f090530c8ef3801f`, copie-le en `%APPDATA%\Blender Foundation\Blender\5.2\scripts\addons\blender_mcp.py`, puis active-le de façon persistante : `blender.exe --background --python-expr "import bpy, addon_utils; addon_utils.enable('blender_mcp', default_set=True, persistent=True); bpy.ops.wm.save_userpref()"`. Vérifie avec `addon_utils.check('blender_mcp')`. Le serveur MCP est déjà déclaré dans `.mcp.json` du dépôt : rien d'autre à faire.
9. Sources de référence de Bevy (ignorées par Git, indispensables à la méthode d'AGENTS.md §4) : à la racine du dépôt, `git clone --depth 1 --branch v0.20.1 https://github.com/bevyengine/bevy.git .reference/bevy-0.20.1`.
10. Première compilation, en arrière-plan (≈ 15 min) : `cargo build --features dev` à la racine du dépôt. Ensuite, `powershell -ExecutionPolicy Bypass -File tools/check.ps1` doit passer en entier. Fais une capture d'écran avec l'autopilote (AGENTS.md §3.3) sur le point d'entrée le plus récent de `tools/check.ps1`, et regarde-la.
11. GPU : si ce PC n'a pas d'Intel UHD 770, dis-le-moi. Les budgets de performance restent ceux de l'UHD 770 (AGENTS.md §0 et §10), et les mesures faites ici ne valent pas pour la machine de référence.
12. Bilan : tableau des outils installés (versions), résultat de `tools/check.ps1`, ce qui a échoué ou reste à faire. Ajoute une entrée « Installation d'un poste Windows » en haut de `docs/JOURNAL.md`, commite-la (`docs(journal): installation d'un poste Windows`) et pousse-la. Termine en me disant de redémarrer complètement Claude, d'ouvrir une nouvelle session sur le dossier du dépôt, d'approuver le serveur MCP `blender` et de coller le prompt de reprise de `docs/reprise-autre-pc.md`.
```

### Linux (Arch, CachyOS)

Prérequis : l'application Claude (onglet Code) installée et connectée, le dépôt cloné (`git clone git@github.com:veHRz/Star-Histories.git`).

1. Dans un terminal, à la racine du dépôt, lance **toi-même** `tools/setup-linux.sh` : il demande le mot de passe sudo s'il manque des paquets (git-lfs, uv, Blender, bibliothèques de Bevy), puis récupère les fichiers LFS, les sources de référence de Bevy et installe l'addon MCP de Blender. Il est rejouable sans risque.
2. Ouvre une session Claude Code sur le dossier du dépôt (modèle Opus 5.5) et colle le **prompt d'installation Linux**.
3. Quand l'agent a fini : ferme complètement Claude, relance-le, ouvre une **nouvelle** session sur le dépôt, approuve le serveur MCP `blender` et colle le **prompt de reprise**.

#### Prompt d'installation Linux

```text
Tu vérifies ce poste Linux (Arch / CachyOS) pour reprendre le développement de **Star Histories**, un RPG space opera en Rust (Bevy 0.20.1) dont les modèles 3D sont produits dans Blender via le MCP « MCP for Blender ». J'ai déjà lancé `tools/setup-linux.sh`. Pendant cette session, tu ne codes rien : tu vérifies, puis tu me passes la main.

Règles : tu n'as pas sudo ; s'il manque un paquet système, donne-moi la commande `sudo pacman -S …` à lancer. Ne touche pas à ma configuration Git globale. Lance les commandes longues en arrière-plan. Si une action est refusée par le système de permissions, ne la contourne pas : dis-moi ce que je dois faire à la main.

1. Lis `AGENTS.md` en entier (en particulier §3.1 bis), puis la dernière entrée de `docs/JOURNAL.md` et l'état de `docs/ROADMAP.md`.
2. Vérifie : `git status` propre et `git pull` ; `rustc --version` (1.99.1) ; `git lfs ls-files` et que `assets/fonts/*.ttf` et les `.glb` de `assets/models/` font plus de 1 Ko ; `.reference/bevy-0.20.1` présent en `v0.20.1` ; `blender --version` (5.2.3) ; l'addon `~/.config/blender/5.2/scripts/addons/blender_mcp.py` (SHA-256 `6c2917c157c26d7549adcbd480243db2154a9b06bc580664f090530c8ef3801f`) ; `uvx --version`. Montre-moi un tableau.
3. Première compilation en arrière-plan (≈ 15 min) : `cargo build --features dev`. Puis `tools/check.sh` doit passer en entier. Fais une capture d'écran avec l'autopilote (AGENTS.md §3.3) sur le point d'entrée le plus récent de `tools/check.sh`, et regarde-la.
4. Bilan : ce qui marche, ce qui a échoué. Ajoute une entrée « Installation du poste Linux » en haut de `docs/JOURNAL.md`, commite-la (`docs(journal): installation du poste Linux`) et pousse-la (si le push SSH échoue, dis-moi de charger ma clé avec `ssh-add`). Termine en me disant de redémarrer Claude, d'ouvrir une nouvelle session, d'approuver le serveur MCP `blender` et de coller le prompt de reprise.
```

## 4. Prompt de reprise (les deux postes)

```text
Tu reprends le développement de **Star Histories** sur ce poste. Tu es l'agent de développement ; je suis l'auteur de l'univers. Le travail alterne entre un poste Windows (machine de référence, UHD 770) et un poste Linux (AGENTS.md §3.1) : regarde d'abord sur lequel tu es.

1. Lis `AGENTS.md` en entier (tes règles, elles priment sur tes habitudes et sur ta mémoire de Bevy), `docs/ROADMAP.md`, `docs/GDD.md`, les dernières entrées de `docs/JOURNAL.md` (elles disent sur quel poste et où on s'est arrêté), `docs/QUESTIONS.md` et `docs/propositions.md`, puis les sections 0, 2, 3, 9, 10 et 11 de `_context/STAR-HISTORIES-export-IA.md` (le reste à la demande, par recherche).
2. Vérifie l'état : `git pull`, `git log --oneline -10`, `git status` propre ; ouvre Blender (Windows : `Start-Process "$env:LOCALAPPDATA\Programs\Blender Foundation\Blender 5.2\blender.exe"` ; Linux : `setsid blender >/dev/null 2>&1 &`) et appelle l'outil MCP `get_addon_status` ; lance la vérification complète (Windows : `powershell -ExecutionPolicy Bypass -File tools/check.ps1` ; Linux : `tools/check.sh`). Si quelque chose ne va pas, dis-le-moi en une ligne avant d'aller plus loin.
3. Reprends à la première tâche non cochée de `docs/ROADMAP.md`. Au 03/11/2026, c'est la fin de M1 (éclairage d'intérieur performant, critères de performance non atteints : voir le journal), avant M2. Sur le poste Linux, les mesures de performance ne valent pas (AGENTS.md §10) : implémente et vérifie visuellement, laisse les critères de performance ouverts et note dans le journal ce qui reste à mesurer sous Windows ; si tout ce qui reste dans le jalon est une mesure, dis-le-moi et propose de quoi avancer en attendant. Sur le poste Windows, commence par refaire les mesures que le journal signale comme en attente (`tools/perf.ps1`, en plein écran 1080p).
4. Même méthode que jusqu'ici (AGENTS.md §12) : petits incréments, `cargo fmt` + `cargo lint` + `cargo t` verts, captures d'écran que tu regardes, roadmap cochée, journal (avec le poste), commits. À la fin de chaque jalon : vérifie ses critères « Fini quand », pousse sur `origin/main`, fais-moi un bilan court (ce qui marche avec le chemin des captures, les déviations, les nouvelles propositions et questions), puis enchaîne le jalon suivant. Ne t'arrête pour me demander que si une installation exige les droits administrateur, si une dépense serait nécessaire, ou si une décision d'univers ou de direction artistique bloque vraiment la suite. Quand je te dis que je change de poste, suis la section 1 de `docs/reprise-autre-pc.md`.
```
