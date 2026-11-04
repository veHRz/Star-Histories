# Journal de bord

> Une entrée courte par session (2 à 5 lignes) : ce qui a été fait, ce qui reste en suspens, les décisions prises. Le plus récent en haut.

## 04/11/2026 — Poste Windows : M8, lot 0 (orchestrateur et sous-agent artiste)

- **Lot 0 livré** par le sous-agent `artiste-blender` (≈ 1 h 30) : 15 matériaux « futur usé » procéduraux (numpy, raccords parfaits, déterministes), 50 PNG, trim sheet, fonctions `textured_material` / `uv_box` / `uv_trim` / `export_gltf`, salle témoin `sector:salle_temoin` (coursive froide à pans coupés, atelier ambré, lightmaps à 128 échantillons). Côté jeu (`materials.rs`) : glTF séparé à textures partagées, préchargement, mipmaps sur un fil de calcul, répétition, anisotrope par préréglage. Vérifié en jeu : `target/shots/m8-temoin-{entree,atelier}.png` (Moyen, 115–140 i/s ici). Détails et points ouverts (≈ 207 Mio de textures en mémoire vidéo) : `docs/refonte-visuelle.md` §9.
- **Décision de l'auteur (Q15)** : la machine de référence est faible, la cible de performance passe de Moyen à **Bas** (≥ 60 i/s en 1080p) ; `perf.ps1` / `perf.sh` mesurent en Bas par défaut.
- Avant M8 : `tools/check.ps1` vert, captures de l'avant à points de vue fixes (`tools/captures.ps1`, nouveau ; non versionnées). Après le lot 0, à la demande de l'auteur, seuls `cargo t` et un démarrage de la salle témoin ont été refaits : **relancer `tools/check.sh` en entier** sur le poste Linux.
- **Reste** (poste Linux) : refaire l'avant, captures complètes de la salle témoin, point de contrôle avec l'auteur (direction, lot U), puis les vagues A, D, E, F, T et B, C. Mesures de performance finales sous Windows, en Bas.

## 04/11/2026 — Poste Windows : préparation de M8

- L'auteur a testé la tranche de M7 : elle tourne, mais elle est « moche » (murs plats, aucun détail) et il manque un vrai tutoriel. Décision (Q14) : refonte visuelle futuriste, « futur usé » inspiré de Star Wars sans emprunt, et tutoriel guidé désactivable. Direction sonore P27 validée « pour un début ».
- Préparé pour un agent orchestrateur dans une nouvelle conversation : plan `docs/refonte-visuelle.md` (brief artistique, contraintes de performance et de pipeline, lots à propriétaire unique, vagues, point de contrôle avec l'auteur après le lot 0, tests par l'orchestrateur seul), sous-agents `.claude/agents/artiste-blender.md` (Blender en arrière-plan, rendus regardés, ni cargo ni git) et `.claude/agents/dev-jeu.md` (`cargo lint` seulement), jalon M8 dans la feuille de route, AGENTS.md (§9.3, §12.5), prompt de départ `docs/prompt-m8.md`.

## 04/11/2026 — Poste Windows : retour de l'auteur sur le menu de pause

- Après « Reprendre », l'écran de pause restait affiché (rien ne l'effaçait une fois la pause retirée) ; un double Échap plantait : la condition du groupe chaîné (fermeture puis affichage) n'était évaluée qu'une fois, et la fermeture retirait la pause entre les deux. Même défaut latent dans les options (bouton « Retour »). Conditions posées sur chaque système, écran effacé en reprenant ; scénario `pause_escape` au vrai clavier ajouté aux deux scripts. Les tests passaient : aucun ne fermait la pause par Échap ni par ses boutons.

## 04/11/2026 — Poste Windows : M7, la finition

- **Performance** (UHD 770, release, Moyen, plein écran 1080p, `tools/perf.ps1`) : les temps GPU par passe (`SH_GPU_TIMINGS=1`) montraient la passe opaque à 19–23 ms sur ~30, presque tout dans les lumières ponctuelles évaluées par pixel. Puis, en A/B alternés (la machine partagée faisait varier les mesures isolées de ±30 %) : plus de lumières dynamiques dans les cellules cuites (`baked_cell_lights`), filtrage d'ombre `Hardware2x2`, ombre du soleil 1024 / 1 cascade / 40 m, vitrages teintés sans reflets (`lit_glass`), rendu 3D à 0,8. Écartés : prépasse de profondeur (plus lent), ombres du soleil coupées (le soleil traverse la coque : les lumières ne filtrent pas par maillage). Résultat : `walk_ship` 12,1 ms (83 i/s, aucune frame > 33 ms), `fly_ship` 15,1, `port_loop` 16,0 (56 pics aux amarrages), `raid_win` 16,5 (61 i/s, 43 pics), abordages 11,6–12,9. Critères de performance de M1, M2, M6 et M7 atteints en moyenne ; restent des pics de 50 ms aux transitions.
- **Sauvegarde** (Q7) : RON versionné dans `%APPDATA%\StarHistories` ; monde sans constantes (la graine suffit), capitaine, horloge, connaissance, codex, conseils, lieu (à quai ou pose en vol). Menu de pause (Échap, temps suspendu), Continuer / Charger, F5 / F9, sauvegarde automatique, Homme de fer. Tests : un monde rechargé continue la même histoire ; scénario `save_load` (vol et quai). L'autopilote n'écrit jamais chez le joueur.
- **Options** (`settings.ron`) : qualité, plein écran, volumes, sensibilité, axe inversé, conseils, touches remappables par touche (les aides suivent). **Son** : 17 WAV synthétisés par `tools/sh_audio` (P27), musiques de vol et de combat, ambiances, effets observés sans toucher aux systèmes de jeu. **Codex** (K) : fiches des seuls champs publics, test anti-secret. **Conseils** de première fois (tutoriel léger). **Empaquetage** : `tools/package.ps1` (archive de 42,5 Mo testée hors de Cargo).
- **Reste** : le test de l'auteur (dernier critère de M7) ; la direction sonore à valider (P27) ; les pics aux transitions ; PNJ sans squelette ; santé et munitions à pied non sauvegardées.

## 04/11/2026 — Poste Windows : premiers essais de l'auteur, nouvelle partie et Échap

- **« Nouvelle partie » ouvrait encore le vaisseau seul en vue Sectorielle** (le lieu de test de M1, `menu.rs` n'avait pas suivi M2) : pas d'espace, sas fermé, console « en veille à quai ». Elle démarre maintenant comme le départ P1 le prévoit : aux commandes, en vue Globale du système de départ (`EnterGame::new_game`, lu dans les données). `SH_START=new` rejoue exactement ce chemin (fumée de `check.ps1` / `check.sh`). Message de la console de pilotage reformulé : il renvoie au siège du pilote.
- **Échap plantait** (`living::show_news` lisait `State<ViewLevel>`, absent hors partie) et une nouvelle partie aurait repris le monde de la partie quittée : la partie quittée (Échap ou mort) est effacée à la sortie de `InGame` (`state::forget_game`, auparavant dans le seul chemin de la mort). L'effacer en plein `Update` faisait planter le terminal : la condition d'un groupe de systèmes chaîné est évaluée une fois, avant que les ressources disparaissent. Scénario `menu_return` ajouté aux deux scripts.
- Les scénarios de l'autopilote démarraient tous par `SH_START` : aucun ne passait par le menu, d'où ces deux bugs restés invisibles depuis M2.

## 04/11/2026 — Poste Windows : vérification de M6, mesures de performance (M1, M2, M6)

- **Vérification** : `tools/check.ps1` vert sous Windows (fmt, lint, tests, fumée, tous les scénarios dont `raid_win`, `raid_flee`, `raid_lose`, `firefight`, `firefight_lose`) ; `walk_ship` monte bien l'escalier à ≈ 45 i/s. Captures `target/shots/m6-*.png` regardées : correctes. Défauts mineurs déjà connus : étiquettes qui se chevauchent (« HexisApproche du spatioport d'Hexis », hub de Cygnara sous une corvette).
- **Bride trouvée** : Bevy limite à 60 Hz une fenêtre sans focus (`WinitSettings::game`), le cas des fenêtres lancées par l'autopilote. Les frames rapides comptaient ≈ 16,7 ms (le « calé sur 60 Hz » noté sous Linux). L'autopilote passe en `WinitSettings::continuous()`. Nouveaux outils : `SH_WINDOW=<L>x<H>|fullscreen` et `tools/perf.ps1` / `perf.sh` (scénarios des critères en release, résumé dans `target/perf/`).
- **Mesures** (UHD 770, pilote 31.0.101.4953, Vulkan, release, moyenne / pire pendant le scénario) :

  | Scénario | Moyen 1080p | Bas 1080p | Moyen 1600×900 |
  |---|---|---|---|
  | `walk_ship` | 31,8 ms (31 i/s) / 144 | 18,2 / 92 | 22,8 / 81 |
  | `walk_ship` sans lightmaps | 38,6 (26 i/s) / 130 | 19,9 / 117 | 27,0 / 80 |
  | `fly_ship` (marche en vol) | 36,2 (28 i/s) / 77 | 20,4 / 45 | 25,2 / 59 |
  | `port_loop` | 30,3 (33 i/s) / 114 | 24,7 / 122 | 21,3 / 78 |
  | `raid_win`, combat à 10 coques | 34,5 (29 i/s) / 73 | 19,9 / 52 | 24,9 / 52 |
  | `firefight` (2 abordeurs) | 33,7 (30 i/s) / 69 | 20,9 / 95 | 24,1 / 51 |
  | `firefight_lose` (3) | 30,1 (33 i/s) / 51 | 17,8 / 37 | 21,5 / 43 |

- **Critères de performance de M1, M2 et M6 non tenus** : ≈ 30 i/s en Moyen 1080p, à peine 50–55 i/s en Bas, et des frames > 33 ms en continu. **Le GPU est la limite** : en Bas dans une fenêtre de 640×360, 3,2 ms (`walk_ship`) et 3,9 ms (`fly_ship`), donc le CPU (logique, physique, IA, simulation) coûte ≈ 3–4 ms. Le combat ne pèse presque rien (raid ≈ vol seul). Les lightmaps gagnent 18 % (38,6 → 31,8 ms) ; en 1600×900, `walk_ship` passe de 25,0 ms (M1, avant lightmaps) à 22,8 ms. Pics de 70 à 145 ms aux transitions de cellules et aux arrivées. Prochaine étape, en tête de M7 : mesurer le temps GPU de chaque passe (ombres, bloom, HDR, anticrénelage, ciel) avant de toucher aux préréglages.

## 04/11/2026 — Poste Linux (CachyOS) : Q13 et M6, le combat

- **Q13 intégrée** (AZERTY) : les aides citent des jetons de touche (`[KeyE]`) remplacés par l'étiquette de la disposition (`gameplay/keyboard.ron`), corrigée à chaque appui par le vrai caractère. Liaisons physiques inchangées jusqu'au remappage (M7) : la carte est sur « , » en AZERTY.
- **Modèle** (`sh_sim::combat`, P26) : coques en quatre sections + cœur, blindage et pénétration, sections ouvertes, réglages d'impact en données ; dégâts corporels ; raid de la Ligue tiré de la simulation (terrains de chasse, état de la flotte), rescousse de la Flotte de défense (FDU, division de l'UGL) ; `FleetLosses` rend les pertes à la simulation ; coque persistante dans la fiche, réparation payée.
- **Espace** (`combat::space`) : IA de pilotes (attaque avec avance de tir, dégagement, fuite), obus traçants, missiles guidés, torpilles, leurres, impacts par section, explosions ; secousse, alarme, brèches et moteurs touchés à bord ; verrou de croisière à 8 km d'un hostile ; corvettes générées dans Blender. Scénarios : `raid_win` (gagné en ≈ 2 min, 10 coques, puis dégâts gardés jusqu'au chantier d'Hexis), `raid_flee` (≈ 70 s), `raid_lose` (écran de mort, partie effacée). Équilibrage revu deux fois : la première salve de missiles tuait le cargo en 8 s.
- **À pied** (`combat::foot`) : balles réelles arrêtées par murs et portes, recul, chargeurs, gilet ; PNJ calmes → alerte (vue, ouïe, voisins) → combat, couverture trouvée par rayons, chemins par les portails (ils ouvrent les portes), rafales accroupi / debout, changement d'abri pris à revers. Abordage de la soute (`SH_BOARDING`) : repoussé à 2 (le pistolet lourd du capitaine tue en deux balles), perdu à 3 ; le seuil est net. Réparation partielle au prorata quand le compte ne suffit pas (un raid peut coûter plus que ce qu'a un cargo endetté). Captures `target/shots/m6-*.png`.
- **À mesurer sous Windows** : temps de frame pendant `raid_win` (critère « combat à 10 vaisseaux ») et les abordages ; ici (RX 9070 XT) 9 à 17 ms de moyenne pendant le raid (souvent calé sur la synchro à 60 Hz), 0 frame > 33 ms hors capture. Limites : l'abordage n'est déclenché que par l'autopilote (pas encore par le raid en jeu) ; PNJ sans squelette (accroupi et chute simulés sur le modèle).

## 03/11/2026 — Poste Linux (CachyOS) : M5, le cœur RPG

- **Modèle pur** (`sh_sim::career`, P23, P24, testé sur les vraies données) : compétences par l'usage, portefeuille crédit/Wuro au taux d'Hexis, soute, réputations (une division rejaillit pour moitié sur Hexa-Corp), grades, contrats CT/CR/CRS (offres déterministes, ravitaillements issus des pénuries de la simulation), trahison toujours détectée (indemnité ×2, exclusion définitive, « Traître notoire »), Banque (échéances, pénalité, saisie après trois retards), mémoire des PNJ. `SimCommand` : livraisons, achats et trahisons changent les stocks.
- **Jeu** : dialogues en données (`sh_data::dialogue`, P25 : guichetier d'Hexa-Corp, tavernier, accueil selon la mémoire et la réputation), terminal modal (dialogue, contrats, marché, fiche `C`, journal `J`), livraison en orbite par la touche d'amarrage près d'un hub ou de chantiers, épave de CRS à accrocher.
- **Interface en français** : réponse de l'auteur à Q11, intégrée avec retard (elle était dans l'arbre de travail au début de la session ; Q12 aussi, déjà conforme). `fr.ron` + `en.ron` aux mêmes clés, nombres « 12 500,5 ». Nouvelle question Q13 (clavier AZERTY / QWERTY).
- Scénario `contract_loop` (check.sh / check.ps1) : contrat Hexis → Cygnara payé (+6 Logistique, +3 Hexa-Corp), échéance payée, remorquage livré, trahison et dossier clos. Captures : `target/shots/m5-{bureau,contrats,journal,epave,trahison,porte-close,fiche}.png`.
- Limites : le scénario ouvre les consoles par raccourci (l'interaction à pied passe par le même message, non rejouée) ; pas de missions hors Hexa-Corp ni de PNJ incarnés (M6–M7) ; grades seulement chez Hexa-Corp en pratique. **À vérifier sous Windows** : `check.ps1` (nouvelle étape) et les mesures de M1/M2 toujours en attente.

## 03/11/2026 — Poste Linux (CachyOS) : M4, le monde vivant

- **Simulation** (`sh_sim::world`, P22, rails dans `canon/rails.ron`) : flottes du canon à leurs positions de 425at (patrouilles de l'Aube et de la 3e Flotte, errance de la Flotte Beta, réparations des flottes diminuées), économies et pénuries, convois (Cygnara nourrie par Ambroise), raids de la Ligue qui **épargnent l'Empire** (🔒, effet visible seulement), « Grande Faucheuse » sur les convois grothul, guerre Tosear–Ligue, révoltes de Primaterre et d'Ostrag, projets (L'Enclume accélère ; la Flotte d'Attaque reste secrète), Arène et campagne de l'UGL. Aucun événement pour l'Essaim, un vaisseau capitale ou une désassimilation : impossibles par construction.
- **Tests** : déterminisme, deux ans en ≈ 0,01 s, aucun rail violé sur cinq graines, aucun secret public, plausibilité ; journal lisible `target/histoire_2ans.txt` (et `cargo run -p sh_sim --example histoire`). Piège corrigé : `fork_named` repart de la graine, d'où un sous-flux par domaine **et par jour**.
- **Jeu** (`sh_game::living`) : la simulation suit l'horloge (et rattrape un voyage d'un coup), fil des nouvelles à l'écran, console des communications, terminal du marché du spatioport aux prix simulés, connaissance du joueur datée et flottes connues sur la carte. `SH_DAYS=N` pour les captures : `target/shots/m4-{nouvelles,carte-flottes}.png`.
- Q12 ouverte (issues politiques : la simulation ne tranche rien qui touche un personnage nommé).

## 03/11/2026 — Poste Linux (CachyOS) : M3, carte galactique et voyages

- **Planification** (`sh_sim::travel`, testée sur les vraies données) : Portails par le graphe des hubs (P19 : Hexis seul lien entre les blocs), hyperespace en ligne droite contournée autour des nébuleuses bloquantes, durées (P3), carburant et péage (P20) ; aucun plan vers un système secret.
- **Horloge** (`sh_game::clock`) : jour 1 de 425at, ×30 en vues Globale et Sectorielle, « Day N, 425at » à l'écran.
- **Carte** (`sh_data::public_map`, test `la_carte_ne_montre_aucun_secret`) : survol avec infobulle de champs publics, clic pour choisir, panneau d'itinéraire, `1`/`2` pour partir, `M` pour revenir aux commandes (pose reprise). Carte de chargement au départ (destination, durée, date d'arrivée, astuce).
- **Cygnara** en vue Globale, brut (P21). Scénario `voyage` : Hexis → Cygnara en 4,0 h par Portail, retour en 23,9 jours par hyperespace. Captures : `target/shots/m3-{carte,voyage-450,voyage-600,cygnara}.png`.
- Limites : paires fixes de Portails non modélisées, pas de brouillard d'information (M4), étiquettes de la carte qui se chevauchent par endroits.

## 03/11/2026 — Poste Linux (CachyOS) : fin de M2, pilotage et amarrage

- **À bord en vue Globale** : le secteur du vaisseau est co-chargé, sa racine est le référentiel de l'origine flottante (position **et orientation** : l'extérieur, le ciel et la lumière tournent autour de l'intérieur immobile). Départ de partie en données (`propositions/starts/hexis_captain.ron`, P1).
- **Pilotage** (`sh_game::ship`) : modèle de vol pur et testé (assisté, découplé, croisière selon la loi P2), siège du pilote → contexte Pilotage, F pour se lever, maniement dans `gameplay/flight.ron`, bandeau de pilotage, hublot avant du cockpit (fenêtres à allège dans le kit).
- **Amarrage** : portails externes (`@dock`, `@ship`) et secteur composé (`sh_data::dock`, testé) ; spatioport d'Hexis brut (P7 : passerelle, hall, bureau des contrats, taverne, marché) ; balise d'approche en orbite basse ; séquence d'approche automatique avec préchargement, fondu bref, reprise aux commandes ; départ symétrique. Ciel et textures préparés en tâche de fond à quai : `port_loop` sans aucune frame > 33 ms (pire 14 ms ici).
- Captures : `target/shots/m2-{cockpit-vol,croisiere,marche-en-vol,quai,hall,approche-spatioport}.png`.
- **À mesurer sous Windows** (UHD 770, release, Moyen) : `fly_ship` et `port_loop`, en plus des mesures de M1. Limites connues : texture d'Hexis floue en orbite très basse (512 px), pas encore de coque extérieure du vaisseau, `player_ship.py` n'utilise pas encore `sector_tools.py`.

## 03/11/2026 — Poste Linux (CachyOS) : M2, vue Globale d'Hexis

- **Origine flottante** (`sh_core::SpacePos`, `sh_game::space`) : positions absolues en f64, `Transform` recalculés depuis les f64 à chaque recentrage ; test : même précision près du centre et à 10⁹ km.
- **Échelle P2 chiffrée** (`gameplay/space.ron`) : tailles ÷ 10, distances ÷ 100, croisière à vitesse proportionnelle à la distance de la surface la plus proche (1 à 2 min entre planètes voisines, testé). Disposition générique des systèmes (P16) et mise en scène d'Hexis (P15).
- **Système d'Hexis** : ciel généré depuis M7 (P17 : bulbe à l'ouest, Croissant au nord), Hexis texturée, L'Enclume, hub de Portails, chantiers, trafic (P18, `tools/blender/space/hexis_orbitals.py`). Captures : `target/shots/m2-hexis-{arrivee,hub,chantiers,cargo}.png`, `m2-ciel-*.png`. Piège : la skybox de Bevy lit la cubemap avec z inversé.
- Q11 complétée (termes techniques traduits dans l'interface anglaise, hypothèse).

## 03/11/2026 — Poste Linux (CachyOS) : fin de M1, éclairage précalculé

- **Lightmaps** : `tools/blender/lightmap.py` déplie un 2e jeu d'UV par cellule (commun aux versions Locale et Sectorielle), cuit l'éclairage diffus dans Cycles (une lampe par `LIGHT_`, rebonds, ombres douces, lumière des écrans) et écrit des PNG sRGB normalisés + leur exposition dans le descripteur du secteur. ≈ 3 min pour le vaisseau à 512 échantillons (CPU i9-9900K). En jeu : `Lightmap` sur les maillages des cellules, lumières sans ombres dynamiques ni diffus sur les surfaces cuites (spéculaire et portes seulement), `SH_BAKED_LIGHTING=0` pour comparer. Captures : `target/shots/m1-lm-{soute,sas,cockpit,couloir}.png`.
- **Collisions** : les `_col` (tous des pavés) deviennent des `Collider::cuboid` au lieu d'enveloppes convexes calculées à l'instanciation (piste des pics de temps de frame).
- **Bug de marches (encore le taux de rafraîchissement)** : les lightmaps ont fait passer le vaisseau de ≈ 250 à ≈ 360 i/s sur ce poste et `walk_ship` rebloquait au pied de l'escalier : avec quelques millimètres d'avancée par frame, la capsule se posait sur l'arête du nez de marche et glissait en arrière. La montée sonde maintenant au moins 13 cm (débord nécessaire pour une normale praticable, × 1,5) ; vérifié en Bas, Moyen, Haut et sans lightmaps.
- **À mesurer sous Windows** (UHD 770, release, Moyen, `walk_ship`) : temps de frame moyen et pire, frames > 33 ms, avec et sans lightmaps ; vérifier aussi que `check.ps1` passe (marches à ≈ 35 i/s). C'est la seule chose qui reste dans M1.

## 03/11/2026 — Poste Linux (CachyOS), en alternance avec Windows

- Second poste : CachyOS, i9-9900K, Radeon RX 9070 XT. Il alterne avec le poste Windows (machine de référence) ; ses mesures de performance ne valent pas pour l'UHD 770 (AGENTS.md §3.1 bis et §10).
- Scripts multiplateformes : `tools/check.sh` (pendant de `check.ps1`), `tools/setup-linux.sh` (paquets, LFS local, sources de Bevy, addon MCP), recherche de Blender multiplateforme dans `tools/fonts/convert_inter.py`, commandes Linux dans les README. Procédure et prompts de `docs/reprise-autre-pc.md` réécrits pour l'alternance des deux postes.
- **Bug trouvé grâce au second poste** : à ≈ 230 i/s, le scénario `walk_ship` bloquait au pied de l'escalier du cockpit. Sur un nez de marche, la normale du contact de la capsule dépend de son débord, donc de la durée de la frame (39° à 35 i/s, 71° à 230 i/s, limite 46°). `player::walkable_contact` lit alors la normale de la face elle-même. `tools/check.sh` vert ; à revérifier sous Windows avec `check.ps1`.

## 03/11/2026 — M1 À pied (vaisseau du joueur)

- Pipeline Blender versionné (`tools/blender/`) : bibliothèque commune, kit d'intérieur paramétrique (P13), vaisseau du joueur en 8 cellules (P12) avec descripteur de secteur généré. Conventions `_col`, `LIGHT_`, `INTERACT_`, `SOCKET_` relues par le jeu.
- Jeu : streaming Locale/Sectorielle (anticipation, hystérésis, budget), portes de chargement, contrôleur cinématique avian3d (`MoveAndSlide`, marches), interactions par rayon, HUD minimal. Scénario `walk_ship` : sas → cockpit, 25 étapes, 12 transitions de cellule, toutes portes franchies.
- Non vérifié automatiquement : saut, accroupi, pentes (pas de rampe dans le vaisseau), s'asseoir. **Non fait** : essai de lightmaps précalculées (reporté, éclairage dynamique + budget d'ombres en attendant).
- **Mesures release (UHD 770, fenêtre 1600×900, scénario `walk_ship`)** : Moyen 28,6 ms (35 i/s), pire 86 ms, 310 frames > 33 ms ; Moyen avec 1 seule lumière ombrée (nouvelle valeur par défaut) 25,0 ms, pire 70 ms ; Bas 13,2 ms (76 i/s), pire 51 ms, 23 frames > 33 ms. **Critères de performance de M1 non atteints** : les ombres de lumières ponctuelles (cubemaps) et le bloom pèsent lourd, et l'instanciation d'une cellule (colliders convexes, scène) crée des pics. Pistes pour la suite : lightmaps précalculées, carte d'ombre ponctuelle réduite, colliders boîtes au lieu d'enveloppes convexes, instanciation étalée sur plusieurs frames.
- Arrêt demandé par l'auteur après M1 (limite d'usage) : M2 non commencé. `.gitignore` (caches Python) et `docs/reprise-autre-pc.md` ajoutés par l'auteur, commités ici.

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
