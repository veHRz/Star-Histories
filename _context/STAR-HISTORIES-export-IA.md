# STAR HISTORIES — Export de contexte pour IA

> **Ce document** résume l'ensemble du compendium (la « bible d'univers ») du jeu space opera **Star Histories**, pour qu'une IA puisse **construire le monde** (géographie galactique, astres, puissances, forces armées, personnages, histoire et état présent) **sans contredire le canon**.
> **Source :** le wiki Obsidian « Star Histories » (≈ 450 fiches canon + 46 pistes non canon) et la carte canonique `01-Galaxie/carte-astrea.html` (validée le 16/07/2026). **Instantané du 03/11/2026.** Le compendium est en cours d'écriture : ce qui n'apparaît pas ici n'est pas encore défini.
> **Fichier compagnon :** `star-histories-carte.json` contient en données structurées les coordonnées de tous les objets de la carte, les contours des zones, les populations, les hubs de Portails et la position de chaque flotte.

## Sommaire

0. Instructions pour l'IA
1. L'univers en bref
2. Règles fondamentales du monde
3. Géographie d'Astréa : carte, zones, systèmes, astres, stations, positions
4. Puissances et factions
5. Espèces
6. Forces militaires en 425at
7. Personnages
8. Histoire
9. État du monde en 425at
10. Récapitulatif des secrets d'univers 🔒
11. Zones blanches : ce qui n'est pas encore défini
12. Incohérences relevées dans le wiki
13. Glossaire et sigles
- Annexe A : registre des vaisseaux nommés
- Annexe B : pistes narratives NON CANON

---

## 0. Instructions pour l'IA (à lire en premier)

1. **Le présent de l'univers est 425at.** Notation des dates : `Xbt` = X ans avant l'an 0, `Xat` = X ans après. L'an 0 est **le Triomphe**, la victoire finale sur l'Essaim.
2. **Hiérarchie des sources.** Les fiches du wiki font foi ; ce document en est un résumé fidèle. Les pistes du dossier `99-Idees/` ne sont **pas canon** (annexe B) et ne doivent jamais être présentées comme des faits.
3. **Aucune invention silencieuse.** C'est la règle n°1 de l'auteur : un détail absent du canon n'existe pas encore. Si la construction du monde exige de combler un vide, rédige une **proposition marquée comme telle** (« proposition, à valider par l'auteur »), cohérente avec ce document. La section 11 liste les vides connus ; la section 11.2 liste les **choix d'auteur à ne jamais contredire**.
4. **Marqueurs** : 🔒 = **secret d'univers** (vrai dans le monde, mais inconnu des personnages, donc par défaut des joueurs) ; ✝ = détruit ou mort ; ❓ = non défini ; « (proposition) » = non validé.
5. **Secrets** : tu dois les connaître pour rester cohérent, mais ne les rends jamais visibles aux personnages ou aux joueurs sans décision explicite de l'auteur. La section 10 les récapitule.
6. **Noms propres** : à conserver exactement, accents, tirets et articles compris (« le Garant », « la Muraille », « Vorrh-III »). La langue universelle du monde s'appelle **le Standard**.
7. **Positions** : toute position canon est une **case de grille** (colonnes A→T, lignes 1→13). Les coordonnées x, y viennent de la carte canonique. **À l'intérieur d'un système, la disposition des corps sur la carte est schématique** (étalée pour la lisibilité) : ce ne sont pas des orbites. L'ordre orbital n'est connu que lorsqu'il est indiqué.
8. **Laisser de la place** : ne fige pas de chiffres ou de dates précises sans nécessité. Les populations sont des ordres de grandeur.
9. **Ton et inspirations déclarées** : space opera inspiré de **Star Wars** et **Star Trek** (Concordia ≈ Coruscant, l'Essaim ≈ les Borgs, les Grothul ≈ les Hutts, le Syndicat Tosear ≈ le syndicat Pyke, les Sœurs de Myrth ≈ les sorcières de Dathomir, les Hashashin ≈ les Sans-Visage de *Game of Thrones*). Combats spatiaux **brutaux et courts**, **sans boucliers énergétiques**. Ces inspirations sont un ton, pas des emprunts : tout est original.

---

## 1. L'univers en bref

L'humanité, partie de **Terra** et unifiée sous les démocratiques **Nations Unies** (600bt), colonise la galaxie d'**Astréa** à partir de 450bt, bâtit le réseau des **Portails** (trajets quasi instantanés) et intègre les espèces rencontrées. En 75bt, elle découvre l'**Essaim**, collectif cybernétique qui assimile ses victimes, et commet l'erreur de l'intégrer. La **Grande Guerre** (53bt → 0at) tue 55 % de la population et détruit Terra. Seul le parti du **Renouveau Impérial** avait vu venir la menace : au **Triomphe** (0at), son protégé le général **Avrel Hex** détruit la Reine de l'Essaim, et son chef **Alarik Vorrhen** fonde l'**Empire Vorrhen** (autoritaire, méritocratique, humano-centré), qui règne 357 ans depuis Concordia.

En 357at, l'**Union Galactique Libre** (UGL), alliance secrète aidée par le poison des **Sœurs de Myrth**, décapite l'Empire (le **Conseil empoisonné**) et lui arrache 90 % de son territoire. Réfugié sur **Vorrhal**, l'Empire renaît sous l'énigmatique **Corvan Sarkis** (la **Renaissance impériale**, 400at → 410at) et reprend près de la moitié du territoire. Depuis 410at règne la **Fausse Paix armée** : deux blocs de taille égale séparés par un **No Man's Land** aux Portails détruits, une **Bordure Extérieure** indocile (royaumes, pirates, crime organisé, ordres interdits) et **Hexa-Corp**, la corporation neutre qui arme tout le monde depuis Hexis.

🔒 Ce que personne ne sait : une **troisième puissance**, **La Fédération** (un seul système, 23 milliards d'habitants), forge en silence la plus grande flotte d'attaque jamais vue ; et l'**Essaim se reconstruit** autour d'une nouvelle Reine dans la **Zone Inexplorée**, à quelques systèmes de Concordia.

**Les quatre modes d'écriture de l'auteur** : le **Wiki** (description du monde, mode actuel), la **Trame Historique** (le passé), la **Trame Actuelle** (les événements de 425at : c'est là que commencent les histoires et le jeu) et les **Trames Personnages** (récits centrés sur un héros).

---

## 2. Règles fondamentales du monde

### 2.1 Calendrier

- On compte en **Années Stellaires**. **An 0 = le Triomphe** (fin de la Grande Guerre) = fondation de l'Empire. `bt` = *before triumph*, `at` = *after triumph*. L'Empire et l'UGL partagent le même an 0.
- Repères : la Grande Guerre a duré 53 ans ; l'Essaim a vécu 22 ans « intégré » à l'Union (75bt → 53bt) ; l'Empire d'avant la Rébellion a duré 357 ans ; en 425at, l'UGL a 68 ans et la Fausse Paix armée dure depuis 15 ans.
- Chronologie détaillée : section 8.

### 2.2 Voyage interstellaire

**Hyperpropulsion** (tous les vaisseaux à partir de la corvette ; elle exige un réacteur nucléaire)
- Lente : **≈ 3 jours** entre systèmes voisins, **≈ 2 mois** pour traverser un bloc (Empire ou UGL), **≈ 6 mois** pour traverser la galaxie.
- Chaque saut demande des calculs de navigation ; les routes cartographiées sont plus rapides et plus sûres que les trajets improvisés.
- **Les nébuleuses et certaines anomalies bloquent l'hyperespace** : il faut les contourner ou les traverser en vitesse normale. Elles créent goulets, refuges et cachettes.
- Chasseurs et bombardiers n'ont pas d'hyperpropulsion : ils dépendent de porte-vaisseaux ou de bases, sauf de rares chasseurs long-courriers hors de prix (prisés des contrebandiers et des as).

**Portails** (réseau construit par les Nations Unies à partir de ≈ 450bt)
- Trajets quasi instantanés. Réseau mixte : des **hubs** multi-destinations dans les grands systèmes, des **paires fixes** ailleurs. Couper une paire isole une branche ; prendre un hub, c'est prendre un carrefour régional.
- Le trafic est régulé par la faction qui tient le système : péages, contrôles, blocus. Tout vaisseau peut franchir un Portail avec l'autorisation de cette faction.
- **Une grande partie du réseau a été détruite pendant la Grande Guerre.** L'Empire a mis longtemps à le réparer et n'a jamais rebranché les systèmes jugés « sans importance » : ce sont aujourd'hui les **systèmes abandonnés**, refuges de pirates, de fugitifs et de secrets.
- Le savoir-faire est répandu (Empire, UGL, Hexa-Corp, royaumes riches), mais le coût est énorme : des années de travaux et une fortune. Un Portail détruit laisse des ruines partiellement réutilisables (rabais modeste).
- **Bordure** : beaucoup de royaumes ont reconstruit des Portails pour commercer ; ils sont contrôlés par la puissance locale (gang, syndicat, royaume, pirates), qui encaisse le péage. Les mondes d'un même royaume sont bien reliés entre eux, mais peu reliés aux royaumes voisins : la Bordure est « un archipel de méfiances ».
- Les militaires préfèrent l'hyperespace pour les attaques surprises, car les Portails sont surveillés. Détruire ou capturer un Portail vaut une victoire de bataille.
- **Le verrou du No Man's Land** : vers 410at, l'Empire et l'UGL ont chacun détruit les Portails qui reliaient leurs territoires. Toute invasion doit traverser par hyperespace, avec un ravitaillement à plusieurs semaines, voire plusieurs mois : c'est le fondement de la Fausse Paix armée. **Seul le hub d'Hexis relie encore les deux blocs.**
- **Kholvar** (La Fédération) : les Portails des Nations Unies y sont toujours debout, mais **quiconque les emprunte est abattu sans sommation**. Le seul accès toléré passe par un Portail voisin de Sabraze, hors du système.
- Conséquence : deux vitesses de civilisation. Les mondes reliés sont riches et connectés ; les autres vivent isolés, à des mois de tout.

### 2.3 Énergie et propulsion

- Deux sources : le **carburant lourd très condensé** (déplacements hors hyperespace et énergie de bord) et les **réacteurs nucléaires** (hyperpropulsion). La quasi-totalité des vaisseaux embarque les deux ; l'autonomie se mesure d'abord aux réserves de carburant lourd.
- **Croiseurs de nouvelle génération** : nucléaire aussi pour les déplacements et l'énergie de bord. **Cuirassés, vaisseaux capitale et grandes stations** : uniquement nucléaires (ils peuvent emporter du carburant lourd pour ravitailler les autres). **Porte-vaisseaux** : comme les croiseurs, pour consacrer un maximum de stockage au carburant des appareils embarqués. Les vaisseaux expérimentaux peuvent avoir leur propre source d'énergie.
- Le **combustible et les déchets nucléaires** ne sont transportés que par des vaisseaux militaires spécialisés : aucun civil n'en transporte légalement. C'est l'une des contrebandes les plus dangereuses et les plus lucratives qui soient (le Syndicat Tosear s'y risque, mais les occasions sont rarissimes).
- Couper le ravitaillement en carburant lourd immobilise une flotte sans détruire un seul vaisseau. Un cuirassé ne tombe jamais en panne sèche, mais il dépend de convois nucléaires militaires rares et très protégés.
- Les épaves de la Grande Guerre contiennent encore leurs réacteurs : les cimetières de flottes sont à la fois des zones contaminées et des mines d'or.

### 2.4 Vaisseaux : classes, armement, conventions, nommage

| Classe | Taille | Équipage | Hyperpropulsion | Rôle |
|---|---|---|---|---|
| Chasseur | 10-20 m | 1-2 | non (en général) | Combat léger, escorte ; dépend d'un porteur |
| Bombardier | 15-30 m | 2-4 | non (en général) | Frappe antinavire |
| Corvette | 50-150 m | 10-80 | oui | Patrouille, escorte, raids |
| Frégate | 200-400 m | 100-400 | oui | Polyvalente, cœur des escadres |
| Croiseur | 500-1 000 m | 1 000+ | oui | Ligne de bataille, commandement régional |
| Porte-vaisseaux (PV) | 800-1 500 m | 3 000+ | oui | Transporte chasseurs et bombardiers |
| Cuirassé | 1 500 m+ | 5 000+ | oui | Suprématie, symbole de puissance, très rare |
| Vaisseau capitale (« vaisseau-mère ») | > 10 000 m | > 1 000 000 | oui | Vaisseau-ville autonome pendant des années. **N'a jamais existé** (seul projet : le *Souverain* de Vorrh-V, abandonné) |

- **Vaisseaux spécialisés** (taille entre frégate et croiseur, coques modifiées) : expérimentaux (armement, moteur ou énergie expérimentale) ; transporteurs à haut risque (seuls autorisés pour le nucléaire, uniquement militaires). L'exemple fondateur, connu de tous : les frégates d'antimatière d'Avrel Hex au Triomphe.
- **Paramilitaires** : navires scientifiques (anciennes frégates ou anciens croiseurs réhabilités, rattachés à la division scientifique de chaque entité) ; transports militaires (anciennes frégates vidées et désarmées : carburant lourd, prisonniers, munitions, troupes (taille d'une corvette, pour le débarquement), matériel, pièces détachées avec réparateurs, transport et remorquage de vaisseaux), qui voyagent au cœur des flottes.
- **Civils** : navettes, cargos (du tramp indépendant au convoi de guilde), paquebots (entre mondes reliés aux Portails), utilitaires (mineurs d'astéroïdes, remorqueurs, vaisseaux-hôpitaux, yachts).
- **Armement** : canons à projectiles (magnétiques), missiles et torpilles. **Aucun bouclier énergétique** : seul le blindage protège. Chaque impact compte, chaque erreur se paie.
- **Conventions des Nations Unies** : les classes moyennes et lourdes (corvette → cuirassé) sont normalisées entre grandes puissances (même système de pilotage et d'armement de base ; nombre de canons, moteurs et tailles varient). Un équipage formé pilote n'importe quelle coque, et Hexa-Corp vend ou loue les mêmes coques des deux côtés de la frontière. Les classes légères (chasseurs, bombardiers) ne sont pas conventionnées : elles diffèrent totalement d'une faction à l'autre. Les factions mineures qui fabriquent leurs propres vaisseaux ont leurs propres normes (ex. : les frégates « de facture pirate »).
- **Nommage** : cuirassés, porte-vaisseaux, croiseurs, expérimentaux et scientifiques portent **toujours un nom** ; frégates, corvettes et chasseurs n'en ont pas, sauf exception historique ou héroïque.

| Préfixe | Signification | Entité |
|---|---|---|
| VIV | Vaisseau Impérial Vorrhen | Empire |
| NSU | Navire de Service de l'Union | UGL |
| FED | — | La Fédération |
| HC | Hexa-Corp | Flottes propres d'Hexa-Corp |
| *(aucun)* | — | Vaisseaux loués sous contrat Hexa-Corp ; pirates de la Ligue et indépendants |
| TOS | — | Syndicat Tosear |
| OXL | faux pavillon pirate | 🔒 Flotte Nordique du Syndicat Tosear |
| GRT | — | Royaume Grothul |
| ECR | — | Écorcheurs ✝ |
| UNS | *United Nations Ship* (vieux Standard) | Nations Unies — **proposition non validée** |

- **Doctrines** : l'**Empire** mise sur la puissance de la ligne (cuirassés, croiseurs lourds, classes uniformes, discipline visuelle totale : « une flotte pensée comme un marteau ») ; l'**UGL** sur la projection (porte-vaisseaux et essaims de chasseurs, armée régulière standardisée, plus les **héros de la Rébellion** qui gardent leurs vaisseaux personnels : « toute aide, tout vaisseau est bon à prendre »). La doctrine fédérale n'est pas formalisée.

### 2.5 Flottes : formations et chaîne de commandement

| Formation | Taille | Composition type |
|---|---|---|
| Escadrille | 2 à 8 vaisseaux | Petite formation spécialisée (ex. : l'EESS) |
| Flottille | 5 à 20 | Corvettes et frégates, éventuellement un croiseur de tête |
| Escadre | 20 à 50 | Frégates et croiseurs, transports militaires intégrés |
| Flotte | 50 et plus | Toutes classes, cuirassé ou porte-vaisseaux au cœur, chaîne logistique complète |

Chasseurs et bombardiers n'entrent pas dans le décompte (ils sont rattachés aux porteurs et aux bases).

- **Chaîne de commandement (Empire comme UGL)** : l'**Amiral** dirige la flotte depuis le vaisseau amiral ; un **sous-amiral** commande ce vaisseau et le remplace au besoin ; un second sous-amiral, sur le vaisseau le plus puissant après l'amiral, peut reprendre la flotte si le vaisseau amiral est détruit ou muet. Chaque cuirassé, porte-vaisseaux et croiseur est commandé par un sous-amiral secondé d'un **contre-amiral**. Spécificités impériales : un cuirassé non amiral est commandé par un Amiral ; les **EDS** par un **Sous-Amiral** ; les **FDM** par un **Contre-Amiral**.
- **Têtes d'armée** : Empire, le **Grand Amiral Ardan Vek** (rend compte à l'empereur) ; UGL, le **Grand Amiral Cassian Drey** (FOU, ne dépend que de la Présidente) et la **Grande Amirale Petra Solen** (FDU, avec les gouverneurs) ; Hexa-Corp, le **Grand Amiral Deylan Mora** ; La Fédération, le **Chancelier** lui-même.

### 2.6 Stations spatiales

- **Toute station a un nom, une fiche et une case de grille.** Globalement stationnaires, elles peuvent se déplacer, mais lentement et à un coût prohibitif. Les grandes stations sont entièrement nucléaires et stockent du carburant lourd pour les vaisseaux de passage.
- Pas de classes : une station se décrit par sa **taille** (petite, moyenne, grande, très grande).
- Tout le monde peut en construire, à condition d'avoir beaucoup d'argent et de temps (aucun secret technologique, contrairement aux Portails).
- Types : **stations-villes** (le mode de vie des royaumes de la Bordure) ; **stations de défense** (tenir un lieu sans immobiliser une flotte) ; **stations de recherche** (autour d'anomalies, ou isolées pour contrôler l'expérience) ; **stations-chantiers** (pas d'atmosphère à quitter, donc pas de coût de carburant au lancement : l'avenir de la construction navale).

### 2.7 Pouvoirs, espèces douées et magie

- **Pouvoirs des espèces** : des facultés **biologiques**, innées et documentées par la science : télépathie des **Ethari**, soins des **Saluviens**, métamorphose des **Vashyr** (détails en section 5). D'autres espèces ont des capacités plus faibles (à créer).
- **La magie** : un phénomène **inexpliqué par la science**, sans lien avec la biologie. Le don est **inné** et rare (certaines lignées, certains peuples), mais il ne suffit pas : il faut être **formé**, sinon le don est aussi dangereux pour son porteur que pour son entourage. Il existe des **voies détournées** (pactes, artefacts, rituels) pour voler un pouvoir non reçu, à un prix terrible.
- **Le prix de la magie : la corruption.** Aucun sacrifice n'est nécessaire, mais plus on pratique, plus on se corrompt (corps et esprit), jusqu'à la mort si l'on perd le contrôle. La formation apprend surtout à survivre à son usage.
- **Plusieurs traditions** existent : la magie noire des Sœurs de Myrth n'en est qu'une ; d'autres ordres discrets, probablement réfugiés dans la Bordure, restent à créer.
- **Statut légal en 425at** : l'**Empire** interdit et chasse la magie (les pratiquants sont des ennemis d'État) ; l'**UGL** la tolère officiellement, sous surveillance discrète. Les ordres restent neutres pour survivre.
- Les **ordres religieux** (Frères de l'Aube, Confrérie des Hashashin) ne relèvent en apparence ni des pouvoirs ni de la magie. 🔒 C'est faux pour les Hashashin : ce sont des Vashyr.

### 2.8 Monnaies et économie

| Monnaie | Émetteur | Époque | Statut en 425at |
|---|---|---|---|
| L'écu | Nations Unies | 600bt → 0at (mort définitive après la Fronde de l'Écu, 110at) | Disparu (numismatique) |
| Le crédit (ex-crédit impérial) | Empire, à sa fondation | 0at → aujourd'hui | **Monnaie universelle de fait** : UGL, Bordure, et encore massivement dans l'Empire |
| Le Wuro | Empire, depuis la Renaissance | ≈ 400at → aujourd'hui | Imposé de force dans l'Empire ; **interdit dans l'UGL** (change au marché noir) |

- Ironie de l'Histoire : à la Rébellion, les imprimeries de crédit sont restées en territoire UGL. L'Empire ressuscité a créé le Wuro pour payer ses soldats et ses fonctionnaires. Dans l'Empire, le crédit reste la monnaie du quotidien ; le Wuro, plus rare, circule dans les cercles de l'État (armée, administration, fournisseurs). Payer en Wuro marque comme impérial ; le refuser dans l'Empire est un délit, mais la répression reste mesurée, sauf envers les **gradés**, censés utiliser la monnaie de l'Empire.
- Le **« taux d'Hexis »** (division Finance d'Hexa-Corp) est la référence officieuse du change crédit/Wuro : personne ne l'admet, tout le monde l'utilise. Toutes les monnaies existent en version physique et virtuelle, mais le **physique reste très utilisé** (conversion, contrebande).
- Économie de l'UGL ruinée par la guerre et en reconstruction ; industrie impériale centralisée sur deux systèmes ; dépendance des deux blocs envers Hexa-Corp ; marchés noirs (Tosear).

### 2.9 Démographie : règle de comptage

- Les populations sont des **ordres de grandeur** qui ne comptent que les **représentants des espèces civilisées**, **par lieu de résidence**, en trois catégories : **civile**, **militaire / forces de l'ordre**, **ouvriers de chantiers** (construction et entretien de vaisseaux et de stations). Une lune se compte à part de sa planète.
- Dans un bagne, « civile » = population carcérale (l'encadrement est militaire). Les **esclaves** sont comptés mais signalés à part. Les astres morts ou détruits ont une population de 0 ; les étoiles n'en ont pas.
- **Humains ≈ 75 %** de la population des mondes connus. Plus grands foyers : Concordia (≈ 176,5 Md), Vorrhal (≈ 76 Md), Tharkast (≈ 18 Md), Primaterre (≈ 15 Md), Moisson (≈ 10 Md), Grotha (≈ 5 Md). La somme des astres et stations déjà décrits atteint ≈ 333 Md, ce qui n'est pas la population de la galaxie : de nombreux mondes restent à créer.

### 2.10 Langue, croyances, mœurs

- **Le Standard**, langue universelle héritée des Nations Unies, est parlé par toutes les espèces civilisées.
- Les **Nations Unies** étaient laïques (une bonne partie de la population était athée ; les sectes jugées dangereuses étaient interdites et leurs chefs exilés aux frontières). L'**UGL** reste laïque. L'**Empire** ne tolère **aucune religion** : on ne glorifie que l'empereur vivant (le culte populaire qui avait divinisé Alarik le Fondateur fut dissous en 280at).
- Les **Frères de l'Aube** sont les derniers héritiers d'une religion terrienne disparue (leur dieu est « Dieu ») ; la **Confrérie des Hashashin** se réclame du **Silencieux** (🔒 une couverture).
- Jeux d'argent : **interdits dans l'Empire**, tolérés mais mal vus dans l'UGL. D'où le succès des casinos de **Grotha** (le premier de la galaxie) et d'**Hexis** (le deuxième).
- Espèces « non civilisées » : les Nations Unies, puis l'UGL, ne les touchent pas (la lune Zénith est un sanctuaire) ; l'Empire les déplace ou les exploite ouvertement là où leurs mondes ont des ressources.

---

## 3. Géographie d'Astréa

### 3.1 Repère de la carte

- **Astréa** est une galaxie spirale à deux bras, vue de dessus. Sur la carte canonique, elle est dessinée comme une **ellipse** de centre (500, 330) et de rayons 310 × 248, dans un repère de **1000 × 660 unités**.
- **Grille de repérage** : **20 colonnes A → T** et **13 lignes 1 → 13**, cases de 50 × 50 unités. Colonne = lettre(⌊x / 50⌋) avec A = 0 ; ligne = ⌊y / 50⌋ + 1. La galaxie occupe à peu près les colonnes **D → Q** et les lignes **2 → 12**.
- **Orientation** : nord = haut (y petit), sud = bas, est = droite (x grand), ouest = gauche.
- **Le noyau galactique** est un **trou noir central tout à fait ordinaire**, en (500, 330), à la jonction des cases J7 et K7. On évite simplement de s'en approcher ; Concordia orbite dans les régions proches, à distance sûre.
- **Échelle indicative (déduction NON CANON, pour la cohérence seulement)** : la galaxie mesure ≈ 620 unités d'est en ouest pour ≈ 6 mois d'hyperespace ; un bloc (≈ 170 unités) se traverse en ≈ 2 mois. Donc **1 case ≈ 2 semaines de saut** et **≈ 10 unités ≈ 3 jours** (systèmes voisins).
- La carte place **10 systèmes** dessinés en contour pointillé (plus le système Solaire, marqué par Terra ✝), **14 astres isolés** et **3 stations isolées**. L'auteur prévoit de faire grossir l'univers d'environ **2 à 3 fois** (la « petite centaine » de systèmes initialement envisagée n'était qu'un ordre de grandeur) : les nouveaux systèmes se placeront sur la carte au fil de leur création.

### 3.2 Les grandes zones

| Zone | Emprise (cases) | Contrôle | Caractère |
|---|---|---|---|
| **Zone Inexplorée** | Toute la moitié ouest (≈ **40 %** de la galaxie), à l'ouest de x ≈ 450 : colonnes **D → I** | Personne. 🔒 L'Essaim s'y reconstruit | Jamais explorée (jugée sans intérêt par les Nations Unies). **Aucune barrière physique** : seulement l'absence de routes, de Portails et de raisons d'y aller. Son bord passe juste à l'ouest du noyau, à quelques systèmes de Concordia. Silence total depuis 425 ans. 🔒 Contient la colonie secrète de Vesper (I10) ; l'EESS impériale explore son nord, le NSU Méridien la cartographie |
| **Territoire de l'UGL** | ≈ **J4 → M10**, contour irrégulier | UGL | Centre-est, adossé au noyau et à la Zone Inexplorée ; même taille que l'Empire ; légèrement étendu à l'est de Cygnara pour englober Le Berceau |
| **No Man's Land** (depuis ≈ 410at) | ≈ **L5 → N10**, bande fine et courbe | Personne | Zone tampon aux Portails détruits, traversée seulement par pirates, marchands neutres et contrebande Tosear. Contient Hexis (M7), Cendres (M5), la nébuleuse du Croissant et Rade-Noire (M6) |
| **Territoire de l'Empire** | ≈ **M4 → Q10** | Empire Vorrhen | Masse orientale de taille égale à l'UGL ; bordure étendue au sud-est pour inclure le système Primaterre (P9). Coincé entre le front (ouest) et le vide galactique (est) : sa seule direction d'expansion est l'UGL |
| **Marches nord** (Bordure Extérieure) | Lignes ≈ 2 → 4 | Royaumes et indépendants | Royaume Grothul (L3 → N3, avancée en N4), Myrth et Franchise (K4) ; le « nord de la Bordure » (territoire de Raeilla la Guerrière et de la Flotte Nordique). Volontairement vides sur la carte : réservées aux futurs royaumes |
| **Marches sud** (Bordure Extérieure) | Lignes ≈ 10 → 12 | Royaumes et indépendants | La Fédération (J11), Vashyra (L11), Rocaille (N11), Terminus (P10). Volontairement vides : réservées aux futurs royaumes |
| **Systèmes abandonnés** | Dispersés dans tout l'ancien territoire | Personne | Jamais rebranchés aux Portails après la Grande Guerre : dépeuplés, refuges de pirates, de fugitifs et de secrets (aucun encore nommé) |

**La Bordure Extérieure** désigne les confins civilisés hors des deux blocs : royaumes et confédérations nés des survivants de la Grande Guerre et des exilés (opposants chassés par l'Empire, sectes bannies par les Nations Unies). Ils doivent leur existence à la destruction des Portails, qui les mettait à des mois de toute puissance. La plupart sont **neutres**, et se savent des proies depuis l'annexion de Primaterre.

### 3.3 Systèmes stellaires et leurs astres

*Population : civile / militaire et forces de l'ordre / ouvriers de chantiers. L'ordre des corps dans les tableaux n'est un ordre orbital que lorsque c'est précisé.*

#### Système de Concordia — K7 — UGL (système-capitale)
Un système « sans arrière-cour » : une seule planète, trois lunes, aucune ceinture. Force symbolique immense, dépendance totale (ni nourriture ni matières premières produites sur place). À quelques systèmes de la Zone Inexplorée.

| Corps | Type | Rôle | Population |
|---|---|---|---|
| **Hélion** | Étoile exceptionnellement puissante | Ses collecteurs orbitaux fournissent **toute l'énergie** de Concordia : une cible stratégique évidente (infrastructure ❓) | — |
| **Concordia** | Ville-planète (écoumenopole couvrant 100 % de la surface), diamètre **8 × Terra** | Capitale de l'UGL depuis 357at (capitale impériale de 0at à 357at) ; plus grande ville de l'Histoire ; monde natal des **Andari**, métissés avec les colons ; hub de Portails le plus fréquenté de la galaxie ; siège de la Présidence, du Sénat des Mondes et du Conseil des Espèces ; berceau de la conspiration de 347at | 175 Md / 1 Md / 500 M |
| ↳ **Aube** (lune n° 1) | Lune rocheuse | Petite base, rien d'autre | 50 K / 150 K / 0 |
| ↳ **Zénith** (lune n° 2) | Lune vivante | Sanctuaire d'espèces non civilisées jamais dérangées | 0 |
| ↳ **Crépuscule** (lune n° 3) | Lune rocheuse ferreuse | Mines de fer du système | 5 M / 100 K / 0 |

#### Système Vorrh — P7 — Empire (système-capitale)
Forteresse autosuffisante à l'extrémité orientale de l'Empire, le point de l'espace civilisé le plus éloigné du front et de la Zone Inexplorée. Seule la capitale porte un vrai nom : les autres mondes sont des « mondes-outils » à numéros militaires. Système le mieux défendu de l'Empire (EDS, FDM, port d'attache des 1ère et 2ème Flottes). Moins peuplé que Concordia : ses planètes sont plus petites.

| Corps | Type | Rôle | Population |
|---|---|---|---|
| **Vorrh** | Géante de classe K, orangée | Astre-symbole de la propagande (« tout ce qui vit ici vit sous Vorrh ») | — |
| **Vorrh-III** | Planète de feu (croûte en fusion, tempêtes de cendres), **la plus proche de l'étoile** | Seule planète inhabitée du système | 0 |
| **Vorrhal** | Ville-planète | Capitale depuis 357at ; empereur et sénat ; planète natale d'Alarik Vorrhen ; ex-monde agricole ravagé par l'Essaim, entièrement rasé et rebâti (aucune trace de l'occupation) ; grand hub de Portails | 75 Md / 1 Md / 300 M |
| **Vorrh-II** | Monde rocheux et plat | **Seul chantier à cuirassés de l'Empire** (et il ne construit que ça) | 500 M / 150 M / 2 Md |
| **Vorrh-IV** | Planète-forêt | Bois des chantiers ; garnison | 100 M / 5 M / 0 |
| **Vorrh-V** | Planète-chantier géante en sommeil | Chantier du vaisseau capitale *Souverain* : lancé en 309at, arrêté par la Rébellion, relancé en 380at, **stoppé par Sarkis en 400at** (les fonds sont allés aux croiseurs). Infrastructures titanesques inachevées, garnison contre le vol. L'UGL le sait et le juge non prioritaire | 100 K / 500 K / 0 |
| **Vorrh-VI** | Planète minière | Minerais rares ; **Académie Militaire de Vorrh**, principale académie impériale depuis 357at | 250 M / 7 M / 500 K |
| **La Ceinture** | Ceinture d'astéroïdes | Extrêmement riche en minerais ; flottilles minières militarisées | 500 K / 500 K / 0 |

#### Système Ferrok — O7 — Empire (2e système industriel)
Bâti à marche forcée pendant la Renaissance : l'Empire centralise toute sa production navale sur Vorrh et Ferrok. Les chantiers n'atteignent 100 % de leur capacité que depuis quelques années. La 2ème Flotte y est stationnée le temps de sa reconstruction.

| Corps | Type | Rôle | Population |
|---|---|---|---|
| **Ferrok** | Sous-géante de classe G, jaune | Le « soleil d'usine » | — |
| **Ferrogane** | Planète-chantier | Croiseurs lourds et porte-vaisseaux ; théâtre de la bataille de Ferrogane (409at) | 500 M / 150 M / 1,25 Md |
| **Karvek** | Planète-chantier | Frégates, corvettes (l'essentiel de celles de l'Empire), transports | 400 M / 125 M / 750 M |
| Ceinture de Ferrok | Ceinture d'astéroïdes (sans nom propre) | Ressources de base | — |
| Autres corps | ❓ | Non définis | — |

#### Système Ostrag — O5 — Empire (le bagne)
LA prison de l'Empire, sous une naine rouge : un complexe pénitentiaire minier sans Portail public, à des mois d'hyperespace de tout, gardé par une flotte permanente et des stations de surveillance. On y enferme les fidèles et récalcitrants des mondes conquis ou annexés (loyalistes de Primaterre, vétérans Golden Sword) et les prisonniers de guerre de l'UGL. Leur minerai nourrit les chantiers de Ferrok. Fondé peu après 12at ; « finir à Ostrag » est une expression de tout l'Empire. Une poudrière tenue fermée par l'isolement et la faim.

| Corps | Type | Rôle | Population |
|---|---|---|---|
| **Ostrag** | Naine rouge | « Un soleil terne pour un système sans joie » | — |
| **Ostrag-Majeur** | Monde de garnison | Administration pénitentiaire, quartiers des gardes, spatioport militaire, mouillage de la flotte de garde | 0 / 4,5 M / 1 M |
| **Ostrag-II** | Bagne minier, froid polaire | Fer et titane | 750 K détenus / 1 M / 0 |
| **Ostrag-III** | Bagne minier, tempêtes de poussière | Carrières à ciel ouvert | 1 M détenus / 1 M / 0 |
| **Ostrag-IV** | Bagne minier, chaleur étouffante | Galeries profondes : « le pire des trois » | 500 K détenus / 1 M / 0 |
| **Morne** | Planète-cimetière | On n'y creuse que des tombes ; le seul monde « baptisé » du système | 0 / 50 K / 0 |

#### Système Primaterre — P9 — Empire (annexé)
Première colonie des Nations Unies dans la Bordure (420bt), royaume indépendant dès 390bt, l'un des plus riches de la Bordure, **annexé de force pendant la Renaissance** et intégré à la bordure impériale. Occupation brutale (voir 4.6).

| Corps | Type | Rôle | Population |
|---|---|---|---|
| **Ostre** | Étoile | Alimente les champs solaires de Solferra | — |
| **Solferra** | Planète désertique, **la plus proche d'Ostre** | Champs de panneaux solaires (énergie quasi illimitée) ; **QG de l'Escadre de Défense de Primaterre** depuis les sabotages | 0 / 1,5 M / 750 K |
| **Primaterre** | Planète-capitale agricole et riche, occupée | La majorité de la population du système ; grand hub de Portails ; révoltes réprimées dans le sang | 15 Md / 25 M / 3 M |
| **Gravanne** | Planète minérale | Mines qui tournent désormais pour l'Empire | 1 Md / 10 M / 0 |

#### Système Cygnara — L8 — UGL (le poumon industriel)
Cinq planètes qui lui donnent une large indépendance en ressources industrielles, mais **rien à manger** : la nourriture arrive par convois (notamment d'Ambroise). Couper ces convois affamerait l'industrie de l'Union sans tirer un missile. Voisin des ruines de Terra, dont il est le gardien officieux. Gardé par la Flotte de la Libération et la Flotte de défense de Cygnara. Son étoile n'est pas nommée (❓).

| Corps | Type | Rôle | Population |
|---|---|---|---|
| **Cygnara** | Planète-chantier | Croiseurs, frégates et surtout transports pour la reconstruction ; grand hub de Portails ; monde d'origine des **Saluviens** ; siège de la **Guilde des Minerais de l'Union** | 1,5 Md / 100 M / 1 Md |
| **Aldène** | Planète-forêt mortelle | Taux d'oxygène très élevé, végétation démesurée, **plantes carnivores** dévorant des animaux cinq fois plus grands qu'un humain, créatures et **espèces intelligentes indigènes** qui chassent tout ce qui bouge. Riche en **terres rares** : extraction sous forte escorte | 3 M / 7 M / 125 K |
| **Bruma** | Géante gazeuse | Carburant lourd | 150 K / 100 K / 0 |
| **Ferral** | Monde minier | Minerai des chantiers | 500 M / 1 M / 0 |
| **Osca** | Monde glacé | Eau et gaz rares | 1 M / 3 M / 100 K |
| **Le Berceau** | Station-chantier **en construction** | Construira des croiseurs en espace libre | 0 / 125 K / 100-150 K |

#### Système Solaire — L8 — espace UGL (détruit)
Le berceau de l'humanité, et son tombeau : l'Essaim a anéanti **Terra et toutes les planètes du système** (≈ 50bt) pour briser le moral humain. Il reste **le Soleil**, intact, et des **champs de débris**, devenus le lieu de **pèlerinage** le plus sacré de l'humanité, et un voyage dangereux. Les impériaux qui veulent s'y recueillir doivent passer la frontière.

| Corps | Type | Rôle | Population |
|---|---|---|---|
| **Le Soleil** | Étoile | Intacte : « elle éclaire un cimetière » | — |
| **Terra** ✝ | Monde-mère de l'humanité, ancienne capitale des Nations Unies | Détruite ; l'humanité est depuis une espèce sans monde-mère | 0 |

#### Système Ambroise — K9 — UGL (le grenier)
Deux soleils, presque pas de nuit : des saisons de croissance quasi continues. Militairement insignifiant, stratégiquement vital. Raids pirates répétés ; la défense dépend de la flotte de Cygnara en attendant la station Rempart. Culture de fermiers, de convoyeurs et d'une milice agricole fière.

| Corps | Type | Rôle | Population |
|---|---|---|---|
| **Ambre et Braise** | Étoiles jumelles (Ambre dorée et douce, Braise rouge et ardente) | Doubles levers, deux ombres, calendriers agricoles hors norme | — |
| **Moisson** | Planète-capitale agricole | Plaines céréalières, greniers-silos continentaux ; départ des grands convois alimentaires | 10 Md / 5 M / 0 |
| **Fenaison** | Monde agricole | Élevage, cultures vivrières | 2,5 Md / 1 M / 0 |
| **Brumaire** | Géante gazeuse | Carburant lourd des convois | 150 K / 100 K / 0 |
| **Rempart** | Station de défense **en construction** | Défendre le grenier sans mobiliser la flotte de Cygnara | 0 / 125 K / 100-150 K |

#### Système d'Hexis — M7 — Hexa-Corp (neutre), au centre du No Man's Land
Le système le plus espionné de la galaxie. Étoile non nommée (❓).

| Corps | Type | Rôle | Population |
|---|---|---|---|
| **Hexis** | Monde-usine et place forte | Ancien monde industriel ravagé, offert par l'Empire à Avrel Hex ; siège d'Hexa-Corp ; chantiers orbitaux ; Banque, Casino, Coffre, tribunal d'Hexis ; **seul hub de Portails reliant l'UGL et l'Empire** ; terrain neutre où impériaux et unionistes se croisent sans se tirer dessus. Être citoyen d'Hexis, c'est être employé d'Hexa-Corp (logement, éducation, santé fournis ; perdre son emploi, c'est perdre sa citoyenneté) | 1,5 Md / 400 M / 1,5 Md |
| **L'Enclume** | Station-chantier de **cuirassés**, en construction (elle accélère) | Mettra fin au monopole des cuirassés et vendra à quiconque paiera | 0 / 125 K / 100-150 K |

#### Système Vesper 🔒 — I10 — UGL (colonie secrète en pleine Zone Inexplorée)
Découvert par la FSS et colonisé en **405at** : un **secret absolu** pour l'Empire. Autarcie totale (aucun convoi révélateur), gouvernement spécial, FSS en garde. Étoile non nommée (❓).

| Corps | Type | Rôle | Population |
|---|---|---|---|
| **Vesper** | Planète-colonie | Riches gisements de **ressources nucléaires**, la clé de l'hyperpropulsion ; base de la FSS. Des frégates déguisées en transporteurs rapportent discrètement les ressources vers l'UGL | 125 K / 500 K / 150 K |
| **Verdine** | Planète agricole | Fournit tous les besoins primaires de la colonie | 500 K / 500 K / 0 |

#### Système de Kholvar — J11 — La Fédération (marches sud)
Le système le plus fermé de la galaxie : **6 planètes et 6 lunes**, **≈ 23,3 Md d'habitants** (≈ 21 Md civils, ≈ 1,3 Md militaires, ≈ 1 Md d'ouvriers de chantiers). Une économie en circuit fermé : métal d'Ordrakh, minerais de Molvrek, énergie d'Ignar, bois de Draskal, gaz d'Ulveth et nourriture de Grimmar convergent vers Hargan (qui construit) et Tharkast (qui consomme). Deux corps échappent au drapeau fédéral : Vrasq (indéfendable) et Sabraze (laissée libre par calcul). Proche du bord de la Zone Inexplorée ; « voisin » d'Espérance.

| Corps | Type | Rôle | Population |
|---|---|---|---|
| **Kholvar** | Géante jaune-blanc | Lumière dure et blanche (« nous vivons sous une étoile qui ne pardonne pas ») | — |
| **Vrasq** (1re planète) | Géante gazeuse à ≈ 500 °C | Hors Fédération ; rien n'y vit | 0 |
| **Sabraze** (2e planète) | Rocheuse désertique (50-60 °C), ancien relais minier épuisé | **Hors Fédération : le « sas »**. On y commerce et on y négocie avec la Fédération sans jamais entrer ; sans loi (un « syndic » élu par personne tient le seul puits et la seule piste) ; poste d'écoute mutuel ; accès par un Portail voisin, hors du système | 25 K / 0 / 0 |
| **Ordrakh** | Planète métallique | La fonderie | 200 M / 3 M / 500 K |
| ↳ **Tesk** | Lune rocheuse stérile | Inhabitée, sans usage (« même nous, nous n'en avons rien fait ») | 0 |
| **Grimmar** | Planète rocheuse très arable | **Le grenier** : nourrit tout le système | 2 Md / 200 M / 1 M |
| ↳ **Draskal** (lune n° 1) | Lune forestière | Bois | 175 M / 5 M / 0 |
| ↳ **Ignar** (lune n° 2) | Lune volcanique | Énergie géothermique du système | 100 M / 50 M / 0 |
| ↳ **Vaskir** (lune n° 3) | Lune-bastion | Mouillage de la Flotte de Défense ; 750 M de militaires | 250 M / 750 M / 5 M |
| **Tharkast** | Planète naine devenue planète-ville, bâtie vers le bas (des dizaines de niveaux superposés) | **Capitale** : le Chancelier, le Parti, l'**Arène** ; 3e monde le plus peuplé de la galaxie ; ne produit rien de ce qu'elle mange | 18 Md / 150 M / 3 M |
| ↳ **Molvrek** (lune n° 1) | Lune minière | Minerais de tout le système | 175 M / 25 M / 500 K |
| ↳ **Hargan** (lune n° 2) | **Lune-chantier** | Toutes les classes, du chasseur au **cuirassé** (l'un des trois sites de la galaxie qui en sont capables) ; 🔒 cadence jamais mesurée de l'extérieur ; berceau de la Flotte d'Attaque | 150 M / 100 M / 1 Md |
| **Ulveth** (planète extérieure) | Géante gazeuse | Plateformes d'extraction : carburants, gaz industriels | 3 M / 1 M / 1 M |

*Ordre orbital connu : Vrasq 1re, Sabraze 2e, Ulveth la plus extérieure ; l'ordre d'Ordrakh, Grimmar et Tharkast n'est pas précisé.*

### 3.4 Astres hors système (planètes isolées, errantes ou solitaires)

| Astre | Case | Type | Affiliation | Rôle et particularités | Population |
|---|---|---|---|---|---|
| **Calderis** | L7 | Planète **errante** volcanique (sans étoile), planète-chantier | UGL | **Chantier principal**, seul de l'UGL à construire des **cuirassés** ; cales assez vastes pour un futur vaisseau-mère ; chantier hérité des Nations Unies ; chaleur des caldeiras ; protégé par son obscurité (introuvable sans coordonnées). Cible stratégique n° 1 de l'Union | 1,5 Md / 100 M / 2 Md |
| **Port-Vaillant** | K5 | Planète **errante** (orpheline), planète-chantier | UGL | Croiseurs et porte-vaisseaux ; énergie nucléaire ; chantier depuis les Nations Unies ; berceau du gang des Écorcheurs ; deux porte-vaisseaux y ont été volés en construction (par les Écorcheurs vers 385at, par Raeilla pendant la guerre UGL-Empire) | 500 M / 50 M / 1 Md |
| **Alcyone** | L6 | Planète-chantier « solitaire à deux satellites » (étoile éventuelle ❓) | UGL | Frégates et transports ; relativement autonome grâce à ses lunes | 5 M / 15 M / 0 |
| ↳ **Maïa** (lune n° 1) | L6 | Lune-cité | UGL | Lieu de vie des ouvriers d'Alcyone | 45 M / 10 M / 100 M |
| ↳ **Mérope** (lune n° 2) | L6 | Lune-grenier | UGL | Nourrit Alcyone | 20 M / 10 M / 0 |
| **Bastia** | K8 | Planète-chantier solitaire, sans terre arable | UGL | Frégates et transports ; dépend entièrement des convois ; pas de flotte décentralisée propre | 50 M / 25 M / 100 M |
| **Karza** | N6 | Planète solitaire **sans étoile** (errante), base militaire | Empire | Base de la **3ème Flotte** ; grand hub de Portails, « porte d'entrée de l'Empire » ; frontière ouest ; installations enterrées et dômes, énergie nucléaire | 500 M / 500 M / 1,5 M |
| **Myrth** | K4 | Planète-sanctuaire | Sœurs de Myrth (exclusivement) | Marches nord, côté UGL ; **aucun lien avec la magie** (un refuge choisi pour son isolement) ; protégée en sous-main par l'UGL ; on n'y atterrit pas sans invitation ; esthétique sombre (❓) | ≈ 500 (🔒 ≈ 100 Sœurs + ≈ 400 apprenties) |
| **Vashyra** | L11 | Planète isolée | Vashyr (indépendante, isolationniste) | Marches sud ; refuse tout contact prolongé. 🔒 Siège des Hashashin, enrichi par les « offrandes » réduites en esclavage ; clans en guerre permanente | 25 M + 25 M esclaves / 10 M / 0 |
| **Rade-Noire** | M6 (coin est) | Planète, base pirate | Ligue des Pirates | Au cœur de la nébuleuse du Croissant ; cales de radoub, marchés de butin et de pièces (Brakk), tavernes ; cour du Roi Torg Varga ; garnisons au sol des Mercenaires de Cendres ; petite communauté de Frères de l'Aube protégée par Arven le Doux. Jamais attaquée officiellement | 5 M / 15 M / 1 M |
| **Cendres** (ex-Promesse) | M5 | Planète ravagée, monde-refuge | Aucune (revendiquée par personne) | Ancien monde-vitrine de l'Empire, rasé par la 1ère Flotte pendant la Renaissance ; plus grand monde-refuge de la galaxie, sans foi ni loi (seigneurs de camps, milices, troc et dette) ; recrutement pirate, commerce Tosear et jaskari ; berceau des Mercenaires de Cendres. Chaque puissance accuse l'autre de l'avoir créé | 1,5 Md / 50 M / 2 M |
| **Espérance** ✝ | J9 | Planète détruite (ancienne colonie NU) | Aucune (ruines interdites) | Zone explorée, contre la frontière sud-ouest de l'inconnu. Monde d'origine des Tosear ; découverte de l'Essaim (75bt) ; trône de la Reine ; anéantie au Triomphe (0at) | 0 |
| **Rocaille** | N11 | Désert de canyons et de mesas, abandonné | Aucune | Capteurs brouillés, appui orbital impossible ; ruines du **refuge du peuple de la Fédération** pendant la Grande Guerre ; capture de Jaro Fesk (401at) ; légende d'une **cache d'armes des Golden Sword** (vérité 🔒 non tranchée) | 0 |
| **Grotha** | M3 | Planète-capitale | Royaume Grothul | Monde natal des Grothul ; cour-caravansérail où le palais est un marché et le marché un tribunal ; **premier casino de la galaxie** ; douane de façade (on inspecte ce qui entre, jamais ce qui sort) ; toucher à Grotha, c'est déclarer la guerre à tous les Grothul | 5 Md / 15 M / 0 |
| **Vantra** | L3 | Monde-entrepôt, port franc | Royaume Grothul | Hub de Portails de tout le royaume ; front commercial Grothul/Tosear (entrepôts incendiés, cargaisons détournées) ; gardé par les Gardiens de Vantra (Hexa-Corp) | 1 Md / 150 M / 25 M |
| **Skarn** | N4 | Monde minier fortifié | Royaume Grothul | Face à la frontière nord de l'Empire ; plus grosse concentration de mercenaires du royaume (commandements tournants) ; « mur de Skarn » (flotte louée à Hexa-Corp) ; poste d'écoute ; jamais attaquée depuis | 100 M / 250 M / 5 M |
| **Jask** ✝ | ❓ | Monde d'origine des Jaskari (nom à l'état de proposition) | — | Détruit pendant la Grande Guerre par une flotte des Nations Unies, alors qu'il était devenu un bastion de l'Essaim. Position non définie | 0 |

### 3.5 Stations spatiales

| Station | Position | Type | Affiliation | État | Population | Particularités |
|---|---|---|---|---|---|---|
| **Terminus** | P10 (coin bas-gauche), près de Primaterre et des limites de l'Empire | Station-ville | Syndicat Tosear (base principale) | Active ; ex-base militaire avancée des NU (125bt) | 250 M / 10 M / 500 K | Abandonnée pendant la Grande Guerre, devenue station-ville des Écorcheurs (25at), prise par Tosear (403at). Ville franche du crime ; foule immense (« le meilleur des camouflages ») ; ≈ 25 % de Frères de l'Aube, réprimés. 🔒 Protégée par le pacte de Terminus |
| **Franchise** | Milieu de K4, sous Myrth, au-dessus des limites de l'UGL | Station-ville, nation autoproclamée démocratique | Indépendante | Active mais **moteur détruit** (immobile, jamais complètement réparée) ; bâtie en 300bt | 150 M / 10 M / 250 K | Ancienne prison des NU puis de l'Empire ; révoltée en 12at, ravagée, crue morte par l'Empire, reconstruite en secret. Dirigeants élus. Défendue par des mercenaires et tueurs à gages (≈ 60 % des forces au sol : Mercenaires de Cendres ; ≈ 40 % des navires : Flotte Démocratique de Franchise). **Plus grand sanctuaire hashashin** ; **temple principal des Frères de l'Aube** (≈ 30 % de la population), dont les raids contre le sanctuaire ensanglantent la station |
| **Le Verrou** | Centre de J6 | Station pénitentiaire | UGL | Active, inaugurée en 361at | 1 M détenus / 2 M / 250 K | Prisonniers de guerre impériaux, criminels, profiteurs de la Rébellion. Gardée par la Flotte de défense du Verrou (Amiral Odo Vrell, aussi chef de la prison). Contraste assumé : « le Verrou détient, le bagne impérial exploite » |
| **Rempart** | Système Ambroise (K9) | Station de défense | UGL | En construction | 0 / 125 K / 100-150 K | Défense du grenier et escorte des convois |
| **Le Berceau** | Système Cygnara (L8) | Station-chantier (croiseurs) | UGL | En construction, défendue par la Flotte de la Libération | 0 / 125 K / 100-150 K | Minerai de Ferral, carburant de Bruma, terres rares d'Aldène |
| **L'Enclume** | Système d'Hexis (M7) | Station-chantier (cuirassés) | Hexa-Corp | En construction, en accélération | 0 / 125 K / 100-150 K | Le jour où elle forgera son premier cuirassé, Hexa-Corp changera de rang |

### 3.6 Phénomènes et lieux remarquables

- **Le noyau galactique** (J7/K7) : trou noir central ordinaire, sans mystère.
- **La nébuleuse du Croissant** (coin est de M6, No Man's Land) : nébuleuse en arc qui **bloque l'hyperespace** et dégrade les capteurs. On ne la traverse qu'en vitesse normale, par des passes que seuls les pirates connaissent. Sanctuaire de la Ligue des Pirates (Rade-Noire en son cœur). 🔒 L'Empire (EESS) en possède une carte secrète des passes.
- **La Ceinture** (système Vorrh) et la **ceinture de Ferrok** : ceintures d'astéroïdes industrielles.
- **Les champs de débris du système Solaire** (L8) : pèlerinage.
- **Les cimetières de flottes de la Grande Guerre** : ils existent (réacteurs contaminés, reliques et armes de l'Essaim, marché noir), mais aucun n'est encore localisé.
- **Les ruines** : Espérance (J9, interdite), les ruines de l'exil fédéral sur Rocaille (N11), Cendres (M5).

### 3.7 Les grands hubs de Portails

| Hub | Case | Camp | Rôle |
|---|---|---|---|
| Concordia | K7 | UGL | Le carrefour le plus fréquenté de la galaxie |
| Cygnara | L8 | UGL | Cœur industriel, dessert l'arsenal de la reconstruction |
| Vorrhal | P7 | Empire | Carrefour de la forteresse Vorrh |
| Karza | N6 | Empire | « Porte d'entrée de l'Empire » : relie ses grands systèmes entre eux |
| Primaterre | P9 | Empire (annexé) | Relie son système aux systèmes des alentours |
| Vantra | L3 | Royaume Grothul | Dessert tout le royaume |
| Hexis | M7 | Hexa-Corp (neutre) | **Le seul hub qui relie à la fois l'UGL et l'Empire** |

Le No Man's Land n'a aucun hub. Partout ailleurs, on circule par paires fixes ou par hyperespace.

### 3.8 Lecture stratégique de la carte

- **Deux blocs de taille égale** face à face, séparés par une bande étroite. Le seul passage instantané entre eux est le hub neutre d'Hexis.
- **L'Empire n'a qu'une direction d'expansion** : l'ouest, donc l'UGL. Sa capitale est à l'abri au bout de l'est ; son cœur industriel (Vorrh et Ferrok) tient en **deux cibles**.
- **L'UGL disperse** ses chantiers (Calderis et Port-Vaillant sont des planètes errantes, invisibles sans coordonnées). Sa capitale est aussi la ville la plus exposée : à quelques systèmes de la Zone Inexplorée.
- **Talons d'Achille** : Cygnara dépend des convois d'Ambroise pour manger ; Concordia dépend des collecteurs d'Hélion pour son énergie ; Bastia dépend des convois ; Grimmar nourrit seule les 23 Md de la Fédération.
- **Le No Man's Land** est la zone la plus active du « temps de paix » : contrebande Tosear, raids de la Ligue depuis le Croissant, camps de Cendres, diplomatie et espionnage à Hexis.
- **Les marches** sont presque vides : la place est réservée aux futurs royaumes.

### 3.9 Tableau des positions (coordonnées de la carte canonique)

*x, y en unités de carte (repère 1000 × 660). « Dessiné en » signale un corps tracé dans une case voisine de sa case canon : c'est la case canon qui fait foi. Pour les étoiles et les ceintures non dessinées, la position est celle du centre du système. Population totale ≈ somme des trois catégories.*

| Objet | Catégorie | Case canon | x | y | Rattachement | Pop. totale ≈ |
|---|---|---|---|---|---|---|
| **Système de Concordia** | système | K7 | 515.5 | 317.6 | UGL — système-capitale | 176,5 Md |
| ↳ Hélion | étoile | K7 | 515.5 | 317.6 | UGL | — |
| ↳ Concordia | planète | K7 | 515.5 | 319.6 | UGL — capitale (ancienne capitale impériale 0at → 357at) | 176,5 Md |
| ↳ Aube | lune de Concordia | K7 | 508.5 | 323.6 | UGL | 200 K |
| ↳ Zénith | lune de Concordia | K7 | 523.5 | 321.6 | UGL | 0 |
| ↳ Crépuscule | lune de Concordia | K7 | 516.5 | 328.6 | UGL | 5,1 M |
| **Système Vorrh** | système | P7 | 766.6 | 322.6 | Empire — système-capitale | 79,3 Md |
| ↳ Vorrh | étoile | P7 | 766.6 | 322.6 | Empire | — |
| ↳ Vorrh-III | planète | P7 | 767.6 | 322.6 | Empire | 0 |
| ↳ Vorrhal | planète | P7 | 755.6 | 312.6 | Empire — capitale depuis 357at | 76,3 Md |
| ↳ Vorrh-II | planète | P7 | 779.6 | 313.6 | Empire | 2,7 Md |
| ↳ Vorrh-IV | planète | P7 | 750.6 | 326.6 | Empire | 105 M |
| ↳ Vorrh-V | planète | P7 | 762.6 | 336.6 | Empire | 600 K |
| ↳ Vorrh-VI | planète | P7 | 781.6 | 330.6 | Empire | 257,5 M |
| ↳ La Ceinture | ceinture | P7 | 766.6 | 322.6 | Empire | 1 M |
| **Système Ferrok** | système | O7 | 725.1 | 325 | Empire — 2e système industriel | 3,2 Md |
| ↳ Ferrok | étoile | O7 | 725.1 | 325 | Empire | — |
| ↳ Ferrogane | planète | O7 | 718.1 | 318 | Empire | 1,9 Md |
| ↳ Karvek | planète | O7 | 734.1 | 332 | Empire | 1,3 Md |
| ↳ Ceinture de Ferrok | ceinture | O7 | 725.1 | 325 | Empire | — |
| **Système Ostrag** | système | O5 | 725.1 | 225.1 | Empire — système-bagne | 10,8 M |
| ↳ Ostrag | étoile | O5 | 725.1 | 225.1 | Empire | — |
| ↳ Ostrag-Majeur | planète | O5 | 717.1 | 218.1 | Empire | 5,5 M |
| ↳ Ostrag-II | planète | O5 | 734.1 | 220.1 | Empire | 1,8 M |
| ↳ Ostrag-III | planète | O5 | 737.1 | 231.1 | Empire | 2 M |
| ↳ Ostrag-IV | planète | O5 | 723.1 | 236.1 | Empire | 1,5 M |
| ↳ Morne | planète | O5 | 713.1 | 229.1 | Empire | 50 K |
| **Système Primaterre** | système | P9 | 762 | 424.2 | Empire (annexé pendant la Renaissance) — ex-Royaume de Primaterre | 16 Md |
| ↳ Ostre | étoile | P9 | 762 | 424.2 | Empire (annexion) | — |
| ↳ Solferra | planète | P9 | 772 | 431.2 | Empire (annexion) | 2,3 M |
| ↳ Primaterre | planète | P9 | 762 | 416.2 | Empire (annexée pendant la Renaissance) — ex-capitale du Royaume de Primaterre | 15 Md |
| ↳ Gravanne | planète | P9 | 753 | 432.2 | Empire (annexion) | 1 Md |
| **Système Cygnara** | système | L8 | 593 | 394.5 | UGL — poumon industriel | 3,1 Md |
| ↳ Cygnara | planète | L8 | 593 | 383.5 | UGL | 2,6 Md |
| ↳ Aldène | planète | L8 (dessiné en M8) | 605 | 392.5 | UGL | 10,1 M |
| ↳ Bruma | planète | L8 (dessiné en L9) | 598 | 404.5 | UGL | 250 K |
| ↳ Ferral | planète | L8 | 582 | 399.5 | UGL | 501 M |
| ↳ Osca | planète | L8 | 583 | 386.5 | UGL | 4,1 M |
| ↳ Le Berceau (station) | Station-chantier (croiseurs) | L8 (dessiné en M9) | 609 | 404.5 | UGL — en construction (défendue par la Flotte de la Libération) | 250 K |
| **Système Solaire** | système | L8 | 552.7 | 362.2 | espace UGL — système détruit | 0 |
| ↳ Le Soleil | étoile | L8 | 552.7 | 362.2 | espace UGL | — |
| ↳ Terra ✝ | planète | L8 | 552.7 | 362.2 | aucune (ruines en espace UGL) | 0 |
| **Système Ambroise** | système | K9 | 525.1 | 425 | UGL — le grenier | 12,5 Md |
| ↳ Ambre et Braise | étoile | K9 | 525.1 | 425 | UGL | — |
| ↳ Moisson | planète | K9 | 517.1 | 419 | UGL | 10 Md |
| ↳ Fenaison | planète | K9 | 535.1 | 423 | UGL | 2,5 Md |
| ↳ Brumaire | planète | K9 | 522.1 | 436 | UGL | 250 K |
| ↳ Rempart (station) | Station de défense | K9 | 539.1 | 434 | UGL — en construction | 250 K |
| **Système d'Hexis** | système | M7 | 639.5 | 330 | Hexa-Corp — neutre, au centre du No Man's Land | 3,4 Md |
| ↳ Hexis | planète | M7 | 636.5 | 326 | Hexa-Corp (indépendante et neutre) | 3,4 Md |
| ↳ L'Enclume (station) | Station-chantier (cuirassés) | M7 | 646.5 | 338 | Hexa-Corp — en construction — loin d'être finie, mais elle accélère | 250 K |
| **Système Vesper** 🔒 | système | I10 | 425 | 475.1 | UGL — colonie secrète en Zone Inexplorée | 1,8 M |
| ↳ Vesper 🔒 | planète | I10 | 419 | 471.1 | UGL (colonie secrète, 405at) | 775 K |
| ↳ Verdine 🔒 | planète | I10 | 433 | 482.1 | UGL (secret) | 1 M |
| **Système de Kholvar** | système | J11 | 496.9 | 524.9 | La Fédération (sauf Sabraze et Vrasq) | 23,3 Md |
| ↳ Kholvar | étoile | J11 | 496.9 | 524.9 | La Fédération | — |
| ↳ Vrasq | planète | J11 | 462.9 | 519.9 | aucune (hors Fédération) | 0 |
| ↳ Sabraze | planète | J11 | 470.9 | 532.9 | aucune — enclave libre hors Fédération | 25 K |
| ↳ Ordrakh | planète | J11 | 483.9 | 515.9 | La Fédération | 203,5 M |
| ↳ Tesk | lune d'Ordrakh | J11 | 488.9 | 511.9 | La Fédération | 0 |
| ↳ Grimmar | planète | J11 | 499.9 | 531.9 | La Fédération | 2,2 Md |
| ↳ Draskal | lune de Grimmar | J11 | 494.9 | 535.9 | La Fédération | 180 M |
| ↳ Ignar | lune de Grimmar | J11 (dessiné en K11) | 504.9 | 535.9 | La Fédération | 150 M |
| ↳ Vaskir | lune de Grimmar | J11 | 499.9 | 525.9 | La Fédération | 1 Md |
| ↳ Tharkast | planète | J11 (dessiné en K11) | 517.9 | 516.9 | La Fédération — capitale | 18,2 Md |
| ↳ Molvrek | lune de Tharkast | J11 (dessiné en K11) | 522.9 | 512.9 | La Fédération | 200,5 M |
| ↳ Hargan | lune de Tharkast | J11 (dessiné en K11) | 512.9 | 512.9 | La Fédération | 1,3 Md |
| ↳ Ulveth | planète | J11 (dessiné en K11) | 530.9 | 529.9 | La Fédération | 5 M |
| **Calderis** | planète errante | L7 | 575 | 325 | UGL | 3,6 Md |
| **Port-Vaillant** | planète errante | K5 | 525.1 | 225.1 | UGL | 1,6 Md |
| **Alcyone** | planète | L6 | 575 | 274.9 | UGL | 20 M |
| ↳ Maïa | lune d'Alcyone | L6 | 569 | 275.9 | UGL | 155 M |
| ↳ Mérope | lune d'Alcyone | L6 | 581 | 275.9 | UGL | 30 M |
| **Bastia** | planète | K8 | 525.1 | 374.9 | UGL | 175 M |
| **Karza** | planète errante | N6 | 675.2 | 274.9 | Empire | 1 Md |
| **Myrth** | planète | K4 | 506.2 | 151.4 | Sœurs de Myrth (exclusivement) | 500 |
| **Vashyra** | planète | L11 | 593 | 508.6 | indépendante, isolationniste | 60 M |
| **Rade-Noire** | planète | M6 | 645 | 272 | Ligue des Pirates | 21 M |
| **Cendres (ex-Promesse)** | planète | M5 | 630.2 | 225.8 | aucune | 1,6 Md |
| **Espérance** ✝ | planète | J9 | 465.9 | 414.3 | aucune (ruines interdites) | 0 |
| **Rocaille** | planète | N11 | 675.2 | 524.9 | aucune | 0 |
| **Grotha** | planète | M3 | 624.9 | 124.9 | Royaume Grothul | 5 Md |
| **Vantra** | planète | L3 | 575 | 124.9 | Royaume Grothul | 1,2 Md |
| **Skarn** | planète | N4 | 675.2 | 175 | Royaume Grothul | 355 M |
| **Jask** ✝ | planète | ❓ | — | — | aucune | 0 |
| **Terminus** (station) | Station-ville | P10 | 755.1 | 491.9 | Syndicat Tosear (base principale) | 260,5 M |
| **Franchise** (station) | Station-ville — nation autoproclamée démocratique | K4 | 525.1 | 175 | indépendante | 160,3 M |
| **Le Verrou** (station) | Station pénitentiaire | J6 | 474.9 | 274.9 | UGL | 3,3 M |
| **Le noyau galactique (trou noir central)** | phénomène | J7/K7 (jonction) | 500 | 330 | — | — |
| **Nébuleuse du Croissant** | phénomène | M6 (coin est) | 645 | 272 | — | — |

---

## 4. Puissances et factions

### 4.1 L'Empire Vorrhen (0at → aujourd'hui) — capitale Vorrhal (P7)

- **Nature** : empire autoritaire, humano-centré et méritocratique, né du parti du **Renouveau Impérial**. Le nom « Vorrhen » (celui du premier empereur) reste celui de l'État, même si la dynastie est éteinte.
- **Institutions** : un **empereur « élu à vie »** doté de tous les pouvoirs ; un **sénat** (plus hauts gradés, gouverneurs, représentants de communautés) qui vote lois et décisions, mais sur lequel l'empereur a un **veto absolu** et qu'il peut **recomposer à sa guise**. Aucune religion : on ne glorifie que l'empereur. Méritocratie (les postes se gagnent par les preuves), réservée de fait aux humains.
- **Espèces** : « **libres mais non citoyennes** » (pas de vote, hautes responsabilités presque inaccessibles, vies moins valorisées) ; certaines sont asservies de fait ; les espèces non évoluées sont déplacées ou exploitées ouvertement ; les Saluviens sont interdits d'armée ; 🔒 des Ethari servent en secret. La magie est interdite et chassée.
- **Territoire** : la moitié orientale du monde civilisé, dont environ la moitié des territoires de l'ancienne Union, repris pendant la Renaissance. Systèmes décrits : Vorrh (P7), Ferrok (O7), Ostrag (O5), Primaterre (P9, annexé), Karza (N6).
- **Dirigeants** : l'**empereur Corvan Sarkis** (15e empereur, depuis 400at) ; le **Grand Amiral Ardan Vek**.
- **Économie** : production navale centralisée sur Vorrh et Ferrok ; État payé en Wuro ; dépendance envers Hexa-Corp (**la moitié des missiles impériaux** sort de sa division Armement, contrat CLAV ; l'Escadre de Défense de Primaterre est louée).
- **Objectif** : écraser l'UGL et conquérir toute la galaxie. Concordia n'est plus un trône à reprendre mais une **cible à détruire** (le coup au moral ultime). Vorrhal doit rester la capitale symbolique.
- **Mémoire** : légitimité fondée sur « nous seuls avions vu venir l'Essaim » ; humiliation de la Rébellion ; infamie de la trahison des Golden Sword (405at) ; annexion de Primaterre (le roi assassiné par les Hashashin contre l'offrande d'un général fidèle).
- 🔒 **Secrets** : l'Empire **finance la Ligue des Pirates** (qui bloque la contrebande vers son territoire et harcèle l'UGL sans qu'un vaisseau impérial soit engagé) ; **pacte de Terminus** avec Tosear (403at) ; **EESS**, son programme secret d'exploration de la Zone Inexplorée depuis 320at (alors qu'il soupçonne l'UGL d'en avoir un) ; carte secrète des passes du Croissant ; Ethari cachés ; la vengeance de Sarkis contre Myrth couve.
- ❓ **Le mystère central de l'univers** : les « **moyens flous** » qui ont permis à Sarkis de reconstruire une flotte en 400at. Ce n'est **pas** un pacte avec Hexa-Corp (ses services étaient publics et ouverts aux deux camps) ; le vol des Golden Sword en explique une partie, pas tout. Pistes évoquées par l'auteur, **aucune tranchée** : technologie ou ressources trouvées dans les ruines de la guerre, recours caché à la magie, ou quelque chose venu de la Zone Inexplorée.

### 4.2 L'Union Galactique Libre (357at → aujourd'hui) — capitale Concordia (K7)

- **Origine** : une alliance secrète formée **vers 347at sur Concordia même**, au cœur de la capitale impériale (d'où son ralliement éclair en 357at). Elle réunit des citoyens impériaux jugeant le totalitarisme périmé, les espèces lasses d'être non citoyennes, les victimes de la répression, des royaumes extérieurs et, secrètement, les Sœurs de Myrth. Après le Conseil empoisonné, ralliements en chaîne : l'Empire perd 30 % de ses systèmes et 50 % de sa flotte d'un coup, puis 90 % de son territoire en deux ans. Structures improvisées et révoltes internes empêchèrent l'UGL d'achever l'Empire.
- **Institutions (à Concordia)** : la **Présidence** (mandat de 8 ans, 3 mandats au plus : « l'anti-empereur ») ; le **Sénat des Mondes** (chaque monde membre y a voix ; lois et budget) ; le **Conseil des Espèces** (peut bloquer toute loi discriminatoire).
- **Présidente** : **Nyra Maren**, élue en 411at à 26 ans, réélue en 419at (second mandat jusqu'en 427at ; un troisième et dernier mandat la mènerait à 435at). En façade, la paix temporaire pour reconstruire ; en privé, tous les moyens d'affaiblir l'Empire.
- **Valeurs** : héritage revendiqué des Nations Unies (démocratie, égalité de toutes les espèces, laïcité, libertés). Crédit officiel ; Wuro interdit. Magie tolérée sous surveillance. Dette envers les Sœurs de Myrth honorée discrètement (Myrth protégée en sous-main) mais relation officielle gelée.
- **Armée** : **FOU** (Flottes Officielles de l'Union, sous le Grand Amiral Cassian Drey, qui ne dépend que de la Présidence) et **FDU** (Flottes Décentralisées de l'Union, propriété des gouverneurs, coordonnées par la Grande Amirale Petra Solen ; intégrables aux FOU en temps de guerre). Personnel aux mêmes standards dans les deux. Doctrine de projection.
- **Situation en 425at** : économie ruinée, reconstruction depuis 15 ans ; dispositif défensif massé le long du No Man's Land (dont l'UGL a elle-même détruit les Portails) ; les FOU se reconstruisent autour du NSU Liberté (420at) ; cliente d'Hexa-Corp (**30 % de ses armes d'infanterie**, contrat CLPE courant depuis la perte d'une grande usine sur Promesse, dont le terme contractuel est « la destruction de l'Empire » ; le Garant et 10 frégates loués pour Concordia) tout en s'inquiétant qu'elle arme l'ennemi ; courtise les neutres de la Bordure ; subit les raids de la Ligue des Pirates ; vit à quelques systèmes de la Zone Inexplorée.
- **Gouverneurs notables** : Teo Malvin (Cygnara, dirige aussi la Guilde des Minerais), Aldric Manne (Alcyone), Mira Castel (Port-Vaillant).
- 🔒 **Secrets** : programme secret d'exploration de la Zone Inexplorée (position officielle : « non rentable ») ; colonie de **Vesper** (405at) ; **FSS** et son cuirassé « détruit », le NSU Lumières.
- ❓ Grandes figures du Sénat des Mondes et du Conseil des Espèces ; tensions internes (anciens impériaux ralliés, mondes nostalgiques de l'ordre impérial, dette envers les royaumes extérieurs).

### 4.3 La Fédération (350bt → aujourd'hui) — capitale Tharkast (système de Kholvar, J11)

- **Identité** : « La Fédération », sans autre nom. Ancienne colonie des Nations Unies (410bt), indépendante en **350bt** au terme d'une guerre si ruineuse que l'Union préféra lâcher prise plutôt que détruire le système. **Un seul système**, **≈ 23 Md d'habitants**, tout le système de Kholvar sauf Sabraze et Vrasq. **Troisième puissance de la galaxie**, et la seule que personne n'a vue venir.
- **Histoire** : l'un des premiers systèmes tombés face à l'Essaim (53bt →) ; exil d'une grande partie du peuple sur **Rocaille** ; retour après le Triomphe, infrastructures retrouvées en grande partie intactes (l'Essaim avait pris le système sans le raser) ; puis repli total. Pendant quatre siècles, elle n'a parlé au monde que pour **chasser des pirates** et **commercer**. Depuis peu, elle s'intéresse à la **diplomatie extérieure**.
- **Régime** : totalitaire, **parti unique** (le Parti de la Fédération, dont l'ancien nom est oublié, et c'est une fierté) ; le **Chancelier**, chef de l'État, des armées et des flottes, ne se vote pas : il se gagne. **L'Arène** : tous les cinq ans, le Chancelier affronte à mort ses prétendants dans l'Arène de Tharkast ; le survivant gouverne. Titulaire : **Hakkon Skeld**, au pouvoir depuis 391at, six victoires (déduction : la prochaine Arène tomberait vers 426at). **Militarisme intégral** : 1,3 Md de militaires pour 21 Md de civils ; agriculture et mines administrées comme des unités de l'armée.
- **Politique étrangère** : isolationnisme armé (on ne visite pas la Fédération ; les Portails NU du système sont des pièges mortels) ; **Sabraze**, le « sas », est laissée hors du territoire par calcul : on y commerce, négocie et espionne sans jamais entrer ; chasse aux pirates d'une brutalité réputée dans les marches sud ; commerce avec l'UGL, l'Empire et les guildes de la Bordure via Sabraze.
- **Perception extérieure** : tout le monde connaît son existence ; on la classe « puissance régionale isolationniste ». Personne ne connaît sa force.
- **Forces** : préfixe FED ; trois flottes (Exil, Défense, 🔒 Attaque) ; toutes les coques sortent de **Hargan** ; plusieurs vaisseaux hérités des Nations Unies servent encore.
- 🔒 **Secrets** : depuis 410at, elle construit la **Flotte d'Attaque**, destinée à devenir la plus grande force offensive jamais assemblée ; en 425at, elle aligne déjà 4 cuirassés, 4 porte-vaisseaux et 10 croiseurs, et en alignera 5, 6 et 15 d'ici 430at, plus que l'Empire ou l'UGL. L'ouverture diplomatique récente n'est pas de la curiosité : **c'est de la préparation**.
- ❓ Statut des espèces non humaines ; ce que la Fédération veut faire de sa Flotte d'Attaque (conquête, dissuasion, revanche sur les héritiers des Nations Unies ?).

### 4.4 Hexa-Corp (3at → aujourd'hui) — siège Hexis (M7)

- **Nature** : corporation militaro-industrielle **indépendante et neutre**, fondée en 3at par le général **Avrel Hex**, héros du Triomphe, sur la planète Hexis offerte par l'Empire. Elle fournit **tous les services militaires imaginables à tous les camps, ouvertement**, et prospère comme jamais depuis la Fausse Paix.
- **Gouvernance** : la lignée d'Avrel Hex s'est éteinte ; un **directoire des six divisions** gouverne collégialement. Les divisions sont **en concurrence** entre elles et se vendent leurs services à tarif préférentiel.

| Division | Directeur·rice | Activités et contrats |
|---|---|---|
| **Chantiers** | Wella Nash | Construction sur mesure (**CCV**, vaisseaux préfixés du préfixe du client, liste d'attente) et réparation (**CRV**). ≈ 10 % des vaisseaux militaires de la galaxie depuis 3at. Grand projet : **L'Enclume** |
| **Mercenariat** (la plus importante) | Kael Brandt | Flottes entières dirigées par le client (**CFM**) ; vaisseaux loués avec équipage (**CVLM**) ou sans (**CVL**) ; équipages (**CE**) ; mercenaires au sol (**CM**). Tout est location, couvert par des clauses de remboursement ; vaisseaux loués non préfixés |
| **Armement** | Osa Ferren | Vaisseaux d'occasion (**CVV**) ; équipements de personnel à l'achat (**CAEE**) ou en location (**CLPE** : plus gros client l'UGL, 30 % de ses armes) ; armement de vaisseaux à l'achat (**CAAV**) ou en location (**CLAV** : plus gros client l'Empire, la moitié de ses missiles). Rachète et revend des épaves remises à neuf |
| **Logistique** | Dario Melk | Remorquage (**CRS**), ravitaillement (**CR**), transport (**CT**), minage spatial (**CMS**) ; convois escortés |
| **Renseignement** | « Le Facteur » (identité inconnue de tous, même du directoire) | 3e réseau d'espions de la galaxie ; vend et achète l'information au plus offrant ; pas de contrats écrits ; plus gros client : le Royaume Grothul ; traque des mauvais payeurs |
| **Finance** | Ona Krayne | **La Banque** (« taux d'Hexis ») ; **le Casino** (2e de la galaxie, paris sur événements, y compris sur la guerre) ; **le Coffre** ; assurance de flotte (**CAV**), crédit (**CC**), rachat de dette (**CRD**), crédits de guerre (**CCG**, royaumes et factions), hypothèque (**CH**, saisie automatique : ainsi est né le HC Recouvrement) ; investissements et financement de colonisations |

- **Règle d'or** : deux contrats Hexa-Corp ne s'affrontent jamais (sur un théâtre donné, le premier signé a priorité) ; pour le reste, **au plus offrant**, sans plafond.
- **Forces propres** : les Flottes de la Corporation (préfixe HC), Alpha (défense d'Hexis) et Beta (traque des mauvais payeurs), sous le Grand Amiral Deylan Mora (Andari). Elles n'agiraient au nom d'Hexa-Corp que si elle prenait un jour parti, ce qui n'est pas prévu.
- **Clients notables** : l'UGL (CLPE, CVL), l'Empire (CLAV, CVLM), le Royaume Grothul (CFM de Skarn, CM de Vantra, renseignement), Franchise (CFM), le Syndicat Tosear (le TOS Tribut acheté au grand jour ; 🔒 coques de la Flotte Nordique effacées des registres).
- **Atouts** : Hexis tient le seul hub reliant les deux blocs ; Hexa-Corp sait construire des Portails ; elle s'approvisionne auprès des guildes de la Bordure, moins chères et hors quotas des deux blocs.
- **Tension montante** : ni l'Empire ni l'UGL n'acceptent plus sereinement qu'elle équipe l'ennemi. Sa neutralité est un pilier de l'équilibre ; chacun se demande combien de temps elle tiendra.

### 4.5 Crime organisé et piraterie

- **Le Syndicat Tosear** — principale organisation criminelle de la galaxie, dirigée par un **clan de l'espèce tosear** (modèle du syndicat Pyke : conseil dirigeant issu du clan, lieutenants par territoire ; on ne traite qu'avec un émissaire du clan, qui n'oublie jamais une dette). **Base principale : Terminus** (P10). Il fait tourner le **marché noir entre l'UGL et l'Empire** (le seul « commerce » qui traverse vraiment le No Man's Land), vend nourriture et armes dans les zones pauvres, carburant lourd et pièces ailleurs, troque avec les chantiers en pénurie, pratique racket, corruption de gouverneurs, jeux illégaux, change au noir du Wuro, et même, à l'occasion, le nucléaire. Traqué par les deux puissances, qui l'utilisent probablement en secret l'une contre l'autre (❓). **Guerre ouverte avec la Ligue des Pirates** ; rivalité avec le Royaume Grothul (le TOS Trophée a été volé à la couronne). Fondé pendant la Grande Guerre par un clan réfugié d'Espérance, qui élimina ensuite les gangs alliés. 🔒 Pacte de Terminus avec l'Empire ; fournisseur secret d'informations de l'Empire ; **Flotte Nordique** sous faux pavillon pirate (OXL) ; d'autres liens avec l'Empire et l'UGL existent (à développer par l'auteur).
- **Le Royaume Grothul** — « le crime devenu État » (marches nord, L3 → N3 + N4 ; mondes Grotha, Vantra, Skarn). Les **Grothul** monopolisent le pouvoir et tous les postes principaux ; les autres espèces y prospèrent sans jamais gouverner. Royaume officiel (frontières, tribunaux, diplomatie : on ne peut pas y « faire une descente ») et syndicat du crime (la couronne prélève sa part sur tout l'illégal). Sécurité **externalisée** : Mercenaires de Cendres au sol, contrats Hexa-Corp dans l'espace ; une seule flotte en propre, la Garde dorée. Plus gros client du Renseignement d'Hexa-Corp. **Trois plaies** : le Trophée volé par Tosear, la « Grande Faucheuse » qui massacre ses convois du nord (🔒 la Flotte Nordique de Tosear, ce que la couronne ignore), et le **tribut payé à Raeilla la Guerrière**. Dirigeants non nommés (❓).
- **La Ligue des Pirates** — fédération de **huit meutes** basée à **Rade-Noire**, dans la nébuleuse du Croissant (No Man's Land), sous le **Roi des pirates Torg Varga** (qui prit le trône vers 418at en anéantissant en une nuit la flotte de l'ancien Roi). Proies : la contrebande Tosear et les convois et mondes frontaliers de l'UGL ; les convois impériaux sont curieusement épargnés. Ni l'Empire ni l'UGL n'ont jamais officiellement tenté de prendre Rade-Noire. 🔒 Financée en sous-main par l'Empire.
- **Les pirates indépendants** — tous les pirates n'appartiennent pas à la Ligue. La plus célèbre : **Raeilla la Guerrière** et ses « Sans-Couronne » (nord de la Bordure). Elle a refusé de plier le genou devant Varga, a mis le **Royaume Grothul au tribut** (GRT Bannière détruit, GRT Sceptre capturé) et a infligé à la Flotte Nordique sa seule défaite (OXL Moissonneuse détruit) ; la « grande faucheuse » l'évite depuis. Sa loi : « Le tribut, ou le vide. »
- **La Confrérie des Hashashin** — ce que la galaxie croit : un ordre d'**assassins humains** au service d'un dieu, **le Silencieux** (« toute vie est un bruit »). On n'achète pas une mort : on « **parraine une offrande** » (tribut immense, rituel d'acceptation ; l'ordre peut refuser). Sanctuaires sur les principales planètes, gardés par des mercenaires « humains » ; le plus grand est sur **Franchise**. Méthode inexplicable : ils atteignent n'importe quelle cible. Offrande la plus célèbre : un général fidèle livré par Sarkis pour faire assassiner le roi de Primaterre. 🔒 **Les Hashashin sont les Vashyr de Vashyra** : la méthode est l'imitation parfaite (remplacer un garde, un serviteur, la cible elle-même) ; les offrandes sont des êtres vivants **réduits en esclavage sur Vashyra** (le général de Sarkis y sert, vivant) ; les gardes « humains » sont peut-être eux-mêmes des Vashyr.
- **Les Écorcheurs** ✝ (≈ 25at → 403at) — gang né dans les bas-fonds de Port-Vaillant, chassé vers 25at, qui fit de l'ancienne base militaire NU de **Terminus** une station-ville ouverte à tous les fuyards. Leur flotte, « la pilleuse » (chef Malko Dresh, croiseur volé ECR Dépeceur), pilla le **sud de l'Empire** pendant l'Empire réduit. Tosear, leur locataire et « protecteur », les livra en 403at au **piège** de la 1ère Flotte impériale (fausse rumeur de cargaison de carburant radioactif) : tous les vaisseaux et capitaines détruits. Morale du crime : « ne laisse jamais un plus gros que toi payer ton loyer ». Des survivants ou héritiers, ennemis à la fois de Tosear et de l'Empire, existent peut-être.

### 4.6 Bordure, ordres, guildes et mercenaires

- **Les factions de la Bordure** — une mosaïque de royaumes et principautés, d'alliances régionales, de syndicats de guerriers, de guildes marchandes (maîtresses des routes sans Portails) et de mondes-refuges (où se cachent sans doute des ordres magiques). Neutralité majoritaire mais fragile : l'UGL et l'Empire courtisent, infiltrent ou menacent ; la Bordure sert aussi de soupape (contrebande, déserteurs, réfugiés, négociations officieuses). Leçon de Primaterre : **la richesse sans flotte est une invitation**. Peu de royaumes sont encore créés.
- **Les Sœurs de Myrth** — ordre exclusivement féminin de **magie noire**, basé uniquement sur **Myrth** (K4), très redouté. Deux fois elles ont changé l'Histoire : pendant la Grande Guerre, elles anéantirent des légions de drones aux côtés des pro-empire (pertes immenses, jamais reconnues ni récompensées par l'Empire, qui cacha ce recours à la magie) ; en **357at**, invitées au grand conseil impérial, elles **empoisonnèrent toutes les personnes présentes** (empereur, amiraux, gouverneurs, sénateurs fidèles), après plus de trois siècles d'une vengeance transmise au sein de l'ordre. Longévité ≈ 2 × celle d'un humain (prolongeable par des sorts, au prix d'une corruption accélérée) ; leur puissance croît avec l'âge, et la hiérarchie suit l'âge ; chaque Sœur abandonne son nom pour un nom de code. Elles n'agissent que pour les Sœurs : ni pour l'UGL, ni pour l'Empire ; elles se préparent au cas où un ennemi externe ferait basculer la galaxie. Tout ce qui les concerne est secret. 🔒 ≈ 100 Sœurs et ≈ 400 apprenties, des **enfants enlevées** à travers la galaxie (une sur dix devient Sœur ; on n'entend plus parler des autres) ; dirigées par la **matriarche Benedict** depuis 178at, qui ordonna le Conseil empoisonné ; la vengeance de Sarkis contre Myrth est en préparation.
- **Les Frères de l'Aube** — secte apocalyptique **bannie dès les Nations Unies**, l'un des plus vieux ordres humains (déjà présente sur Terra avant 450bt), héritière d'une religion terrienne disparue (leur dieu est « Dieu »). Doctrine : la **Grande Nuit** engloutira toute vie si tous les êtres ne sont pas convertis, par la parole ou par la force. L'Essaim leur a semblé donner raison (« nous l'avions dit ») : aucun lien réel, une coïncidence exploitée. Présents sur toutes les planètes civilisées de la Bordure ; places fortes : **Franchise** (≈ 30 %, temple principal), **Terminus** (≈ 25 %, réprimés par Tosear), **Rade-Noire** (protégés par Arven le Doux). Hiérarchie : le **Prêtre Suprême Malachie** (Franchise), un Grand Prêtre par implantation, des prêtres. Haine mutuelle avec les Hashashin.
- **Le Royaume de Primaterre** ✝ (390bt → Renaissance) — premier royaume de la Bordure, l'un des plus riches (minerai de Gravanne, greniers de Primaterre, énergie de Solferra), indépendant grâce à un gros pot-de-vin. Sa flotte, presque détruite pendant la Grande Guerre, ne fut jamais reconstruite. Le roi fut assassiné par les Hashashin (offrande de Sarkis) et le royaume annexé. **Occupation en 425at** : protestations, sabotages et révoltes, toutes réprimées dans le sang par l'Escadre de Défense de Primaterre (« la Flotte des Oppresseurs ») de l'Amiral Dreven Kass ; QG chassé de Primaterre vers Solferra ; croiseur amiral endommagé par la dernière révolte. Les loyalistes sont au bagne d'Ostrag. Ce qui reste de la maison royale ne s'exprime plus (❓ héritier).
- **La Guilde Marchande de Primaterre** — plus vieille guilde de la Bordure (410bt). Sous tutelle impériale (licences, taxes, inspecteurs), officiellement docile et indispensable, bienfaitrice de la vie civile (greniers, hôpitaux, écoles). 🔒 Double jeu : elle utilise le réseau de contrebande de Tosear (Terminus est tout proche) pour contourner l'Empire et relever Primaterre ; ni l'Empire ni la population ne le savent.
- **La Guilde des Minerais de l'Union** — bras commercial du gouvernement de Cygnara, dirigé par le gouverneur **Teo Malvin** en personne. Le gouvernement de Cygnara possède toutes les mines du système ; les quotas de l'Union sont livrés d'abord, le surplus est vendu par la Guilde et les bénéfices reviennent à Cygnara. Ce trésor a payé, cette fois-ci, les réparations de la Flotte de la Libération face à la montée des pirates. Certains sénateurs regardent ce cumul d'un œil de plus en plus attentif.
- **Le Groupe des Mercenaires de Cendres** — le seul concurrent sérieux d'Hexa-Corp, **au sol uniquement** (des régiments, pas de flottes) : moins chers, sans clauses ni registres, durs et fiables. Clients : le Royaume Grothul (garnisons de Grotha, Vantra et Skarn), Franchise (≈ 60 % des forces au sol), Rade-Noire (garnisons de la Ligue). Nés des camps de Cendres ; leur argument : « ils n'ont rien à perdre, ayant déjà tout perdu ». La division Mercenariat d'Hexa-Corp recule devant eux dans la Bordure. Dirigeants et effectifs ❓.
- **Les Golden Sword** ✝ (0at → 405at) — la plus grande compagnie de mercenaires de l'Histoire (≈ 10 % des effectifs militaires connus à son apogée), fondée en 0at par des déserteurs écœurés par l'inaction des Nations Unies, avec une hiérarchie très claire. Engagée tout entière par Sarkis en 400at, envoyée en première ligne, saignée ; en 405at, quand elle réclama son paiement, l'Empire **assassina ses chefs** et intégra de force les survivants à l'armée régulière. Héritiers et vétérans dispersés (armée impériale, Ostrag, Bordure, vengeance) ; légende d'une cache d'armes sur Rocaille.
- **Franchise** (station-nation, K4) — voir 3.5 : démocratie de parias, président élu (non nommé ❓) qui a loué la Flotte Démocratique à Hexa-Corp par crainte que l'Empire se souvienne de la station.
- **Les Jaskari** — espèce-diaspora devenue la guilde marchande la plus répandue et la plus lucrative de la galaxie (section 5).

### 4.7 Factions historiques

- **Les Nations Unies** (600bt → 0at), « l'Union » : démocratie laïque née de l'unification de Terra, puissance interstellaire dès 450bt. Le Standard, l'écu, le réseau des Portails, l'intégration des espèces compatibles avec les mêmes droits que les humains (les espèces peu développées laissées intactes, les Vashyr refusés). Choix lourd : 40 % de la galaxie laissés inexplorés. Deux fautes historiques : intégrer l'Essaim (75bt), puis concentrer sa flotte dans un exercice géant (la Nuit Silencieuse). Décapitée, elle céda le pouvoir aux pro-empire avec l'accord du peuple. Sa mémoire est un champ de bataille : « la belle époque trahie » pour l'UGL, « la faiblesse qui a failli tous nous tuer » pour l'Empire.
- **Le Renouveau Impérial** : parti pro-empire minoritaire et patient de l'Expansion (empereur élu à vie, sénat sous contrôle, primauté humaine, culte de l'empereur), qui refusa toujours de prendre le pouvoir par la force. Porté par de grandes familles comme les Vorrhen de Vorrhal ; seul à voir venir l'Essaim (il arma ses fidèles et garda ses flottes à l'écart). En 55bt, Alarik Vorrhen, 23 ans, en prend la tête, réoriente les fonds vers la flotte et purge le favoritisme. À la Nuit Silencieuse, ses flottes (25 % du total de l'Union) sont épargnées ; elles portent 53 ans de reconquête. En 3bt, il mise 75 % de son budget sur l'antimatière d'Avrel Hex. En 0at, le parti devient l'État. ❓ Fondateur, monde d'origine et premiers succès électoraux du parti.

---

## 5. Espèces

| Espèce | Origine | Traits | Dans l'UGL | Dans l'Empire | Notes |
|---|---|---|---|---|---|
| **Humains** | Terra (détruite) | Aucune faculté : le nombre, l'organisation, l'industrie | Citoyens | Seuls citoyens | ≈ 75 % de la population connue ; espèce sans monde-mère ; le conflit actuel est aussi une guerre civile humaine (« l'humanité doit-elle régner seule ? ») |
| **Andari** | Concordia (natifs) | Quasi identiques aux humains (souvent confondus) ; métabolisme lent ; vivent **jusqu'à 150 ans** (90 ans ≈ la cinquantaine humaine) | Citoyens ; hauts postes (Amiral Edran Kole) | Non citoyens | Mêlés aux colons depuis la colonisation : beaucoup ont des ancêtres andari sans le savoir ; « mémoires vivantes » (archivistes, juges, diplomates, officiers) |
| **Jaskari** | Jask (détruite ; nom = proposition) | Ex-tribus de ferrailleurs ; espèce **très soudée** | Citoyens | Non citoyens (mais l'Empire commerce avec eux) | La **guilde est l'espèce** : la guilde marchande la plus répandue de la galaxie, « on échange de tout » ; escroquer un Jaskari ferme la guilde entière, partout, pour toujours |
| **Grothul** | Grotha (M3) | — (physiologie ❓) | — | — | Espèce dirigeante du Royaume Grothul ; clan et État confondus ; sécurité achetée à des mercenaires |
| **Tosear** | Espérance (détruite) | Reconnaissables au premier regard (impossible de se fondre dans la foule) | — | — | Accusés à tort de complicité avec l'Essaim (75bt) ; le clan réfugié a fondé le Syndicat ; « la dette est sacrée » ; pas d'État, un réseau |
| **Ethari** (pouvoir) | Aucun monde connu : nomades, présents partout dès l'époque des NU | **Télépathes** : rien ne protège d'une lecture ; la plupart lisent à quelques mètres, de rares **prodiges** à grande distance (chacun « vaut une flotte ») ; lire épuise, jusqu'à tuer | Citoyens, majoritairement intégrés, avec une méfiance sociale persistante | 🔒 Gardés en secret dans l'armée | Décimés pendant la Grande Guerre (en première ligne pour lire les flottes de l'Essaim, dont la pensée-somme écrasait les lecteurs) ; rares survivants intégrés aux deux armées |
| **Saluviens** (pouvoir) | Cygnara | Bipèdes très proches des humains, couverts de **fourrure** ; **soin par transfert d'énergie vitale** (inné) ; trop soigner tue le soigneur ; aucune résurrection | Très largement intégrés à l'armée (soigneurs des cas graves, vénérés) | **Interdits d'officier dans l'armée** | Intégrés aux NU dès leur découverte |
| **Vashyr** (pouvoir) | Vashyra (L11) | **Métamorphes** : imitation parfaite d'un individu précis, voix comprise ; tenir une forme épuise (quelques jours au plus) ; **température corporelle de 50 °C** en toute forme (un scanner thermique les trahit, mais personne ne cherche) | Intégration refusée (déjà par les NU) | — | Officiellement isolationnistes, « ne quittent jamais leur monde » (seuls quelques renégats légendaires de l'espionnage). 🔒 Ils **sont** les Hashashin ; société tribale de clans en guerre pour le titre de clan dirigeant ; cruauté normale ; esclaves issus des offrandes. ❓ Apparence véritable, noms des clans |
| **L'Essaim** | Inconnue (**mystère définitif**) | Collectif cybernétique : **assimile** les êtres en **drones** ; conscience-ruche unique contrôlée par la **Reine** ; patience stratégique ; guerre psychologique | — | — | Découvert en 75bt ; 90 % des drones désactivés au Triomphe (mort de la Reine), ≈ 10 % exilés dans la Zone Inexplorée ; silence depuis 425 ans. **Aucune désassimilation n'a jamais réussi** (recherche interdite par les deux puissances). Reliques et technologies récupérées et étudiées en secret par les deux puissances ; marché noir des épaves dans la Bordure. 🔒 **Une nouvelle Reine a émergé** : l'Essaim se reconstruit |

- **Espèces non évoluées** (« non civilisées ») : laissées sans contact par les NU et l'UGL ; l'Empire les déplace ou les exploite sans dissimulation. Exemple : la faune de Zénith ; les créatures et espèces indigènes d'Aldène.
- **Espèces disparues** : plusieurs ont été totalement exterminées pendant la Grande Guerre (❓ lesquelles, traces, survivants).
- **Animaux** : faune terrienne exportée par la colonisation, faunes locales.
- **La Fédération** est majoritairement humaine (statut des autres espèces ❓).

---

## 6. Forces militaires en 425at

### 6.1 Où sont les flottes

| Flotte | Entité | Base → position en 425at | État |
|---|---|---|---|
| 1ère Flotte « L'Invincible » | Empire | Vorrh (P7) → Vorrh | En réparation ; tient l'intérim défensif de la capitale |
| 2ème Flotte « Le Dernier Rempart » | Empire | Vorrh → **Ferrok (O7)** | Très mauvais état, reconstruction |
| 3ème Flotte « Les Semeurs de Mondes » | Empire | Karza (N6) | Opérationnelle (de fait la 2e de l'Empire) |
| EESS 🔒 | Empire | Mobile → **nord de la Zone Inexplorée** | Opérationnelle |
| Escadre de Défense de Primaterre | Empire (vaisseaux loués) | Solferra (P9) | Opérationnelle, croiseur amiral en réparation |
| Flotte de garde d'Ostrag | Empire | Ostrag-Majeur (O5) | Permanente (composition ❓) |
| EDS / FDM | Empire | Chaque système / chaque monde civilisé impérial | Standard |
| Flotte de la Libération | UGL (FOU) | Concordia → **système Cygnara (L8)** | Opérationnelle, la plus puissante de l'UGL |
| Flotte du Concorde | UGL (FOU) | Concordia (K7) | Très diminuée, en réparation |
| Flotte de l'Aube | UGL (FOU) | Calderis (L7), en patrouille | Opérationnelle (devenue la 2e de l'UGL) |
| FSS 🔒 | UGL (FOU) | Vesper (I10) ; le Méridien explore la Zone | Opérationnelle |
| Flotte défensive de Concordia | UGL (FDU) | Concordia | Opérationnelle (en partie louée) |
| Flotte d'Alcyone | UGL (FDU) | Alcyone (L6) | Reconstruite, réduite |
| Flotte de Port-Vaillant | UGL (FDU) | Port-Vaillant (K5) | Reconstruite, réduite |
| Flotte de défense du Verrou | UGL (FDU) | Le Verrou (J6) | Opérationnelle |
| Flotte de défense de Cygnara | UGL (FDU) | Cygnara (L8) | Opérationnelle |
| Flotte de l'Exil | Fédération | Tharkast (J11) | Opérationnelle |
| Flotte de Défense | Fédération | Vaskir / Grimmar (J11) | Opérationnelle, la plus puissante fédérale en service |
| Flotte d'Attaque 🔒 | Fédération | Hargan (J11) | En construction (opérationnelle vers 430-440at) |
| Flotte Alpha | Hexa-Corp | Hexis (M7) | Opérationnelle |
| Flotte Beta | Hexa-Corp | Hexis → **mobile dans toute la galaxie** | Opérationnelle |
| Flotte de Défense de Skarn | Hexa-Corp (CFM, Royaume Grothul) | Skarn (N4) | Opérationnelle |
| Flotte Démocratique de Franchise | Hexa-Corp (CFM, Franchise) | Franchise (K4) | Opérationnelle |
| Gardiens de Vantra (au sol) | Hexa-Corp (CM, Royaume Grothul) | Vantra (L3) | En poste |
| Flotte de Terminus | Syndicat Tosear | Terminus (P10) | Opérationnelle ; une partie dispersée dans les trafics |
| Flotte Nordique 🔒 (OXL) | Syndicat Tosear | Aucune base → **nord de la Bordure** | Opérationnelle |
| Garde dorée | Royaume Grothul | Grotha (M3) | Opérationnelle, réduite |
| Les huit meutes | Ligue des Pirates | Rade-Noire (M6) / No Man's Land | Opérationnelles |
| Meute de Raeilla | Pirates indépendants | Mouillages secrets → **nord de la Bordure** | Opérationnelle |

### 6.2 Empire Vorrhen — ordre de bataille

- **1ère Flotte « L'Invincible »** (attaque, la force de frappe). Surnom hérité de la première flotte du Renouveau Impérial, épargnée à la Nuit Silencieuse et fer de lance du Triomphe. Amiral **Torvin Askel** (héros de la Renaissance) sur le **VIV Vorrhen** ; le Grand Amiral dirige parfois en personne les grandes offensives coordonnées. **2 cuirassés** : Vorrhen (amiral, fierté de l'Empire, construit avant la Renaissance), Triomphe (vieux, du début de l'Empire : la plus grande puissance de feu impériale, mais moins avancé). **2 PV** : Fauconnier (très endommagé à Ferrogane, déployable en mauvais état), Aiglon (tout juste livré par Ferrogane). **7 croiseurs** neufs de Ferrogane : Glaive, Lance, Estoc, Pique, Masse, Espadon, Framée. 30 frégates, ≈ 100 corvettes. En réparation à Vorrh. A anéanti les Écorcheurs (403at), rasé Promesse et brisé l'Écarlate et les Fantômes Blancs à Ferrogane (409at, en y perdant son ancien amiral, le Conquérant).
- **2ème Flotte « Le Dernier Rempart »** (défense de Vorrh et Ferrok). Amirale **Neva Sorren** (« la plus grande stratège de l'Empire », victorieuse à Ostrag contre un ennemi plus nombreux, mieux armé et déjà sur place) sur le **VIV Bastion**. **2 cuirassés** : Bastion (vieille génération, longue réparation), Alarik (premier cuirassé de l'Empire, promis au démembrement en 399at, sauvé par la Renaissance ; « un miracle qu'il fonctionne encore »). L'Intrépide, le plus récent et le plus puissant des trois, a été détruit à Ostrag. **2 PV** : Grand-Duc (éventré par la charge suicide du NSU Serment, condamné au démantèlement), Gerfaut (récent, très endommagé à Ferrogane, échangé par la 1ère contre l'Aiglon). **4 croiseurs** : Ténace (seul croiseur survivant d'Ostrag, en réparation), Sabre (confié par la 1ère pour réparation), Fléau et Hallebarde (opérationnels, prêtés par la 1ère). 25 frégates dont 5 opérationnelles, ≈ 100 corvettes opérationnelles. Ses vaisseaux en bon état sont régulièrement redirigés vers les flottes d'attaque, qui lui confient en retour leurs vaisseaux à réparer. On l'accuse injustement d'avoir « abandonné » Ferrogane.
- **3ème Flotte « Les Semeurs de Mondes »** (prise de territoire, invasions, colonisation). Amiral **Joss Vek** (jugé inutile, placé là comme « connaissance proche » du Grand Amiral, révélé à la Trouée) sur le **VIV Apogée** (navire de prestige de 300at, aujourd'hui dépassé). **1 PV** : Arche (vieux mais en bon état : transports de troupes et appui d'artillerie). **5 croiseurs** : Coutelas et Merlin (endommagés à la Trouée, non opérationnels), Doyen (le plus vieux croiseur en service, évidé en croiseur-transport, tenu en retrait), Braquemart et Fauchard (rafistolés pendant l'Empire réduit, en bon état). 20 frégates, des centaines de corvettes, **la majorité des transports de troupes**. Doctrine : la 1ère fixe les flottes ennemies, la 3ème prend les mondes.
- **EESS 🔒** (Escadrille d'Explorations Scientifiques Secrète, depuis 320at) : VIV **Théorème** (ancien croiseur devenu vaisseau scientifique, déguisé en cargo, amiral), VIV **Corollaire** (ancienne frégate scientifique déguisée en corvette d'escorte, capable de reprendre l'expédition), 1 frégate grimée en cargo, 2 corvettes sans symboles, 2 transports de carburant lourd (autonomie maximale, aucun contact avec des mondes habités). Toujours dirigée par un scientifique : aujourd'hui la professeure **Anya Rell** (xénogéologue). Elle explore le nord de la Zone Inexplorée ; elle a cartographié les passes du Croissant.
- **Escadre de Défense de Primaterre** (« la Flotte des Oppresseurs ») : l'EDS du système, faite de vaisseaux loués à Hexa-Corp avec équipages (CVLM) : **le Mandataire** (croiseur d'Hexa-Corp, amiral, en réparation sur Solferra), 10 frégates (8 d'Hexis, 2 impériales), 80 corvettes d'Hexis. Amiral **Dreven Kass**.
- **EDS** (une par système impérial, même ceux qui ont déjà une flotte) : 1 croiseur amiral souvent vieux (Sous-Amiral, qui gère aussi les avant-postes et stations de défense), 10-15 frégates, 75-100 corvettes, des centaines de chasseurs. **FDM** (une par planète civilisée) : 8 frégates dont une grande frégate amirale (Contre-Amiral), 12 corvettes, ≈ 100 chasseurs.

### 6.3 Union Galactique Libre — ordre de bataille

- **Flotte de la Libération** (FOU) : la plus puissante de l'UGL, première flotte assemblée par la Rébellion, qui « libéra » Concordia. Amiral **Edran Kole** (Andari) sur le **NSU Liberté**. **2 cuirassés** : **Liberté** (Calderis, 420at : **le cuirassé le plus puissant jamais construit**, jamais éprouvé au combat ; à l'entraînement, la flotte ne serait pas loin de pouvoir résister seule à la 1ère Flotte impériale), Insurgé (ancien amiral, mutilé à Ostrag, en réparation à Calderis ; commandé par le sous-amiral Vasco Reth). **3 PV** : Havre (endommagé à la défense de Promesse, en réparation), **Arsenal** (tout juste sorti de Port-Vaillant : le plus gros et le plus avancé porte-vaisseaux de la galaxie), Tête-de-Pont (transport de troupes, réparé, attend ses nouveaux transports, détruits à Promesse). **4 croiseurs** : Ferveur (vestige de la Rébellion, remis en service en 424at), Audace, Résolution, Fulgurance (nouvelle génération, de Port-Vaillant et de Cygnara : les trois croiseurs les plus puissants de l'UGL). 25 frégates (15 en réparation, 5 héritées de la Rébellion, 5 neuves de Bastia et d'Alcyone), ≈ 100 corvettes. Déployée à Cygnara pour défendre Le Berceau et le système, au plus près de la frontière sud.
- **Flotte du Concorde** (FOU), « la libératrice de mondes » : ex-2e flotte de l'UGL, presque anéantie à Ostrag. Amirale **Elara Venn** (nommée en 414at) sur le NSU **Fidèle** (amiral par intérim, seul navire lourd opérationnel). Cuirassé **Concorde** (amiral, fierté de l'UGL, en réparation à Calderis), PV **Concordia** (colonisation, en réparation à Port-Vaillant), croiseurs **Généreux** (condamné, désossé pour pièces à quai de Concordia) et **Opiniâtre** (en réparation à Port-Vaillant), 10 frégates (4 opérationnelles), ≈ 50 corvettes (patrouille avec la flotte défensive). Stationnée à Concordia.
- **Flotte de l'Aube** (FOU), « la sauveuse » : flotte de soutien, devenue la 2e de l'UGL. Amiral **Corin Dast** sur le **NSU Levant** (amiral depuis Ostrag). Cuirassé **Zénith** (ex-amiral, en réparation ; l'Amiral Reyan Solvet y mourut en couvrant la retraite du Concorde), PV **Orient** et **Nourrice** (transport de carburant, réparation et ravitaillement), croiseurs **Éclat** et **Lueur** (réparés) et **Étincelle** (en réparation depuis la prise initiale de Promesse), 20 frégates (5 en réparation), ≈ 100 corvettes. Positionnée sur Calderis.
- **FSS 🔒** (Flotte Scientifique Secrète, FOU) : NSU **Lumières** (cuirassé transformé en vaisseau scientifique sans toucher à son armement, amiral, officiellement « détruit par l'Empire »), NSU **Méridien** (croiseur scientifique qui cartographie la Zone Inexplorée avec 4 frégates), NSU **Azimut** (croiseur scientifique qui protège le Lumières et Vesper), 15 frégates dont 4 grimées en transporteurs. Amiral scientifique **Ezren Kaal**.
- **FDU** : **Flotte défensive de Concordia** (« la protectrice de la capitale » : NSU **Sentinelle**, qui n'a jamais quitté le système, amiral ; **le Garant** et 10 frégates loués à Hexa-Corp en CVL avec équipages unionistes ; 30 corvettes ; Amiral Benett Oray) · **Flotte d'Alcyone** (« la sauveuse qui n'arriva jamais » : NSU **Prompt-Secours**, 5 frégates, 15 corvettes ; Amirale Suna Reves ; créée en 405at par le gouverneur Aldric Manne) · **Flotte de Port-Vaillant** (« la Sentinelle des chantiers » : NSU **Charpentier**, 3 frégates, 8 corvettes ; Amiral Joran Filt ; créée en 405at par la gouverneure Mira Castel) · **Flotte de défense du Verrou** (NSU **Geôlier**, 10 frégates, 25 corvettes ; Amiral Odo Vrell) · **Flotte de défense de Cygnara** (« la sauveuse du grenier » : cuirassé NSU **Abondance**, croiseur NSU **Glaneur** en réparation après un raid pirate, 30 frégates, 60 corvettes ; Amiral Bram Tolvar). Bastia n'a pas de FDU propre.
- **Détruites** ✝ à Ferrogane (409at) : la **Flotte Écarlate** (vaisseaux peints en rouge, 4e flotte de l'UGL, meilleur atout contre les petites flottes et convois impériaux ; ex-Alcyone) et la **Flotte des Fantômes Blancs** (vaisseaux blancs, son ombre et sa sœur, qui interceptait les fuyards ; peu connue de l'Empire).
- La « **flotte défensive du No Man's Land** » du langage courant désigne le dispositif d'ensemble massé depuis 410at le long de la frontière (FOU redéployées au sud et FDU frontalières), pas une flotte constituée.

### 6.4 La Fédération — ordre de bataille

- **Flotte de l'Exil** « Ceux du Retour » (garde de Tharkast ; la flotte historique qui sauva le peuple) : Amiral **Ulmar Draveth** sur le FED **Exode** (cuirassé des Nations Unies affrété pour évacuer le système, jamais reparti). PV **Premier-Né** (le tout premier vaisseau sorti de Hargan), croiseurs **Prise** (pirate capturé lors d'un raid sur le système), **Harpon**, **Inflexible**, 20 frégates, ≈ 50 corvettes.
- **Flotte de Défense** « Le Bouclier de Grimmar » (la plus puissante en service ; protège le grenier et répond aux appels des autres mondes) : Amirale **Brynn Kaskar** sur le FED **Inébranlable** (Hargan, 330at). Cuirassé **Vétéran** (NU, qui combattit l'Essaim), PV **Ancêtre** (NU, très vieux) et **Semeur**, croiseurs **Aïeul** (NU), **Meurtrière**, **Soc**, **Barbacane**, **Ressuscité** (relevé d'une épave de la Grande Guerre), 35 frégates, ≈ 100 corvettes. Mouillage : Vaskir.
- **Flotte d'Attaque** 🔒 « Le Poing de la Fédération » (en construction à Hargan depuis 410at ; elle « ne défend rien : elle attend ») : Amiral **Rhoden Tarq** sur le FED **Suprématie** (achevé en 420at). Cuirassé **Hégémonie** (sortie prévue en 430at) ; PV **Aquilon** (le dernier sorti), **Ouragan** et **Tourmente** (d'ici 430at) ; croiseurs **Vindicte** et **Talion** (tout juste sortis), **Verdict**, **Anathème**, **Interdit** (d'ici 430at), **Dernier-Mot** (commandé, d'ici 440at) ; 45 frégates, ≈ 100 corvettes.

### 6.5 Hexa-Corp et ses contrats

- **Flotte Alpha** « l'invaincue » (ironique : elle n'a jamais combattu que des pirates) : défense d'Hexis, qu'elle n'a jamais quittée. Amiral **Casimir Velt** (qui a acheté sa place) sur le HC **Fleuron** (premier croiseur des chantiers d'Hexis). Croiseurs HC **Recouvrement** (ex-croiseur impérial hypothéqué puis saisi par la Banque) et HC **Garde-Fou** (contre la menace de la Ligue toute proche), PV HC **Invendu** (épave impériale réparée, sans acheteur), 10 frégates anciennes, ≈ 50 corvettes.
- **Flotte Beta** « l'ennemie des criminels » : parcourt la galaxie, guidée par le Renseignement, pour faire payer les mauvais payeurs (ceux qui refusent sont capturés avec leurs biens et jugés sur Hexis). Amirale **Naya Dorst** sur le HC **Huissier** (le croiseur le plus puissant d'Hexa-Corp). HC **Repenti** (ex-pirate capturé, ancien amiral), 15 frégates, ≈ 50 corvettes.
- **Flotte de Défense de Skarn** « le mur de Skarn » (CFM, dirigée par les Grothuls) : **la Muraille** (amiral, ex-Flotte Beta), **le Contrefort**, PV **le Donjon** et **la Herse**, 20 frégates, ≈ 50 corvettes ; renforcée après l'annexion de Primaterre et la trahison des Golden Sword. Amiral **Tobin Rasq** (jamais éprouvé : personne n'a jamais attaqué le mur).
- **Flotte Démocratique de Franchise** « le Coup de Pouce d'Hexis » (CFM ; le peuple de Franchise la croit prêtée gratuitement, elle est payée au prix fort) : **le Grognard** (vieux croiseur amiral), 10 frégates, une dizaine de corvettes. Vieil Amiral **Hedric Toll** (alcoolique ; aucun pillard n'a pourtant touché Franchise depuis son arrivée).
- **Gardiens de Vantra** « les Brigands d'Hexis » (CM, au sol) : Général **Marek Sorn**. Payés pour être ripoux : laisser tourner les trafics grothul, tenir Tosear dehors.
- Vaisseaux loués à des flottes étrangères : **le Garant** + 10 frégates (UGL, CVL) ; **le Mandataire** + 8 frégates + 80 corvettes (Empire, CVLM).

### 6.6 Crime et pirates

- **Flotte de Terminus** « La Main du Syndicat » (Tosear, contrôlée directement par les membres du Syndicat) : TOS **Trophée** (amiral : **le plus grand croiseur jamais construit**, pas forcément le plus puissant ; commandé à Hexis par la couronne grothul et volé par Tosear lors d'un affrontement meurtrier), TOS **Revenant** (reconstruit à partir d'épaves de croiseurs de Ferrogane ; escorte les convois dans le No Man's Land), TOS **Tribut** (acheté à Hexa-Corp), PV TOS **Butin** (volé aux Golden Sword lors d'un raid), 30 frégates (20 sous faux noms ou grimées en transports, cachées dans l'UGL et l'Empire), 55 corvettes (40 au trafic). Les vaisseaux dispersés peuvent être rappelés en urgence.
- **Flotte Nordique** 🔒 « la grande faucheuse » (Tosear sous faux pavillon OXL ; la galaxie la prend pour une flotte pirate errante qui pille et détruit tout) : OXL **Faucheuse** (croiseur acheté à Hexis, effacé des registres), PV OXL **Spectre** (le porte-vaisseaux volé en construction à Port-Vaillant par les Écorcheurs, fini avec des pièces d'épaves et méconnaissable), 5 frégates (2 d'Hexis effacées des registres, 3 prises aux Écorcheurs restés sur Terminus), 10 corvettes. Ancien amiral **Kordan Vail**. Cible surtout les convois grothul. A perdu l'OXL Moissonneuse ✝ contre Raeilla.
- **Garde dorée** (Royaume Grothul) : GRT **Couronne** (très vieux croiseur racheté aux Nations Unies, plus de quatre siècles), 4 frégates, 5 corvettes ; contrôlée directement par les dirigeants grothuls. Elle alignait trois croiseurs avant sa défaite contre Raeilla (GRT Bannière ✝, GRT Sceptre capturé).
- **Les huit meutes de la Ligue des Pirates** (sans préfixe, base Rade-Noire) :

| Meute (surnom) | Capitaine | Force | Croiseur chef / particularités |
|---|---|---|---|
| Varga (« la faiseuse de veuves ») | **Torg Varga**, Roi des pirates | 3 croiseurs, 10 frégates, 25 corvettes | **l'Ossuaire** (rouge sang, orné des crânes de ses ennemis), le Châtiment, le Régicide |
| Circa (« la faiseuse d'orphelins ») | Circa, bras droit du Roi | 2 croiseurs, 5 frégates, 15 corvettes | la Morgue ; la Récompense (prise à la flotte de l'ancien Roi) |
| Virgile Le Maléfique (« les fils du diable ») | Virgile Le Maléfique | 1 croiseur, 1 PV, 7 frégates, 20 corvettes | l'Enfer ; le Purgatoire (vieux porte-vaisseaux) |
| Rajik Le Conquérant (« les conquérants ») | Rajik | 1 croiseur, 5 frégates, 15 corvettes | l'Étendard ; préfère les planètes aux convois |
| Nerra la Silencieuse (« les Sans-Voix ») | Nerra | 1 croiseur, 6 frégates, 18 corvettes | le Silence ; coupe toutes les communications avant le premier tir |
| Brakk le Ferrailleur (« les Ferrailleurs ») | Brakk | 1 croiseur, 8 frégates, 12 corvettes | la Carcasse ; désosse les épaves |
| Jumelles Osk (« les Tenailles ») | Ivra et Nyla Osk | 2 croiseurs, 6 frégates, 20 corvettes | la Serre (Ivra), la Griffe (Nyla) ; chasse en tenaille |
| Arven le Doux (« les Courtois ») | Arven le Doux | 1 croiseur, 5 frégates, 15 corvettes | la Révérence (impeccable) ; « qui se rend vit » |

- **Meute de Raeilla la Guerrière** « les Sans-Couronne » (indépendante) : **le Défi** (amiral), **le Sceptre-Brisé** (ex-GRT Sceptre, « l'humiliation faite vaisseau »), **l'Amalgame** (assemblé d'épaves de dix chantiers, classification contestée), PV **l'Inachevé** (capturé presque fini à Port-Vaillant, achevé par la meute), 10 frégates (5 de facture pirate, 5 prises aux vaincus qui ont plié le genou), 15 corvettes. Personne d'autre que Raeilla ne l'a jamais commandée.

### 6.7 Forces disparues ou historiques

- **Flottes des Nations Unies** : ≈ 60 % détruites à la Nuit Silencieuse (concentrées dans un exercice géant avec la plupart des amiraux) ; flottes pro-empire (25 % du total) épargnées, noyau de la future flotte impériale ; une flotte NU détruisit Jask. Noms des grandes flottes ❓ ; nommage proposé (non validé) : « Flottes d'Expansion » numérotées, préfixe UNS.
- **La flottille du Triomphe** (0at) : les quinze frégates d'Avrel Hex, dont cinq modifiées pour l'antimatière. La percée en coûta six (dont une d'antimatière) ; la **Résolue** (vaisseau de Hex) fut touchée, propulsion coupée ; la **Délivrance**, la **Sentence** et l'**Aurore** plongèrent sur Espérance et la détruisirent. La Résolue est préservée, relique la plus sacrée d'Hexa-Corp (lieu ❓).
- **Flottes de l'Essaim** : construites en secret pendant 22 ans, assez puissantes pour écraser 60 % de la flotte de l'Union en une nuit ; grossies des peuples assimilés. 🔒 L'état actuel de ses flottes est inconnu, même de l'auteur. Proposition non validée : les humains nomment ses formations *Ruches* (capitales) et *Nuées* (formations d'attaque), avec des codes militaires (« Nuée-Kappa »).
- **Flotte des Écorcheurs** ✝ « la pilleuse » : ECR **Dépeceur** (croiseur volé à l'Empire, navire-chef de Malko Dresh), ECR **Rançon** (acheté à Hexis, le seul navire lourd « propre » du gang ; capitaine Sera Voln), ECR **Charognard** (frégate convertie en croiseur ; capitaine Ryke Volane), la frégate nommée ECR **Filoute** (ex-Jaro Fesk). Tous détruits au piège de 403at.
- **La flotte perdue de Primaterre** : détruite pendant la Grande Guerre, jamais reconstruite.
- **Les Golden Sword** : flotte et effectifs absorbés de force par l'Empire en 405at.

### 6.8 Chantiers navals

| Chantier | Case | Entité | Production |
|---|---|---|---|
| **Calderis** | L7 | UGL | Chantier principal : toutes classes, **seul chantier à cuirassés de l'UGL** ; cales capables d'un futur vaisseau-mère |
| **Port-Vaillant** | K5 | UGL | Croiseurs et porte-vaisseaux |
| **Cygnara** | L8 | UGL | Croiseurs, frégates, beaucoup de transports (reconstruction) |
| **Bastia** | K8 | UGL | Frégates et transports |
| **Alcyone** | L6 | UGL | Frégates et transports |
| **Le Berceau** (station, en construction) | L8 | UGL | Croiseurs en espace libre |
| **Vorrh-II** | P7 | Empire | **Uniquement des cuirassés** (seul chantier à cuirassés impérial) |
| **Vorrh-V** | P7 | Empire | Chantier géant du vaisseau capitale, suspendu depuis 400at |
| **Ferrogane** | O7 | Empire | Croiseurs lourds et porte-vaisseaux |
| **Karvek** | O7 | Empire | Frégates, corvettes (l'essentiel des corvettes impériales), transports |
| **Hexis** | M7 | Hexa-Corp | Corvettes, frégates, croiseurs, une grande partie des transports, et au moins un porte-vaisseaux (le Donjon, « un des premiers » des chantiers d'Hexis) ; ≈ 10 % des vaisseaux militaires de la galaxie depuis 3at |
| **L'Enclume** (station, en construction) | M7 | Hexa-Corp | Cuirassés |
| **Hargan** | J11 | La Fédération | Toutes classes, du chasseur au cuirassé ; 🔒 cadence inconnue de l'extérieur |

- Deux stratégies opposées : l'UGL **disperse** sa production (perdre un chantier est un coup dur, jamais fatal) ; l'Empire **centralise** dans deux systèmes (rendement maximal, mais deux cibles suffiraient à décapiter son industrie). Les corvettes n'ont pas de lieu attitré (petits chantiers, planètes importantes, stations), sauf dans l'Empire (Karvek).
- Les **cuirassés** ne sortent que de trois sites : Calderis, Vorrh-II, Hargan. L'Enclume sera le quatrième. Aucun **vaisseau capitale** n'a jamais été construit.

### 6.8 bis Devises des flottes (champ « Doctrine » des fiches)

| Flotte | Devise |
|---|---|
| 1ère Flotte « L'Invincible » | « La défaite est pour les faibles et la retraite pour les peureux. » |
| 2ème Flotte « Le Dernier Rempart » | « Nous sommes les défenseurs de l'Empire. » |
| 3ème Flotte « Les Semeurs de Mondes » | « Là où nous passons, l'Empire s'enracine. » |
| EESS 🔒 | « La science est l'arme la plus efficace. » |
| Flotte de la Libération | « La liberté a un prix : nous sommes venus le payer. » |
| Flotte du Concorde | « Là où la flotte passe, les peuples sont libérés. » |
| Flotte de l'Aube | « Une flotte sans support est une flotte qui ne rentrera pas au port. » |
| FSS 🔒 | « Le savoir est la seule frontière. » |
| Flotte d'Alcyone | « Aucun appel ne reste sans réponse. » (tragiquement ironique depuis la Trouée) |
| Flotte de Port-Vaillant | « Qui protège les chantiers protège la flotte entière. » |
| Flotte de défense de Cygnara | « Notre garde ne dort jamais. » |
| Flotte Écarlate ✝ | « L'Empire est le prochain. » |
| Flotte des Fantômes Blancs ✝ | « Nous sommes déjà derrière vous. » |
| Flotte de l'Exil (FED) | « Nous sommes revenus. Nous ne repartirons plus. » |
| Flotte de Défense (FED) | « On ne prend pas notre pain. » |
| Flotte d'Attaque (FED) 🔒 | « Quand nous frapperons, il sera trop tard. » |
| Flotte Alpha (HC) | « La meilleure défense, c'est la clientèle. » |
| Flotte Beta (HC) | « On est là pour faucher la mauvaise herbe. » |
| Flotte de Défense de Skarn | « Un mur qui dissuade est un mur qui tient. » |
| Flotte Démocratique de Franchise | « La liberté a aussi des canons. » |
| Gardiens de Vantra | « Un port propre est un port prospère. » (l'ironie est dans le contrat) |
| Flotte de Terminus (TOS) | « Tout le monde paye, tôt ou tard. » |
| Flotte Nordique (OXL) 🔒 | « Pas de témoins, pas de drapeau. » |
| Meute de Varga | « Nous ne laissons que des veuves. » |
| Meute de Circa | « Avec nous, pas d'hôpital mais directement la morgue. » |
| Meute de Virgile Le Maléfique | « Voyons combien de temps vous allez survivre aux pires tortures. » |
| Meute de Rajik Le Conquérant | « Aucun monde n'est imprenable. » |
| Meute de Nerra la Silencieuse | « Personne ne vous entendra. » |
| Meute de Brakk le Ferrailleur | « Rien ne se perd. » |
| Meute des jumelles Osk | « Une proie, deux mâchoires. » |
| Meute d'Arven le Doux | « La politesse ne coûte rien ; le refus, si. » |
| Meute de Raeilla la Guerrière | « Le tribut, ou le vide. » |
| Flotte des Écorcheurs ✝ | « On prend tout, jusqu'à la peau. » |

Sans devise : Escadre de Défense de Primaterre, Garde dorée, Flotte défensive de Concordia, Flotte de défense du Verrou.

### 6.9 Comparatif des navires lourds (425at)

| Puissance | Cuirassés | Porte-vaisseaux | Croiseurs | Remarques |
|---|---|---|---|---|
| **Empire** (VIV) | 5 en service : 2 opérationnels (Triomphe, Apogée), 3 en réparation (Vorrhen, Bastion, Alarik) | 5 : Aiglon et Arche opérationnels, Fauconnier diminué, Gerfaut très endommagé, Grand-Duc condamné | 16 nommés : 12 opérationnels, 4 en réparation | + croiseurs amiraux des EDS (non nommés), le Mandataire (loué) ; 🔒 2 navires scientifiques. Pertes récentes : Intrépide, Conquérant |
| **UGL** (NSU) | 6 : 3 opérationnels (Liberté, Levant, Abondance), 3 en réparation (Insurgé, Concorde, Zénith) | 6 : Arsenal, Orient, Nourrice opérationnels ; Tête-de-Pont en attente de transports ; Havre et Concordia en réparation | 15 : 11 opérationnels, 3 en réparation, 1 condamné (Généreux) | + le Garant (loué) ; 🔒 Lumières (cuirassé scientifique armé), Méridien et Azimut. Pertes de 409at : 2 cuirassés, 8 PV, 16 croiseurs |
| **La Fédération** (FED) 🔒 | 4 opérationnels (Exode, Inébranlable, Vétéran, Suprématie) + 1 en construction | 4 opérationnels + 2 en construction | 10 opérationnels + 4 en construction ou commandés | Objectif 430at : 5 cuirassés, 6 PV, 15 croiseurs. Les deux blocs l'ignorent |
| **Hexa-Corp** | 0 (L'Enclume en construction) | 1 (HC) + 2 loués (Skarn) | 5 (HC) + 5 loués | Les loués servent des clients étrangers |
| **Syndicat Tosear** | 0 | 1 (TOS) + 🔒 1 (OXL) | 3 (TOS) + 🔒 1 (OXL) | Dont le plus grand croiseur jamais construit (Trophée) |
| **Royaume Grothul** | 0 | 0 | 1 (Couronne) | Le reste de sa puissance est loué |
| **Ligue des Pirates** | 0 | 1 (le Purgatoire) | 12 | Huit meutes |
| **Meute de Raeilla** | 0 | 1 (l'Inachevé) | 2 ou 3 (l'Amalgame est contesté) | Indépendante |

---

## 7. Personnages

*Statut en 425at. ✝ = mort. 🔒 = existence ou fonction secrète. Les personnages sans date de naissance n'ont pas d'âge défini.*

### 7.1 Empire Vorrhen

| Personnage | Fonction | Statut | Points clés |
|---|---|---|---|
| **Corvan Sarkis** | 15e empereur (depuis 400at), Vorrhal | Vivant | Ancien sénateur de l'Empire réduit. A pris le pouvoir par la force et fait tuer tous les anciens sénateurs dès le début de 400at, puis s'est fait élire à vie par un sénat recomposé. A reconstruit une flotte par des « moyens flous » et lancé la Renaissance. Impitoyable, redoutablement efficace, encore vigoureux. Aucun lien avec les Vorrhen, et il s'en vante (« la dynastie est morte de sa faiblesse ; moi, je n'hérite de rien, je mérite tout »). Veut écraser l'UGL et détruire Concordia. A livré un général fidèle aux Hashashin pour Primaterre ; a trahi les Golden Sword. ❓ Âge, apparence, entourage, héritier, passé avant 400at |
| **Grand Amiral Ardan Vek** | Chef de l'armée impériale | Vivant | Rend compte directement à Sarkis ; dirige parfois les grandes offensives depuis la 1ère Flotte |
| **Amiral Torvin Askel** | 1ère Flotte (VIV Vorrhen) | Vivant | Héros de la Renaissance, artisan des reconquêtes |
| **Amirale Neva Sorren** | 2ème Flotte (VIV Bastion) | Vivante | La plus grande stratège de l'Empire ; vainqueure d'Ostrag |
| **Amiral Joss Vek** | 3ème Flotte (VIV Apogée) | Vivant | « Connaissance proche » du Grand Amiral (lien exact ❓), moqué jusqu'à sa victoire écrasante à la Trouée |
| **Amiral Dreven Kass** | Escadre de Défense de Primaterre (le Mandataire) | Vivant | D'une grande cruauté ; réprime toute révolte dans le sang ; « les Oppresseurs » |
| **Professeure Anya Rell** 🔒 | Commandante de l'EESS (VIV Théorème) | Vivante | Xénogéologue ; ignore que l'Essaim se reconstruit dans la Zone qu'elle explore |
| **Professeur Elm Cassar** 🔒 | Premier commandant de l'EESS (320at) | ✝ | Le professeur le plus respecté de l'école scientifique la plus respectée de l'Empire |
| *Le général offert aux Hashashin* | Général fidèle de Sarkis (nom ❓) | 🔒 Vivant, esclave sur Vashyra | Le prix de l'assassinat du roi de Primaterre |

Les quinze empereurs sont listés en 8.5.

### 7.2 Union Galactique Libre

| Personnage | Fonction | Statut | Points clés |
|---|---|---|---|
| **Nyra Maren** | Présidente de l'UGL (élue en 411at, réélue en 419at) | Vivante, née en 385at (40 ans) | Petite-fille de Doran Maren ; tempérament guerrier ; façade de paix temporaire, tous les leviers en privé pour affaiblir l'Empire ; unique héritière d'une lignée endeuillée, élevée par son père ; les dossiers noirs passent par son bureau (exploration secrète de la Zone, dette envers Myrth). Faucons et colombes votent pour elle |
| **Verian Maren** | Amiral à la retraite (depuis 399at), père de Nyra | Vivant, né en 344at (81 ans) | On lui prête l'oreille de la Présidente sur les questions de flotte ; dernier lien vivant avec la Rébellion fondatrice |
| **Grand Amiral Cassian Drey** | Chef des FOU | Vivant | A vu la fine fleur des FOU se briser en 409at ; reconstruit méthodiquement, prudemment, autour du Liberté |
| **Grande Amirale Petra Solen** | Codirige les FDU avec les gouverneurs | Vivante | Poste plus politique que militaire : chaque ordre se négocie |
| **Amiral Edran Kole** | Flotte de la Libération (NSU Liberté) | Vivant, Andari | Contre-amiral à la libération de Concordia (357at), sous-amiral à la défense de Promesse, amiral depuis 414at ; servit sous Doran Maren ; c'est grâce à lui que la Libération existe encore |
| **Sous-amiral Vasco Reth** | Commandant du NSU Insurgé | Vivant | Ancien amiral de la Libération, en charge de la débâcle d'Ostrag, rétrogradé en 414at ; sert sous l'homme promu à sa place, sur l'épave de sa défaite |
| **Amirale Elara Venn** | Flotte du Concorde (NSU Fidèle) | Vivante | Nommée en 414at, tout droit sortie de l'école des gradés ; veut faire ses preuves |
| **Maro Fenn** | Ancien amiral du Concorde | Vivant, destitué en 414at | Jugé en grande partie responsable de la débâcle d'Ostrag |
| **Amiral Corin Dast** | Flotte de l'Aube (NSU Levant) | Vivant | Fin stratège ; ancien second de Solvet ; la rumeur dit qu'il a proposé la manœuvre du Zénith |
| **Amiral scientifique Ezren Kaal** 🔒 | FSS (NSU Lumières) | Vivant | Le plus brillant scientifique de l'UGL ; a proposé la FSS ; découvreur de Vesper (405at) ; ignore l'existence de l'EESS et le retour de l'Essaim |
| **Amiral Benett Oray** | Flotte défensive de Concordia (NSU Sentinelle) | Vivant | Vieil amiral proche de la retraite, bon palmarès |
| **Amirale Suna Reves** | Flotte d'Alcyone (NSU Prompt-Secours) | Vivante | A mené les secours vers Ferrogane ; a dû fuir la Trouée, sauvée par le sacrifice du Dévouement |
| **Amiral Joran Filt** | Flotte de Port-Vaillant (NSU Charpentier) | Vivant | Seul à fuir la Trouée avec son croiseur |
| **Amiral Odo Vrell** | Flotte du Verrou (NSU Geôlier) et chef de la prison | Vivant | Rien n'entre ni ne sort du Verrou sans lui |
| **Amiral Bram Tolvar** | Flotte de défense de Cygnara (NSU Abondance) | Vivant | « Le Tueur de pirates » ; n'a jamais affronté de vraie flotte |
| **Gouverneur Teo Malvin** | Cygnara et son système ; dirige la Guilde des Minerais | Vivant | A fait financer par la Guilde les réparations de la Flotte de la Libération |
| **Gouverneur Aldric Manne** | Alcyone | Vivant | Ancien gradé ; a créé et financé la Flotte d'Alcyone (405at), esprit d'entraide |
| **Gouverneure Mira Castel** | Port-Vaillant | Vivante | Finance la Flotte de Port-Vaillant (405at) |
| **Doran Maren** ✝ | Cofondateur de la Rébellion, premier amiral de la Libération, libérateur de Concordia | 318at – 398at (mort de vieillesse) | Héros fondateur de l'UGL |
| **Liora Maren** ✝ | Mère de Nyra, épouse de Verian | 345at – 385at | Morte en donnant naissance à Nyra |
| **Reyan Solvet** ✝ | Ancien amiral de l'Aube | Mort à Ostrag (409at) | Plaça le Zénith entre la 2ème Flotte et le Concorde en fuite ; l'une des rares gloires commémorées |
| **Vessa Karn** ✝ | Ancienne amirale de la Flotte Écarlate | Morte à Ferrogane (409at) | À bord du Carmin |
| **Telmo Brac** ✝ | Ancien amiral des Fantômes Blancs | Mort à Ferrogane (409at) | À bord de l'Albâtre, en tentant de sauver l'Écarlate |

### 7.3 La Fédération

| Personnage | Fonction | Statut | Points clés |
|---|---|---|---|
| **Chancelier Hakkon Skeld** | Chef de l'État, du Parti, des armées et des flottes | Vivant, au pouvoir depuis 391at | A tué pour le poste, puis six fois pour le garder ; a lancé la Flotte d'Attaque (410at) 🔒 |
| **Amiral Ulmar Draveth** | Flotte de l'Exil (FED Exode) | Vivant | Héros local, dont les écoliers de Tharkast apprennent le nom |
| **Amirale Brynn Kaskar** | Flotte de Défense (FED Inébranlable) | Vivante | Héroïne locale ; la militaire la plus respectée après le Chancelier |
| **Amiral Rhoden Tarq** 🔒 | Flotte d'Attaque (FED Suprématie) | Vivant | Pas un héros de guerre (il n'a pas eu de guerre) : l'homme choisi par Skeld pour tenir l'arme qu'il forge |

### 7.4 Hexa-Corp

| Personnage | Fonction | Statut | Points clés |
|---|---|---|---|
| **Wella Nash** | Directrice des Chantiers | Vivante | Entrée à 14 ans comme apprentie-soudeuse ; 40 ans de carrière ; bâtisseuse de L'Enclume |
| **Kael Brandt** | Directeur du Mercenariat | Vivant | Ancien mercenaire de terrain (contrats CM sur une dizaine de mondes) ; ne laisse personne oublier que sa division est la plus importante |
| **Osa Ferren** | Directrice de l'Armement | Vivante | Ingénieure en balistique ; tient le grand livre des contrats CLPE (UGL) et CLAV (Empire) |
| **Dario Melk** | Directeur de la Logistique | Vivant | Ancien capitaine de convoi : 30 ans sur les routes les plus dangereuses sans perdre une cargaison majeure |
| **« Le Facteur »** | Directeur·rice du Renseignement | Identité inconnue de tous | Siège par écran, voix maquillée ; « livre » l'information au plus offrant |
| **Ona Krayne** | Directrice de la Finance | Vivante | Issue des salles de marché ; quand elle ajuste le taux d'Hexis, deux économies de guerre toussent |
| **Grand Amiral Deylan Mora** | Flottes de la Corporation | Vivant, Andari | Vétéran de la Rébellion (prise de Concordia), venu chercher une « pré-retraite plus tranquille » |
| **Amiral Casimir Velt** | Flotte Alpha (HC Fleuron) | Vivant | N'a jamais vraiment combattu ; a acheté sa place après ses succès à la Finance |
| **Amirale Naya Dorst** | Flotte Beta (HC Huissier) | Vivante | Arnaquée jadis par un mauvais payeur, elle les traque ; a capturé Jaro Fesk |
| **Amiral Tobin Rasq** | Flotte de Défense de Skarn (la Muraille) | Vivant | Sorti de l'école des officiers d'Hexis ; jamais éprouvé |
| **Amiral Hedric Toll** | Flotte Démocratique de Franchise (le Grognard) | Vivant | Vieux, en fin de carrière, alcoolique |
| **Général Marek Sorn** | Gardiens de Vantra | Vivant | Le « ripou sous contrat » : sa bande passe pour les mercenaires les plus pourris de la galaxie |
| **Avrel Hex** ✝ | Fondateur (3at) | 68bt – 27at (95 ans) | Héros du Triomphe, protégé d'Alarik Vorrhen ; sa lignée est éteinte |

### 7.5 Crime et piraterie

| Personnage | Fonction | Statut | Points clés |
|---|---|---|---|
| **Torg Varga** | Roi des pirates (depuis ≈ 418at), l'Ossuaire | Vivant | Le plus dur et le plus craint ; a anéanti en une nuit la flotte de l'ancien Roi. 🔒 ❓ Sait-il que l'Empire finance la Ligue ? (non tranché) |
| **Circa** | Capitaine, bras droit du Roi (la Morgue) | Vivant | Second de Varga lors du coup d'État, récompensé d'un croiseur pris à la flotte royale ; « Avec nous, pas d'hôpital mais directement la morgue. » |
| **Virgile Le Maléfique** | Capitaine (l'Enfer, le Purgatoire) | Vivant | Réputé l'être le plus dépourvu de morale de la galaxie ; torture à mort ses victimes pour le plaisir ; « habité par le diable », disent les superstitieux |
| **Rajik Le Conquérant** | Capitaine (l'Étendard) | Vivant | A pris et tenu Promesse quelques jours (≈ 405at) |
| **Nerra la Silencieuse** | Capitaine (le Silence) | Vivante | Aucun survivant n'a décrit son visage ; ne parlerait qu'en chuchotant |
| **Brakk le Ferrailleur** | Capitaine (la Carcasse) | Vivant | Ancien ouvrier de chantier ; la moitié des pièces détachées du No Man's Land passent par ses marchés |
| **Ivra et Nyla Osk** | Capitaines jumelles (la Serre, la Griffe) | Vivantes | Filles d'un capitaine mort dans la guerre contre Tosear ; ont fait de leur frégate héritée deux croiseurs |
| **Arven le Doux** | Capitaine (la Révérence) | Vivant | Le pirate le plus poli de la galaxie, ancien commissaire de bord de paquebot (dit-on) ; « qui se rend vit » ; protège les Frères de Rade-Noire. 🔒 **Il est lui-même un Frère de l'Aube** : sa règle est une doctrine (on ne convertit pas les morts) |
| **Raeilla la Guerrière** | Cheffe des Sans-Couronne (le Défi) | Vivante | Pirate indépendante du nord ; ancienne capitaine de la Ligue qui refusa le genou ; Grothuls au tribut ; vainqueure de la Flotte Nordique ; guerrière avant d'être pilleuse, « exacte » plutôt que cruelle |
| **Kordan Vail** 🔒 | Commandant de la Flotte Nordique (OXL Faucheuse) | Vivant | Ancien amiral viré de l'UGL pour cruauté, payé par Tosear pour cette cruauté ; sa tête est mise à prix par la moitié de la Bordure, « sous un nom qui n'est même pas le sien » |
| **Jaro Fesk** | Ancien criminel le plus recherché de la galaxie | Vivant, détenu | Arnaqueur de tout le monde, Hexa-Corp comprise ; capturé sur Rocaille par la Flotte Beta (401at), jugé par le tribunal d'Hexis |
| **Malko Dresh** ✝ | Dernier chef des Écorcheurs (ECR Dépeceur) | Mort en 403at | A mené toute sa flotte dans le piège |
| **Sera Voln** ✝ / **Ryke Volane** ✝ | Capitaines des Écorcheurs (Rançon / Charognard) | Morts en 403at | — |

Les dirigeants du Syndicat Tosear (le clan, son conseil, ses lieutenants) et ceux du Royaume Grothul ne sont pas nommés (❓).

### 7.6 Bordure et ordres

| Personnage | Fonction | Statut | Points clés |
|---|---|---|---|
| **La matriarche Benedict** 🔒 | Dirigeante des Sœurs de Myrth depuis 178at (247 ans) | Vivante | Son existence même est secrète ; « Benedict » est un nom de code ; longévité maintenue par des sorts qui la corrompent un peu plus chaque année ; a préparé et ordonné le Conseil empoisonné |
| **Malachie** | Prêtre Suprême des Frères de l'Aube, au temple principal de Franchise | Vivant | Reçoit les comptes de tous les Grands Prêtres ; parle au nom de Dieu à des millions de fidèles qui attendent la Grande Nuit |

### 7.7 Grandes figures historiques

- **Alarik Vorrhen** (78bt – 24at), natif de Vorrhal : chef du Renouveau Impérial à 23 ans (55bt), seul à préparer la guerre, engagea tout dès la Nuit Silencieuse et mena 53 ans de reconquête ; en 0at, il força un vote, fut élu président et proclama l'Empire ; il fit le don d'antimatière à Avrel Hex (3bt) ; il régna 24 ans (« la meilleure des reconstructions ») et mourut à 102 ans. Un culte populaire le divinisa plus tard (dissous en 280at).
- **Avrel Hex** (68bt – 27at) : voir 7.4 et 8.3 (le Triomphe).
- **Maldric Vorrhen** ✝ (300at – 357at), « l'Empoisonné » : onzième empereur et dernier Vorrhen, descendant direct d'Alarik. Seize ans de règne satisfait pendant que la conspiration se formait sous son toit ; il invita les Sœurs de Myrth au grand conseil et y mourut.
- **Les quinze empereurs** : section 8.5.

---

## 8. Histoire

### 8.1 Les neuf ères

| # | Ère | Dates | En bref |
|---|---|---|---|
| 1 | **L'ère de Terra** | 600bt → 450bt | Unification de l'humanité sur Terra sous les Nations Unies (démocratie, laïcité, écu, le Standard). La surpopulation pousse vers les étoiles |
| 2 | **L'Expansion** | 450bt → 75bt | Colonisation, réseau des Portails, intégration des espèces compatibles ; Primaterre (420bt), Kholvar (410bt) ; 40 % de la galaxie laissés inexplorés ; montée du Renouveau Impérial |
| 3 | **L'Essaim parmi nous** | 75bt → 53bt | Découverte de l'Essaim sur Espérance ; le conseil de l'Union choisit le dialogue plutôt que l'isolement ou la destruction ; l'Essaim, libre de circuler, construit des flottes en secret pendant 22 ans |
| 4 | **La Grande Guerre** | 53bt → 0at | La Nuit Silencieuse, la destruction de Terra, 53 ans de guerre, 55 % de la population morte, des espèces entières exterminées, une grande partie des Portails détruite ; le Triomphe |
| 5 | **L'Empire** | 0at → 357at | Alarik Vorrhen fonde l'Empire (capitale Concordia) ; reconstruction dure, purge des élites, espèces non citoyennes, révoltes écrasées, royaumes d'exilés dans la Bordure ; Hexa-Corp (3at) ; dynastie Vorrhen sous vernis électoral |
| 6 | **La Rébellion** | 357at → 359at | Le Conseil empoisonné ; ralliements en chaîne ; Concordia passe à l'UGL ; le sénat s'exile sur Vorrhal ; l'Empire tombe à ≈ 10 % de son territoire, mais l'UGL exsangue ne peut l'achever |
| 7 | **L'Empire réduit** | 359at → 400at | Quatre décennies d'Empire croupion autour de Vorrhal, trois empereurs d'exil ; ère sombre et mal documentée, où se cache le mystère de Sarkis |
| 8 | **La Renaissance impériale** | 400at → 410at | Sarkis reprend près de la moitié des territoires de l'ancienne Union ; Golden Sword engagés puis trahis ; Primaterre annexée ; contre-offensive unioniste brisée en 409at ; Sarkis suspend ses conquêtes |
| 9 | **La Fausse Paix armée** | 410at → aujourd'hui (425at) | Le No Man's Land aux Portails détruits, le Wuro, la reconstruction de l'UGL, la Bordure neutre, Hexa-Corp prospère. **C'est ici que commencent les histoires** |

### 8.2 Dates clés

| Date | Événement |
|---|---|
| 600bt | Unification de Terra sous les Nations Unies |
| 450bt | Début des conquêtes spatiales ; construction des premiers Portails |
| 420bt | Fondation de Primaterre, première colonie de la Bordure |
| 410bt | Fondation de la colonie de Kholvar ; fondation de la Guilde Marchande de Primaterre |
| 390bt | Indépendance du Royaume de Primaterre (pot-de-vin) |
| 350bt | Indépendance de Kholvar après une guerre ruineuse : naissance de La Fédération |
| 300bt | Construction de la station pénitentiaire des NU (future Franchise) |
| 125bt | Construction de la base militaire avancée des NU (futur Terminus) |
| 78bt | Naissance d'Alarik Vorrhen (Vorrhal) |
| 75bt | Découverte de l'Essaim sur Espérance ; les Tosear, accusés à tort, fuient vers la Bordure |
| 68bt | Naissance d'Avrel Hex |
| 55bt | Alarik Vorrhen, 23 ans, prend la tête du Renouveau Impérial |
| 53bt | **La Nuit Silencieuse** : l'Essaim prend un tiers des systèmes de l'Union en une nuit ; ≈ 60 % de la flotte de l'Union détruite avec son haut commandement ; Kholvar parmi les premiers systèmes tombés (exil sur Rocaille) |
| ≈ 50bt | **Destruction de Terra** et de tout le système Solaire |
| Grande Guerre | L'Essaim tient jusqu'à 50 % du territoire de l'Union ; chute de Vorrhal ; les Sœurs de Myrth anéantissent des légions de drones ; une flotte des NU détruit Jask |
| 3bt | Don d'antimatière d'Alarik à Avrel Hex (75 % du budget du parti) ; début de la traque de la Reine |
| **0at** | **Le Triomphe** : destruction d'Espérance et de la Reine ; 90 % des drones désactivés, le reste s'exile dans la Zone Inexplorée. Alarik Vorrhen force un vote, est élu président et proclame l'Empire (capitale Concordia). Fondation des Golden Sword. Retour du peuple fédéral à Kholvar (après 0at) |
| 3at | Fondation d'Hexa-Corp sur Hexis |
| 12at | Révolte de la station-prison (future Franchise), attaquée par la 1ère Flotte de l'époque et crue morte ; fondation du bagne d'Ostrag peu après |
| 24at | Mort d'Alarik Vorrhen (102 ans) |
| ≈ 25at | Les Écorcheurs, chassés de Port-Vaillant, s'installent sur Terminus |
| 27at | Mort d'Avrel Hex (95 ans) |
| 39at → 315at | Révoltes et crises de l'ère impériale (8.4) |
| 178at | Benedict devient matriarche des Sœurs de Myrth 🔒 |
| 300at | Construction du VIV Apogée |
| 309at | Lancement du chantier géant de Vorrh-V (le *Souverain*) |
| 320at | Création de l'EESS 🔒 |
| 330at | Construction du FED Inébranlable à Hargan |
| ≈ 347at | Formation de l'alliance secrète (future UGL) sur Concordia |
| **357at** | **Le Conseil empoisonné** : mort de l'empereur Maldric et de 90 % des très hauts gradés ; naissance de l'UGL ; Concordia passe à l'Union ; le sénat s'exile sur Vorrhal |
| 357-359at | Libération de Concordia ; l'Empire réduit à ≈ 10 % de son territoire |
| ≈ 360at | Défense du Grenier (Ambroise) |
| 361at | Inauguration du Verrou |
| ≈ 365at | Émeutes de la Faim (système Vorrh) |
| 380at | Relance du chantier de Vorrh-V par l'impératrice Odessa Vray |
| 383at | Grève des Puits (La Ceinture) |
| ≈ 385at | Raid des Écorcheurs sur Port-Vaillant (un porte-vaisseaux volé en construction) ; naissance de Nyra Maren |
| 391at | Hakkon Skeld prend l'Arène et devient Chancelier |
| 398at | Mort de l'amiral Doran Maren (80 ans) |
| 399at | Retraite de Verian Maren ; le VIV Alarik promis au démembrement |
| **400at** | Corvan Sarkis prend le pouvoir et fait tuer les anciens sénateurs ; Vorrh-V stoppé ; **début de la Renaissance** ; les Golden Sword engagés |
| 401at | Capture de Jaro Fesk sur Rocaille par la Flotte Beta |
| 403at | **Piège des Écorcheurs** ; Tosear prend Terminus ; 🔒 pacte de Terminus |
| 405at | Trahison des Golden Sword ; 🔒 fondation de la colonie de Vesper ; création des Flottes d'Alcyone et de Port-Vaillant ; vers cette date, prise éclair de Promesse par Rajik |
| Renaissance | Annexion du Royaume de Primaterre (roi assassiné par les Hashashin) ; attaque de Promesse, rasée par la 1ère Flotte (date fine ❓) |
| **409at** | Grande contre-offensive de l'UGL, brisée : **batailles de Ferrogane, d'Ostrag et de la Trouée** |
| 409-414at | Commission d'enquête unioniste sur la débâcle d'Ostrag |
| ≈ 410at | Sarkis suspend ses conquêtes ; les deux camps détruisent les Portails qui les reliaient : **naissance du No Man's Land et de la Fausse Paix armée** ; 🔒 La Fédération lance sa Flotte d'Attaque |
| 411at | Élection de Nyra Maren (26 ans) |
| 414at | Promotion d'Edran Kole et d'Elara Venn ; rétrogradation de Vasco Reth ; destitution de Maro Fenn |
| ≈ 418at | Torg Varga devient Roi des pirates |
| 419at | Réélection de Nyra Maren |
| 420at | Sortie du NSU Liberté (le cuirassé le plus puissant jamais construit) ; 🔒 achèvement du FED Suprématie |
| 424at | Retour en service du NSU Ferveur |
| **425at** | **Aujourd'hui** |
| *À venir* | ≈ 426at : prochaine Arène fédérale (déduction) · 427at : fin du 2e mandat de Nyra Maren · 430at : 🔒 sorties prévues à Hargan · 430-440at : 🔒 Flotte d'Attaque opérationnelle · 440at : 🔒 FED Dernier-Mot |

### 8.3 Batailles et événements militaires

| Événement | Date | Lieu | Camps | Issue |
|---|---|---|---|---|
| La Nuit Silencieuse | 53bt | Tout le territoire de l'Union | Essaim contre Nations Unies | Un tiers des systèmes pris en une nuit ; ≈ 60 % de la flotte de l'Union détruite (concentrée dans un exercice géant ou à quai) ; les flottes du Renouveau (25 %) épargnées |
| Destruction de Terra | ≈ 50bt | Système Solaire (L8) | Essaim | Terra et toutes les planètes du système anéanties pour briser le moral humain |
| Destruction de Jask | Grande Guerre | ❓ | Flotte des NU contre un bastion de l'Essaim | Jask détruit |
| **Le Triomphe** | 0at | Espérance (J9) | Flottille d'Avrel Hex, flotte du Renouveau Impérial, contre la Reine | Six frégates perdues dans la percée ; la Résolue touchée ; la Délivrance, la Sentence et l'Aurore plongent chargées d'antimatière : Espérance et la Reine détruites, 90 % des drones désactivés |
| Attaque de Franchise | 12at | Station-prison (K4) | 1ère Flotte de l'époque contre les révoltés | Station ravagée, moteur détruit, population en majorité tuée ; l'Empire croit tout le monde mort et passe à autre chose |
| Libération de Concordia | 357-359at | Système de Concordia (K7) | Flotte de la Libération (Doran Maren) contre les impériaux loyaux | Victoire de l'UGL au prix d'une bonne partie de la flotte |
| Défense du Grenier | ≈ 360at | Système Ambroise (K9) | Flotte de défense de Cygnara contre une force impériale | L'Empire repoussé ; surnom « la sauveuse du grenier » |
| Raid sur Port-Vaillant | ≈ 385at | Port-Vaillant (K5) | Flotte des Écorcheurs | Un porte-vaisseaux volé en construction (🔒 devenu l'OXL Spectre) |
| Piège des Écorcheurs | 403at | Point de rendez-vous d'une fausse cargaison, hors de Terminus | 1ère Flotte (+ Tosear) contre les Écorcheurs | Tous les vaisseaux et capitaines du gang détruits ; triomphe de propagande pour l'Empire |
| Prise éclair de Promesse | ≈ 405at | Promesse (M5) | Meute de Rajik contre l'escadre impériale | Escadre détruite ; Rajik tient la planète quelques jours et repart les cales pleines : « Le Conquérant » |
| Attaque de Promesse | Renaissance (campagnes de 409at) | Promesse (M5), reprise peu avant par l'UGL lors de sa contre-offensive (« prise initiale », où le croiseur Étincelle fut lourdement endommagé) | 1ère Flotte contre la Flotte de la Libération (Edran Kole alors sous-amiral) | Planète rasée (bombardements orbitaux, villes vitrifiées) : Promesse devient Cendres. La Libération tient bon face à une flotte largement supérieure et couvre les évacuations, au prix du Havre durement touché, de frégates et de presque tous les transports du Tête-de-Pont. L'UGL y perd une grande usine d'armes (d'où le contrat CLPE avec Hexa-Corp). Crime de guerre pour l'UGL, « exemple nécessaire » pour l'Empire |
| **Bataille de Ferrogane** | 409at | Ferrogane, système Ferrok (O7) | 1ère Flotte contre la Flotte Écarlate et la Flotte des Fantômes Blancs | Victoire impériale totale : les deux flottes anéanties (Carmin avec Vessa Karn, Albâtre avec Telmo Brac, Grenat, Rubis, Incarnat, Pourpre, Cinabre, Ivoire, Givre, Linceul). L'Empire perd le Conquérant et trois croiseurs en construction ; Fauconnier, Gerfaut, Sabre, Fléau, Hallebarde endommagés |
| **Bataille d'Ostrag** (« la débâcle d'Ostrag » côté UGL) | 409at | Système Ostrag (O5) | 2ème Flotte (Neva Sorren) contre les flottes de la Libération, du Concorde et de l'Aube (la plus grosse attaque de l'histoire de l'UGL, pour libérer les prisonniers et détruire le bagne) | Victoire impériale contre toute attente, après des semaines de combats. UGL : Colosse (le plus gros PV de l'époque, abordé, moteur mis en surcharge), Convoyeur, Pionnier, Bâtisseur, Horizon, Égide, Farouche, Indomptable, Insolent, **Serment** (charge suicide qui éventra le Grand-Duc), Union et Affranchi (pris à l'abordage), Constance, Loyal, Éclipse, ≈ 25 frégates ; Solvet tué sur le Zénith en couvrant la retraite du Concorde. Empire : Intrépide, 5 croiseurs, un PV, Grand-Duc condamné : la majorité de la 2ème Flotte. Les prisonniers ne sont pas libérés |
| **Bataille de la Trouée** | 409at | Espace profond, sur la route des renforts UGL vers Ferrok | 3ème Flotte (Joss Vek) contre les flottes d'Alcyone et de Port-Vaillant | Victoire écrasante : Dévouement sacrifié, 27 frégates et ≈ 80 corvettes perdues ; seuls le Prompt-Secours et le Charpentier s'enfuient. Sans cette interception, la 1ère aurait sûrement perdu Ferrogane, et l'Empire la guerre |
| Défaite de la Garde dorée | Date ❓ | Nord de la Bordure | Meute de Raeilla contre la Garde dorée | GRT Bannière détruit, GRT Sceptre capturé ; tribut grothul |
| La Faucheuse fauchée | Date ❓ | Nord de la Bordure | Meute de Raeilla contre la Flotte Nordique | OXL Moissonneuse détruit ; seule défaite de la Flotte Nordique |
| Coup d'État pirate | ≈ 418at | No Man's Land | Torg Varga contre l'ancien Roi des pirates | Flotte de l'ancien Roi anéantie en une nuit ; Circa récompensé de la Récompense |

Autres faits d'armes connus, sans date : le vol du Trophée aux Grothuls par Tosear ; le vol du Butin lors d'un raid Tosear sur une base des Golden Sword ; la capture de l'Inachevé à Port-Vaillant par Raeilla ; le vol du croiseur Dépeceur à l'Empire par les Écorcheurs ; la capture du Prise par la Fédération sur des pirates.

### 8.4 Révoltes et crises de l'ère impériale (une par règne)

| Date | Événement | Règne | Nature | Issue |
|---|---|---|---|---|
| 39at | Sécession des Trois Systèmes | Kaelen | Trois systèmes périphériques gouvernés par d'anciens cadres des NU proclament leur sécession au nom de l'héritage démocratique | Écrasée en une campagne |
| 85at | Révolte des Oubliés | Serena | Des mondes découvrent qu'ils sont rayés à jamais du programme de reconnexion des Portails | Réprimée par l'abandon : l'Empire ne fit rien |
| 110at | Fronde de l'Écu | Tavian | Des provinces marchandes refusent la conversion forcée de l'écu en crédit | Siège économique, sans un coup de canon ; mort définitive de l'écu |
| 150at | Conjuration des Faucons | Maribel | Complot de sénateurs militaristes contre « l'impératrice qui parle aux exilés » | Déjouée par son propre renseignement ; purge discrète |
| 172at | Mutinerie de la Vieille Flotte | Toron | Les équipages des vieilles coques de la Grande Guerre (des familles à bord depuis trois générations) refusent la casse | Matée ; une partie des mutins s'enfuit vers la Bordure |
| 215at | Grève du Marbre | Ysolde | Les ouvriers des chantiers du culte impérial sur Concordia posent les outils | Réprimée « sans un mort », par un spectacle |
| **241at** | **Soulèvement des Espèces** | Danthe | Plusieurs espèces se soulèvent de façon coordonnée sur une demi-douzaine de mondes (quotas, zones réservées, métiers interdits) : la plus grande révolte de l'ère | Écrasé sans procès ; plus aucune action commune des non-humains pendant un siècle |
| 280at | Procession interdite | Alarik II | Un culte populaire divinise Alarik le Fondateur : une religion, l'hérésie suprême | Culte dissous par la force |
| 315at | Émeutes du Chantier | Vessian | Les millions d'ouvriers réquisitionnés pour Vorrh-V brûlent leurs cités-dortoirs | Matées, puis conditions améliorées |
| ≈ 365at | Émeutes de la Faim | Halvec Sorn | Rationnement sur les mondes-outils du système Vorrh (Vorrh-VI en tête : mineurs et cadets de l'académie) | Matées, puis apaisées par les décrets d'autosuffisance |
| 383at | Grève des Puits | Odessa Vray | Les mineurs de La Ceinture cessent l'extraction, sans émeute : « la grève la plus polie de l'Histoire » | Réglée sans bruit |

### 8.5 Les quinze empereurs

| # | Empereur·rice | Règne | Surnom et marque |
|---|---|---|---|
| 1 | **Alarik Vorrhen** | 0at – 24at | Le Fondateur : la meilleure des reconstructions |
| 2 | Kaelen Vorrhen | 24at – 71at | Le Continuateur : l'œuvre du père, sans génie et sans faute |
| 3 | Serena Vorrhen | 71at – 102at | La Réparatrice : première impératrice, relance des Portails |
| 4 | Tavian Vorrhen | 102at – 131at | Le Comptable : le crédit impérial achevé |
| 5 | Maribel Vorrhen | 131at – 160at | La Diplomate : la paix armée avec la Bordure |
| 6 | Toron Vorrhen | 160at – 201at | Le Cuirassier : refonte de la flotte, Vorrh-II spécialisée |
| 7 | Ysolde Vorrhen | 201at – 224at | La Splendide : apogée du culte impérial |
| 8 | Danthe Vorrhen | 224at – 267at | Le Sévère : la vis serrée sur les espèces |
| 9 | Alarik II Vorrhen | 267at – 299at | L'Héritier du nom : la nostalgie fondatrice érigée en politique |
| 10 | Vessian Vorrhen | 299at – 341at | Le Bâtisseur d'étoiles : Vorrh-V (309at), 🔒 l'EESS (320at) |
| 11 | Maldric Vorrhen ✝ | 341at – 357at | L'Empoisonné : le dernier Vorrhen, mort au Conseil |
| 12 | Halvec Sorn | 357at – 372at | L'Empereur de l'exil : Vorrhal tenue, l'Empire sauvé de l'anéantissement |
| 13 | Odessa Vray | 372at – 388at | La Gardienne : économie de survie, relance de Vorrh-V (380at) |
| 14 | Rys Kalder | 388at – 400at | L'Indécis : le trône qui attendait un prédateur |
| 15 | **Corvan Sarkis** | 400at – aujourd'hui | Le Renaissant : la purge du sénat, la reconquête |

Règle de succession : chaque empereur est « élu à vie » par un sénat qu'il a lui-même composé, qui choisit presque toujours l'héritier Vorrhen (onze Vorrhen de 0at à 357at) ; trois empereurs d'exil furent élus par le sénat réfugié sur Vorrhal ; Sarkis ne doit son trône qu'à lui-même.

---

## 9. État du monde en 425at

### 9.1 Synthèse

- **La Fausse Paix armée dure depuis 15 ans.** Personne ne l'appelle « la paix » : tout le monde sait que ce n'est qu'un entracte.
- **L'Empire** digère ses conquêtes depuis Vorrhal, impose le Wuro, tient Primaterre par la terreur et reconstitue ses flottes : la 1ère est en réparation à Vorrh, la 2ème se reconstruit à Ferrok, la 3ème (Karza) est la seule pleinement opérationnelle. Les chantiers de Ferrok tournent à plein depuis quelques années. Son but n'a pas changé : écraser l'UGL et détruire Concordia.
- **L'UGL** reconstruit une économie ruinée (Cygnara produit des transports en masse) derrière un dispositif défensif massé au No Man's Land. Ses FOU renaissent autour du NSU Liberté, de l'Arsenal et des nouveaux croiseurs ; Le Berceau et Rempart sont en chantier. Les pirates remontent jusqu'à Cygnara et Ambroise. La Présidente Nyra Maren entame la fin de son deuxième mandat.
- **La Fédération** sort lentement de quatre siècles d'isolement par la diplomatie. 🔒 Sa Flotte d'Attaque est à moitié construite.
- **Hexa-Corp** n'a jamais été aussi riche ; L'Enclume avance vite ; les deux capitales supportent de plus en plus mal qu'elle arme l'ennemi.
- **Le No Man's Land** est la zone la plus active : guerre Tosear contre Ligue, contrebande, camps de Cendres, raids depuis le Croissant, diplomatie et espionnage à Hexis.
- **La Bordure** : guerre larvée Grothul contre Tosear (Vantra), « Grande Faucheuse » au nord, tribut grothul à Raeilla, occupation de Primaterre, Franchise déchirée entre Frères de l'Aube et Hashashin, royaumes neutres qui se savent des proies.
- **La Zone Inexplorée** : 425 ans de silence. 🔒 L'Essaim s'y reconstruit autour d'une nouvelle Reine, pendant que l'EESS impériale (au nord) et la FSS unioniste (Vesper, le Méridien) l'explorent chacune en secret, sans connaître l'existence de l'autre.

### 9.2 Relations entre acteurs

| Acteurs | Relation publique ou connue | 🔒 Réalité cachée |
|---|---|---|
| Empire ↔ UGL | Guerre froide, aucun Portail commun, flottes massées de part et d'autre du No Man's Land ; diplomatie de l'ombre à Hexis | Chacun explore la Zone Inexplorée en secret, en soupçonnant l'autre |
| Empire ↔ Ligue des Pirates | Ennemis de principe ; la Ligue épargne curieusement les convois impériaux | L'Empire la finance et possède la carte des passes du Croissant |
| Empire ↔ Syndicat Tosear | Le Syndicat est traqué par l'Empire | Pacte de Terminus ; Tosear fournit des informations à l'Empire ; d'autres liens existent |
| Empire ↔ Hexa-Corp | Client majeur (missiles, Escadre de Primaterre) ; agacement croissant | — |
| UGL ↔ Hexa-Corp | Client majeur (armes d'infanterie, flotte de Concordia) ; inquiétude croissante | — |
| UGL ↔ Sœurs de Myrth | Dette reconnue, distance officielle | Protection de Myrth en sous-main |
| Empire ↔ Sœurs de Myrth | Haine ; magie interdite et chassée ; aucune représaille à ce jour | La vengeance de Sarkis se prépare |
| UGL ↔ Ligue des Pirates | Raids sur les convois et les mondes frontaliers | (financés par l'Empire) |
| Syndicat Tosear ↔ Ligue | Guerre ouverte, le conflit le plus actif du No Man's Land | — |
| Syndicat Tosear ↔ Royaume Grothul | Rivalité commerciale et criminelle (Vantra, le Trophée volé) | La Flotte Nordique de Tosear pille les convois grothul |
| Royaume Grothul ↔ Raeilla | Tribut payé pour que les convois passent | — |
| Raeilla ↔ Flotte Nordique | La « faucheuse » change de route quand les Sans-Couronne apparaissent | Tosear enrage mais ne peut venger la Moissonneuse sans se dévoiler |
| Royaume Grothul ↔ Hexa-Corp | Client majeur (mur de Skarn, Gardiens de Vantra, renseignement) | — |
| Royaume Grothul ↔ Empire | Méfiance armée à Skarn ; probables échanges d'informations par intermédiaires | — |
| Royaume Grothul ↔ Mercenaires de Cendres | Garnisons au sol de tout le royaume | — |
| Franchise ↔ Empire | L'Empire croit la station morte depuis 12at | — |
| Frères de l'Aube ↔ Hashashin | Haine ; raids sanglants des Frères contre le sanctuaire de Franchise | Arven le Doux est un Frère |
| Frères de l'Aube ↔ Syndicat Tosear | Réprimés dans le sang à Terminus | — |
| Empire ↔ Primaterre | Occupation ; révoltes réprimées | La Guilde Marchande relève Primaterre par les réseaux Tosear |
| Empire ↔ Golden Sword | Trahison de 405at ; vétérans au bagne ou enrôlés de force | — |
| Empire ↔ Hashashin | Sarkis a « parrainé une offrande » (un général) pour faire assassiner le roi de Primaterre | Les Hashashin sont des Vashyr ; le général vit, esclave sur Vashyra |
| La Fédération ↔ le reste du monde | Isolationnisme ; commerce et diplomatie via Sabraze ; chasse aux pirates ; « puissance régionale » | Flotte d'Attaque ; ouverture diplomatique = préparation |
| UGL ↔ Bordure | Courtise les neutres ; des royaumes extérieurs ont aidé la Rébellion | — |
| Hexa-Corp ↔ Syndicat Tosear | Client (le Tribut, acheté au grand jour) | Coques de la Flotte Nordique effacées des registres d'Hexis |
| Hexa-Corp ↔ Mercenaires de Cendres | Rivaux sur le marché du mercenariat au sol | — |
| Ligue des Pirates ↔ Mercenaires de Cendres | Garnisons au sol de Rade-Noire | — |
| L'Essaim ↔ tous | Silence depuis 425 ans ; terreur du passé | Une nouvelle Reine reconstruit l'Essaim |

### 9.3 Projets, chantiers et échéances connues

| Projet | Entité | Lieu | État ou échéance |
|---|---|---|---|
| L'Enclume, chantier spatial de cuirassés | Hexa-Corp | Hexis (M7) | En construction, de plus en plus vite ; aucune date |
| Le Berceau, chantier spatial de croiseurs | UGL | Cygnara (L8) | En construction, sous la garde de la Flotte de la Libération |
| Rempart, station de défense | UGL | Ambroise (K9) | En construction ; avancement à préciser dans la Trame Actuelle |
| Liste d'attente des commandes CCV | Hexa-Corp | Hexis | En cours |
| Flotte d'Attaque | 🔒 Fédération | Hargan | Hégémonie, Ouragan, Tourmente, Verdict, Anathème, Interdit d'ici **430at** ; Dernier-Mot d'ici **440at** ; flotte opérationnelle vers **430-440at** |
| Réparations UGL | UGL | Calderis (Insurgé, Concorde), Port-Vaillant (Concordia, Opiniâtre), ailleurs (Havre, Zénith, Étincelle, Glaneur, plus de vingt frégates) | Aucune date ; le Tête-de-Pont attend ses nouveaux transports |
| Réparations impériales | Empire | Vorrh (1ère Flotte), Ferrok (Bastion pour longtemps, Alarik, Gerfaut, Ténace, Sabre), Karza (Coutelas, Merlin), Solferra (le Mandataire) | Aucune date |
| Démantèlements | Empire / UGL | VIV Grand-Duc (pièces), NSU Généreux (désossé à quai de Concordia) | Prévus |
| Vorrh-V | Empire | Système Vorrh | Suspendu depuis 400at, sous garnison |
| Échéances politiques | Fédération / UGL | Tharkast / Concordia | Prochaine Arène vers 426at (déduction du cycle de cinq ans) ; fin du second mandat de Nyra Maren en 427at (un troisième et dernier possible, jusqu'en 435at) |

### 9.4 Lignes de tension actives (faits canon)

- La **neutralité d'Hexa-Corp** est de moins en moins acceptée par les deux capitales ; L'Enclume en changerait l'échelle.
- Les **flottes impériales sont diminuées** (1ère en réparation, 2ème exsangue) : toute offensive reposerait sur la 3ème. Côté UGL, la Libération et l'Aube sont les seules FOU opérationnelles, et le Liberté n'a jamais été éprouvé au combat.
- **Cygnara** dépend des convois alimentaires ; **Ambroise** subit des raids sans avoir fini Rempart ; **Bastia** dépend des convois de la capitale.
- **Primaterre** se révolte encore ; la dernière révolte a endommagé le Mandataire.
- **Ostrag** est une poudrière : vétérans Golden Sword, loyalistes de Primaterre, prisonniers unionistes, tenus par l'isolement et la faim.
- **Franchise** saigne des raids des Frères contre le sanctuaire hashashin, et vit de l'oubli de l'Empire. À **Terminus**, les Frères (≈ 25 %) prient en silence et attendent.
- **Vantra** est un front quotidien entre Grothul et Tosear (entrepôts incendiés, dockers débauchés, cargaisons détournées).
- **Le nord de la Bordure** appartient à Raeilla et à la « Grande Faucheuse ».
- **Cendres** recrute pour tout le monde, et chaque puissance accuse l'autre d'avoir créé ce monde sans loi.
- **Politique unioniste** : le cumul de Teo Malvin (gouverneur et marchand) attire l'œil de certains sénateurs ; Vasco Reth sert sous l'homme promu à sa place ; Elara Venn veut faire ses preuves.
- **Politique impériale** : la 2ème Flotte porte l'accusation injuste d'avoir « abandonné » Ferrogane.
- **La guerre des monnaies** : crédit contre Wuro, marchés noirs, taux d'Hexis.
- **La Fédération** s'ouvre à la diplomatie extérieure, et personne, dehors, ne se demande encore pourquoi.
- **La Zone Inexplorée** : deux programmes secrets y progressent, chacun ignorant l'autre, dans la zone où l'Essaim se reconstruit.

---

## 10. Récapitulatif des secrets d'univers 🔒

*Vrais dans le monde ; ne jamais les révéler aux personnages ou aux joueurs sans décision de l'auteur.*

| # | Secret | Qui sait | Qui l'ignore |
|---|---|---|---|
| 1 | **Une nouvelle Reine a émergé** : l'Essaim se reconstruit lentement dans la Zone Inexplorée (l'état de ses forces est inconnu, même de l'auteur) | Personne | Tout le monde, y compris l'EESS et la FSS qui explorent la Zone |
| 2 | **Programme secret d'exploration de l'UGL** ; colonie de **Vesper** (I10, 405at) et sa voisine **Verdine** ; la **FSS** ; le **NSU Lumières** n'a pas été détruit | Sommet de l'UGL (la Présidente), FSS, gouvernement spécial de Vesper | L'Empire (qui a des doutes, sans preuve), le public |
| 3 | **L'EESS** : l'Empire explore la Zone depuis 320at (VIV Théorème et Corollaire, déguisés) ; elle est au nord de la Zone | Sommet impérial, EESS | L'UGL, le public |
| 4 | **L'Empire finance la Ligue des Pirates** | Empire | UGL, Tosear, public ; Torg Varga ❓ |
| 5 | **Le pacte de Terminus** (403at) : l'Empire s'est engagé, uniquement, à ne pas détruire Terminus pour les crimes des Écorcheurs ; Tosear fournit des informations à l'Empire ; d'autres liens Tosear-Empire et Tosear-UGL existent | Empire, clan Tosear | Les autres |
| 6 | L'Empire possède une **carte des passes de la nébuleuse du Croissant** | Empire (EESS) | Ligue des Pirates |
| 7 | L'Empire emploie des **Ethari** en secret, contre sa propre doctrine | Empire | Public |
| 8 | La **vengeance de Sarkis contre Myrth** se prépare | Sarkis et ses proches | Les autres |
| 9 | **La Fédération construit la Flotte d'Attaque** depuis 410at ; ses chiffres réels (4 cuirassés, 4 PV, 10 croiseurs en 425at ; 5, 6 et 15 d'ici 430at) ; son ouverture diplomatique est une préparation | Direction fédérale | UGL, Empire, Bordure |
| 10 | **La Flotte Nordique (OXL) appartient au Syndicat Tosear** ; Kordan Vail est payé par Tosear ; l'OXL Spectre est le porte-vaisseaux volé à Port-Vaillant | Clan Tosear (Hexa-Corp a vendu des coques ensuite effacées de ses registres) | Grothuls, Empire, UGL, Bordure |
| 11 | **Les Hashashin sont les Vashyr** : imitation parfaite ; offrandes réduites en esclavage sur Vashyra ; le général offert par Sarkis vit, esclave ; les gardes des sanctuaires sont peut-être des Vashyr ; Vashyra est une société tribale, cruelle, en guerre de clans | Les Vashyr | Tout le monde, Sarkis compris |
| 12 | **Les Sœurs de Myrth** : ≈ 100 Sœurs et ≈ 400 apprenties enlevées enfants ; sélection d'une sur dix ; la **matriarche Benedict** dirige depuis 178at et a ordonné le Conseil empoisonné | Les Sœurs | Tout le monde |
| 13 | **La Guilde Marchande de Primaterre** contourne l'Empire par les réseaux Tosear pour relever Primaterre | La Guilde, Tosear | L'Empire, la population |
| 14 | **Arven le Doux est un Frère de l'Aube** (sa règle « qui se rend vit » est une doctrine) | Arven | Tout le monde |

**Mystères que l'auteur n'a pas encore tranchés** (ne pas les résoudre sans lui) : les « moyens flous » de Sarkis ; Torg Varga sait-il que l'Empire finance la Ligue ? ; la vérité sur la cache des Golden Sword à Rocaille ; ce que la Fédération veut faire de sa Flotte d'Attaque.

---

## 11. Zones blanches : ce qui n'est pas encore défini

### 11.1 Vides connus (à proposer, jamais à affirmer)

- **Univers et technologie** : nature du carburant lourd et des composants radioactifs (origine, gisements, contrôle) ; autonomie type par classe de vaisseau ; détails techniques (armes, blindages, détection, communications, IA, médecine) ; vaisseaux expérimentaux (dossier prévu, vide) ; taux de change et coût de la vie ; autres traditions magiques et cas de « magie volée » ; armées au sol des puissances (structure, effectifs).
- **Géographie** : l'univers doit grossir de 2 à 3 fois (futurs royaumes des marches nord et sud, nouveaux systèmes) ; étoiles de Cygnara, d'Hexis, de Vesper (et éventuellement d'Alcyone et de Bastia) sans nom ; ordre orbital de la plupart des corps ; autres corps du système Ferrok ; systèmes voisins nourriciers de Cygnara ; position de Jask ; systèmes abandonnés, cimetières de flottes, autres nébuleuses, ceintures minières disputées (aucun encore placé) ; découpage en régions (dossier prévu, vide) ; origine des planètes errantes ; collecteurs orbitaux d'Hélion ; quartiers, villes et spatioports de Concordia et d'Hexis (traces de l'ère impériale sur Concordia) ; géographie et atmosphère de Myrth (esthétique sombre envisagée) ; habitat de Port-Vaillant ; installations de Crépuscule et de La Ceinture ; exploitation de Vorrh-IV ; lieu où la Résolue est exposée.
- **Factions et personnages** : figures du Sénat des Mondes et du Conseil des Espèces ; tensions internes de l'UGL ; entourage, âge, apparence, héritier et passé de Sarkis avant 400at ; dirigeants et date de fondation du Royaume Grothul ; conseil et lieutenants du clan Tosear ; président·e de Franchise ; dirigeants et effectifs des Mercenaires de Cendres ; maison royale et héritier de Primaterre ; nom du général offert aux Hashashin ; clans de Vashyra et apparence véritable des Vashyr ; espèces non humaines de la Fédération ; flottes des Nations Unies (noms) et de l'Essaim (composition) ; croiseurs amiraux des EDS (non nommés) ; flotte de garde d'Ostrag ; fondateur du Renouveau Impérial ; physiologie des Grothul et des Tosear ; origine des Ethari ; espèces disparues pendant la Grande Guerre ; rôle des Saluviens pendant la Grande Guerre et la Rébellion.
- **Histoire** : unification de Terra et premières colonies ; premier vol interstellaire et premiers Portails ; le conseil d'intégration de l'Essaim (qui a voté quoi), ses chantiers secrets et les lanceurs d'alerte ignorés ; grandes batailles de la Grande Guerre, chute de Vorrhal, campagnes des Sœurs, traque de la Reine ; le Conseil empoisonné en détail (le poison, les Sœurs présentes, leur fuite) et les ralliements décisifs ; la fuite du sénat ; les campagnes de la Renaissance et les mondes passés de main en main ; date de l'annexion de Primaterre ; chronologie fine de Promesse.

### 11.2 Choix d'auteur à ne jamais contredire

- **L'origine de l'Essaim est un mystère définitif** : personne ne l'a jamais su, personne ne le saura. Ne pas l'expliquer.
- **Aucune désassimilation n'a jamais réussi** (folie, mort ou drone toujours actif) ; la recherche est interdite par les deux puissances.
- **Le trou noir central est ordinaire**, sans mystère.
- **Myrth n'a aucun lien avec la magie** : c'est un refuge, pas une source.
- **Les Frères de l'Aube n'ont aucun lien réel avec l'Essaim** (coïncidence exploitée).
- **Aucun bouclier énergétique** : seul le blindage protège.
- **Aucun vaisseau capitale n'a jamais existé.**
- **La Zone Inexplorée n'a aucune barrière physique** : seulement le silence et l'absence de routes.
- **Sarkis n'a pas de pacte secret avec Hexa-Corp** : les services d'Hexa-Corp étaient publics et ouverts aux deux camps.
- **Hexa-Corp ne cache rien** : servir tout le monde au grand jour est ce qui la rend inattaquable ; deux contrats Hexa-Corp ne s'affrontent jamais.
- **Les idées de `99-Idees/` ne sont pas canon** et ne doivent jamais être citées comme des faits.

---

## 12. Incohérences relevées dans le wiki (à arbitrer par l'auteur)

1. **Le Verrou** : la fiche dit « quatre-vingt-quatre ans plus tard, la station sert toujours », mais de 361at à 425at il s'écoule **64 ans**.
2. **Défense du Grenier** : la fiche situe le système Ambroise en « **M8** » ; partout ailleurs (carte et fiche du système), Ambroise est en **K9**.
3. **Chantiers navals** : la fiche d'ensemble parle d'un quasi-monopole des cuirassés partagé entre Calderis et Vorrh-II, et dit que L'Enclume ferait d'Hexa-Corp « la troisième puissance navale ». Or Hargan (La Fédération) construit déjà des cuirassés (« l'un des trois seuls sites ») et La Fédération est « la troisième puissance de la galaxie ». On peut le lire comme le point de vue erroné des personnages, mais la fiche ne mentionne pas Hargan. Elle omet aussi que les chantiers d'Hexis ont produit au moins un porte-vaisseaux (le Donjon).
4. **Calderis** : sa fiche évoque « sa FDU locale », mais la liste des FDU de l'UGL n'en compte aucune pour Calderis (c'est la Flotte de l'Aube, une FOU, qui y stationne).
5. **Système Vorrh** : `carte-galactique.md` parle d'un « système de mondes désertiques militarisés » et `vorrhal.md` demande le « nombre exact de mondes désertiques », alors que le système détaillé compte une planète de feu, une planète-forêt, un monde minier, des chantiers et une ceinture.
6. **Promesse / Cendres** : une étape manque. Promesse est impériale avant 357at, puis passe à l'UGL ; elle est « sous contrôle de l'Empire » lors du raid de Rajik (≈ 405at) ; l'UGL la reprend lors de sa contre-offensive (« prise initiale ») avant que la 1ère Flotte ne la rase. La reconquête impériale entre 357at et ≈ 405at n'est décrite nulle part (implicitement pendant la Renaissance).
7. **Dessin de la carte** : certains corps sont tracés dans une case voisine de leur case canon (Tharkast, Molvrek, Hargan, Ignar et Ulveth en K11 au lieu de J11 ; Aldène en M8, Bruma en L9 et Le Berceau en M9 au lieu de L8) parce que les contours de système débordent. Le contour du territoire fédéral couvre ≈ J10 → L12. La case canon fait foi.
8. **Dossier fantôme** : `INDEX.md` et `carte-galactique.md` renvoient à un sous-dossier `01-Galaxie/Regions/` qui n'existe pas.
9. **Convention de position** (règle 7 du wiki : la case doit figurer dans l'en-tête) : les en-têtes de Concordia, Vorrhal, Terra et Myrth n'ont pas de champ « Position » (la case n'apparaît que dans le corps de la fiche) ; celui de Vorrh-II n'indique que « Système : Vorrh », sans case ; Hexis utilise « Localisation » ; Cygnara, Ferrogane, Karvek et Vorrh-V donnent la case dans le champ « Système ».

---

## 13. Glossaire et sigles

**Notation** : `bt` / `at` = avant / après le Triomphe (an 0) · 🔒 secret d'univers · ✝ détruit ou mort · ❓ non défini.

**Entités et institutions** : **NU** = Nations Unies (l'ancienne Union, 600bt → 0at) · **UGL** = Union Galactique Libre (« l'Union ») · **FOU** = Flottes Officielles de l'Union · **FDU** = Flottes Décentralisées de l'Union · **FSS** = Flotte Scientifique Secrète (UGL) 🔒 · **EESS** = Escadrille d'Explorations Scientifiques Secrète (Empire) 🔒 · **EDS** = Escadres de Défense des Systèmes (Empire) · **FDM** = Flottilles de Défense des Mondes (Empire) · **FC** = Flottes de la Corporation (Hexa-Corp) · Sénat des Mondes, Conseil des Espèces (UGL) · le Chancelier, le Parti, l'Arène (Fédération) · le directoire (Hexa-Corp) · le clan (Tosear) · la couronne (Grothul) · les meutes, le Roi des pirates (Ligue).

**Préfixes de vaisseaux** : VIV (Empire) · NSU (UGL) · FED (Fédération) · HC (Hexa-Corp) · TOS (Tosear) · OXL (faux pavillon de la Flotte Nordique 🔒) · GRT (Grothul) · ECR (Écorcheurs ✝) · UNS (Nations Unies, proposition non validée) · aucun préfixe pour les vaisseaux loués d'Hexa-Corp et les pirates. **PV** = porte-vaisseaux.

**Contrats d'Hexa-Corp** : CCV construction · CRV réparation · CFM flotte de mercenariat · CVLM vaisseau loué avec équipage · CVL vaisseau loué sans équipage · CE équipage · CM mercenariat au sol · CVV vaisseau de vente · CAEE achat d'équipement de personnel · CLPE location d'équipement de personnel · CAAV achat d'armement de vaisseau · CLAV location d'armement de vaisseau · CRS remorquage · CR ravitaillement · CT transport · CMS minage spatial · CAV assurance de flotte · CC crédit · CRD rachat de dette · CCG crédits de guerre · CH hypothèque.

**Termes du monde** :
- **Astréa** : la galaxie. **Le noyau** : son trou noir central.
- **Le Triomphe** : la victoire de 0at sur la Reine de l'Essaim. **La Nuit Silencieuse** : l'attaque surprise de 53bt. **Le Conseil empoisonné** : l'assassinat collectif de 357at. **La Rébellion**, **l'Empire réduit**, **la Renaissance impériale**, **la Fausse Paix armée** : les ères récentes. **La débâcle d'Ostrag** : nom officieux de la bataille d'Ostrag côté UGL.
- **Portail** : passage quasi instantané ; **hub** (multi-destinations) ou **paire fixe**. **Hyperpropulsion / hyperespace** : voyage lent par ses propres moyens.
- **No Man's Land** : la zone tampon entre les blocs. **Zone Inexplorée** : la moitié ouest jamais explorée. **Bordure Extérieure** : les confins hors des blocs ; **marches nord / sud**. **Systèmes abandonnés** : mondes jamais rebranchés aux Portails. **Mondes-outils** : les mondes numérotés du système Vorrh.
- **Le Standard** : langue universelle. **Écu**, **crédit**, **Wuro** : les monnaies ; **taux d'Hexis** : le change officieux.
- **« Libres mais non citoyennes »** : statut des espèces non humaines dans l'Empire.
- **Drone**, **la Reine**, **assimilation** : vocabulaire de l'Essaim ; *Ruches* et *Nuées* : vocabulaire humain proposé, non validé.
- **La Grande Nuit** : l'apocalypse des Frères de l'Aube. **Le Silencieux** : le dieu des Hashashin ; **parrainer une offrande** : commander un assassinat.
- **Vaisseau capitale / vaisseau-mère** : classe théorique de plus de 10 km ; **le *Souverain*** : celui qui n'exista jamais (« attendre le Souverain » se dit d'un projet grandiose qui ne viendra jamais).
- **« Finir à Ostrag »** : la peine impériale par excellence.

---

## Annexe A — Registre des vaisseaux nommés (158 navires)

*Généré depuis les fiches `04-Vaisseaux/`. Classes : cuirassés, porte-vaisseaux, croiseurs, navires scientifiques et frégates nommées. Les flottes et les devises sont détaillées en section 6. Aucune corvette n'a encore gagné de nom.*

#### Empire Vorrhen (VIV) — 30 navires

| Nom | Classe | Flotte / rattachement | État (425at) |
|---|---|---|---|
| VIV Alarik | Cuirassé | 2ème Flotte « Le Dernier Rempart » | en réparation (Ostrag) — « un miracle qu'il fonctionne encore » |
| VIV Apogée | Cuirassé | 3ème Flotte « Les Semeurs de Mondes » — navire-amiral | opérationnel |
| VIV Bastion | Cuirassé | 2ème Flotte « Le Dernier Rempart » — navire-amiral | très endommagé, longue réparation (Ferrok) |
| VIV Conquérant ✝ | Cuirassé | 1ère Flotte « L'Invincible » — ancien navire-amiral | détruit à la bataille de Ferrogane |
| VIV Intrépide ✝ | Cuirassé | 2ème Flotte « Le Dernier Rempart » | détruit à la bataille d'Ostrag |
| VIV Triomphe | Cuirassé | 1ère Flotte « L'Invincible » | opérationnel (en réparation avec la flotte) |
| VIV Vorrhen | Cuirassé | 1ère Flotte « L'Invincible » — navire-amiral | en réparation (Vorrh) |
| VIV Aiglon | Porte-vaisseaux | 1ère Flotte « L'Invincible » | neuf — doit encore faire ses preuves |
| VIV Arche | Porte-vaisseaux | 3ème Flotte « Les Semeurs de Mondes » | vieux mais en bon état |
| VIV Fauconnier | Porte-vaisseaux | 1ère Flotte « L'Invincible » | déployable mais en mauvaise condition (réparations) |
| VIV Gerfaut | Porte-vaisseaux | 2ème Flotte « Le Dernier Rempart » | très endommagé (réparations à Ferrok) |
| VIV Grand-Duc | Porte-vaisseaux | 2ème Flotte « Le Dernier Rempart » | condamné — démantèlement pour pièces |
| VIV Braquemart | Croiseur | 3ème Flotte « Les Semeurs de Mondes » | relativement bon état malgré l'âge |
| VIV Coutelas | Croiseur | 3ème Flotte « Les Semeurs de Mondes » | endommagé, en réparation |
| VIV Doyen | Croiseur (croiseur-transport) | 3ème Flotte « Les Semeurs de Mondes » | opérationnel, tenu en retrait |
| VIV Espadon | Croiseur | 1ère Flotte « L'Invincible » | opérationnel |
| VIV Estoc | Croiseur | 1ère Flotte « L'Invincible » | opérationnel |
| VIV Fauchard | Croiseur | 3ème Flotte « Les Semeurs de Mondes » | relativement bon état malgré l'âge |
| VIV Fléau | Croiseur | 2ème Flotte « Le Dernier Rempart » (prêté par la 1ère) | opérationnel (dégâts légers) |
| VIV Framée | Croiseur | 1ère Flotte « L'Invincible » | opérationnel |
| VIV Glaive | Croiseur | 1ère Flotte « L'Invincible » | opérationnel |
| VIV Hallebarde | Croiseur | 2ème Flotte « Le Dernier Rempart » (prêté par la 1ère) | opérationnel (dégâts légers) |
| VIV Lance | Croiseur | 1ère Flotte « L'Invincible » | opérationnel |
| VIV Masse | Croiseur | 1ère Flotte « L'Invincible » | opérationnel |
| VIV Merlin | Croiseur | 3ème Flotte « Les Semeurs de Mondes » | endommagé, en réparation |
| VIV Pique | Croiseur | 1ère Flotte « L'Invincible » | opérationnel |
| VIV Sabre | Croiseur | 2ème Flotte « Le Dernier Rempart » (confié par la 1ère) | très endommagé, en réparation |
| VIV Ténace | Croiseur | 2ème Flotte « Le Dernier Rempart » | grièvement endommagé, en réparation |
| VIV Corollaire 🔒 | Navire scientifique (ancienne frégate) | EESS 🔒 | opérationnel, nord de la Zone Inexplorée |
| VIV Théorème 🔒 | Navire scientifique (ancien croiseur) | EESS 🔒 | opérationnel, nord de la Zone Inexplorée |

#### Union Galactique Libre (NSU) — 56 navires

| Nom | Classe | Flotte / rattachement | État (425at) |
|---|---|---|---|
| NSU Abondance | Cuirassé | Flotte de défense de Cygnara — navire-amiral | opérationnel |
| NSU Albâtre ✝ | Cuirassé | Flotte des Fantômes Blancs — navire-amiral | détruit à la bataille de Ferrogane (409at) |
| NSU Carmin ✝ | Cuirassé | Flotte Écarlate — navire-amiral | détruit à la bataille de Ferrogane (409at) |
| NSU Concorde | Cuirassé | Flotte du Concorde — navire-amiral | très endommagé, en réparation (Calderis) |
| NSU Insurgé | Cuirassé | Flotte de la Libération — ancien navire-amiral | en réparation (Calderis), non opérationnel |
| NSU Levant | Cuirassé | Flotte de l'Aube — navire-amiral | opérationnel |
| NSU Liberté | Cuirassé | Flotte de la Libération — navire-amiral | opérationnel |
| NSU Zénith | Cuirassé | Flotte de l'Aube — ancien navire-amiral | en réparation |
| NSU Arsenal | Porte-vaisseaux | Flotte de la Libération | opérationnel (neuf) |
| NSU Bâtisseur ✝ | Porte-vaisseaux | Flotte du Concorde | détruit à la bataille d'Ostrag (409at) |
| NSU Colosse ✝ | Porte-vaisseaux | Flotte de la Libération | détruit à la bataille d'Ostrag (409at) |
| NSU Concordia | Porte-vaisseaux | Flotte du Concorde | très endommagé, en réparation (Port-Vaillant) |
| NSU Convoyeur ✝ | Porte-vaisseaux | Flotte de la Libération | détruit à la bataille d'Ostrag (409at) |
| NSU Égide ✝ | Porte-vaisseaux | Flotte de l'Aube | détruit à la bataille d'Ostrag (409at) |
| NSU Grenat ✝ | Porte-vaisseaux | Flotte Écarlate | détruit à la bataille de Ferrogane (409at) |
| NSU Havre | Porte-vaisseaux | Flotte de la Libération | en réparation |
| NSU Horizon ✝ | Porte-vaisseaux (colonisation) | Flotte du Concorde | détruit à la bataille d'Ostrag (409at) |
| NSU Nourrice | Porte-vaisseaux (ravitaillement) | Flotte de l'Aube | opérationnel (réparé) |
| NSU Orient | Porte-vaisseaux | Flotte de l'Aube | opérationnel |
| NSU Pionnier ✝ | Porte-vaisseaux | Flotte du Concorde | détruit à la bataille d'Ostrag (409at) |
| NSU Rubis ✝ | Porte-vaisseaux | Flotte Écarlate | détruit à la bataille de Ferrogane (409at) |
| NSU Tête-de-Pont | Porte-vaisseaux (transport de troupes) | Flotte de la Libération | réparé — en attente de ses nouveaux transports |
| NSU Affranchi ✝ | Croiseur | Flotte du Concorde | détruit à la bataille d'Ostrag (409at) |
| NSU Audace | Croiseur | Flotte de la Libération | opérationnel (neuf) |
| NSU Charpentier | Croiseur | Flotte de Port-Vaillant — vaisseau amiral | opérationnel |
| NSU Cinabre ✝ | Croiseur | Flotte Écarlate | détruit à la bataille de Ferrogane (409at) |
| NSU Constance ✝ | Croiseur | Flotte du Concorde | détruit à la bataille d'Ostrag (409at) |
| NSU Dévouement ✝ | Croiseur | Flotte d'Alcyone | détruit à la bataille de la Trouée (409at) |
| NSU Éclat | Croiseur | Flotte de l'Aube | opérationnel (réparé) |
| NSU Éclipse ✝ | Croiseur | Flotte de l'Aube | détruit à la bataille d'Ostrag (409at) |
| NSU Étincelle | Croiseur | Flotte de l'Aube | en réparation |
| NSU Farouche ✝ | Croiseur | Flotte de la Libération | détruit à la bataille d'Ostrag (409at) |
| NSU Ferveur | Croiseur | Flotte de la Libération | opérationnel (remis en service en 424at) |
| NSU Fidèle | Croiseur | Flotte du Concorde — vaisseau amiral par intérim | opérationnel |
| NSU Fulgurance | Croiseur | Flotte de la Libération | opérationnel (neuf) |
| NSU Généreux | Croiseur | Flotte du Concorde | condamné — désossé pour pièces (à quai de Concordia) |
| NSU Geôlier | Croiseur | Flotte de défense du Verrou — vaisseau amiral | opérationnel |
| NSU Givre ✝ | Croiseur | Flotte des Fantômes Blancs | détruit à la bataille de Ferrogane (409at) |
| NSU Glaneur | Croiseur | Flotte de défense de Cygnara | endommagé (raid pirate), en réparation |
| NSU Incarnat ✝ | Croiseur | Flotte Écarlate | détruit à la bataille de Ferrogane (409at) |
| NSU Indomptable ✝ | Croiseur | Flotte de la Libération | détruit à la bataille d'Ostrag (409at) |
| NSU Insolent ✝ | Croiseur | Flotte de la Libération | détruit à la bataille d'Ostrag (409at) |
| NSU Ivoire ✝ | Croiseur | Flotte des Fantômes Blancs | détruit à la bataille de Ferrogane (409at) |
| NSU Linceul ✝ | Croiseur | Flotte des Fantômes Blancs | détruit à la bataille de Ferrogane (409at) |
| NSU Loyal ✝ | Croiseur | Flotte du Concorde | détruit à la bataille d'Ostrag (409at) |
| NSU Lueur | Croiseur | Flotte de l'Aube | opérationnel (réparé) |
| NSU Opiniâtre | Croiseur | Flotte du Concorde | gravement endommagé, en réparation (Port-Vaillant) |
| NSU Pourpre ✝ | Croiseur | Flotte Écarlate | détruit à la bataille de Ferrogane (409at) |
| NSU Prompt-Secours | Croiseur | Flotte d'Alcyone — vaisseau amiral | opérationnel |
| NSU Résolution | Croiseur | Flotte de la Libération | opérationnel (neuf) |
| NSU Sentinelle | Croiseur | Flotte défensive de Concordia — vaisseau amiral | opérationnel |
| NSU Serment ✝ | Croiseur | Flotte de la Libération | détruit à la bataille d'Ostrag (409at) |
| NSU Union ✝ | Croiseur | Flotte du Concorde | détruit à la bataille d'Ostrag (409at) |
| NSU Azimut 🔒 | Navire scientifique (croiseur transformé) | FSS | opérationnel — en garde de Vesper |
| NSU Lumières 🔒 | Navire scientifique (cuirassé transformé) | FSS — navire amiral | opérationnel, basé à Vesper (officiellement : détruit) |
| NSU Méridien 🔒 | Navire scientifique (croiseur transformé) | FSS | opérationnel — en exploration |

#### La Fédération (FED) — 25 navires

| Nom | Classe | Flotte / rattachement | État (425at) |
|---|---|---|---|
| FED Exode | Cuirassé | Flotte de l'Exil | opérationnel — vaisseau amiral |
| FED Hégémonie | Cuirassé | Flotte d'Attaque de la Fédération | en construction — sortie prévue en 430at |
| FED Inébranlable | Cuirassé | Flotte de Défense de la Fédération | opérationnel — vaisseau amiral |
| FED Suprématie | Cuirassé | Flotte d'Attaque de la Fédération | opérationnel (achevé en 420at) — vaisseau amiral |
| FED Vétéran | Cuirassé | Flotte de Défense de la Fédération | opérationnel (plus de quatre siècles de service) |
| FED Ancêtre | Porte-vaisseaux | Flotte de Défense de la Fédération | opérationnel (très ancien) |
| FED Aquilon | Porte-vaisseaux | Flotte d'Attaque de la Fédération | opérationnel — le dernier sorti des cales |
| FED Ouragan | Porte-vaisseaux | Flotte d'Attaque de la Fédération | en construction — sortie prévue d'ici 430at |
| FED Premier-Né | Porte-vaisseaux | Flotte de l'Exil | opérationnel (vétéran) |
| FED Semeur | Porte-vaisseaux | Flotte de Défense de la Fédération | opérationnel |
| FED Tourmente | Porte-vaisseaux | Flotte d'Attaque de la Fédération | en construction — sortie prévue d'ici 430at |
| FED Aïeul | Croiseur | Flotte de Défense de la Fédération | opérationnel (très ancien) |
| FED Anathème | Croiseur | Flotte d'Attaque de la Fédération | en construction — sortie prévue d'ici 430at |
| FED Barbacane | Croiseur | Flotte de Défense de la Fédération | opérationnel |
| FED Dernier-Mot | Croiseur | Flotte d'Attaque de la Fédération | commandé — sortie prévue d'ici 440at |
| FED Harpon | Croiseur | Flotte de l'Exil | opérationnel |
| FED Inflexible | Croiseur | Flotte de l'Exil | opérationnel |
| FED Interdit | Croiseur | Flotte d'Attaque de la Fédération | en construction — sortie prévue d'ici 430at |
| FED Meurtrière | Croiseur | Flotte de Défense de la Fédération | opérationnel |
| FED Prise | Croiseur | Flotte de l'Exil | opérationnel — capturé |
| FED Ressuscité | Croiseur | Flotte de Défense de la Fédération | opérationnel — reconstruit à partir d'une épave |
| FED Soc | Croiseur | Flotte de Défense de la Fédération | opérationnel |
| FED Talion | Croiseur | Flotte d'Attaque de la Fédération | opérationnel — tout juste sorti des cales |
| FED Verdict | Croiseur | Flotte d'Attaque de la Fédération | en construction — sortie prévue d'ici 430at |
| FED Vindicte | Croiseur | Flotte d'Attaque de la Fédération | opérationnel — tout juste sorti des cales |

#### Hexa-Corp — Flottes de la Corporation (HC) — 6 navires

| Nom | Classe | Flotte / rattachement | État (425at) |
|---|---|---|---|
| HC Invendu | Porte-vaisseaux | Flotte Alpha | opérationnel |
| HC Fleuron | Croiseur | Flotte Alpha — navire amiral | opérationnel |
| HC Garde-Fou | Croiseur | Flotte Alpha | opérationnel |
| HC Huissier | Croiseur | Flotte Beta — navire amiral | opérationnel |
| HC Recouvrement | Croiseur | Flotte Alpha | opérationnel |
| HC Repenti | Croiseur | Flotte Beta — ancien navire amiral | opérationnel |

#### Hexa-Corp — vaisseaux loués sous contrat (sans préfixe) — 7 navires

| Nom | Classe | Flotte / rattachement | État (425at) |
|---|---|---|---|
| La Herse | Porte-vaisseaux | Flotte de Défense de Skarn | opérationnel |
| Le Donjon | Porte-vaisseaux | Flotte de Défense de Skarn | opérationnel |
| La Muraille | Croiseur | Flotte de Défense de Skarn — navire amiral | opérationnel |
| Le Contrefort | Croiseur | Flotte de Défense de Skarn | opérationnel |
| Le Garant | Croiseur | Flotte défensive de Concordia | opérationnel |
| Le Grognard | Croiseur | Flotte Démocratique de Franchise — navire amiral | opérationnel |
| Le Mandataire | Croiseur | Escadre de Défense de Primaterre — navire amiral | en réparation (Solferra) |

#### Syndicat Tosear (TOS) — 4 navires

| Nom | Classe | Flotte / rattachement | État (425at) |
|---|---|---|---|
| TOS Butin | Porte-vaisseaux | Flotte de Terminus | opérationnel |
| TOS Revenant | Croiseur | Flotte de Terminus | opérationnel |
| TOS Tribut | Croiseur | Flotte de Terminus | opérationnel |
| TOS Trophée | Croiseur | Flotte de Terminus — navire amiral | opérationnel |

#### Syndicat Tosear 🔒 sous faux pavillon pirate (OXL) — 3 navires

| Nom | Classe | Flotte / rattachement | État (425at) |
|---|---|---|---|
| OXL Spectre 🔒 | Porte-vaisseaux | Flotte Nordique | opérationnel |
| OXL Faucheuse 🔒 | Croiseur | Flotte Nordique — chef de flotte | opérationnel |
| OXL Moissonneuse ✝ | Croiseur | Flotte Nordique (🔒 Syndicat Tosear) | détruit par la Meute de Raeilla la Guerrière |

#### Royaume Grothul (GRT) — 2 navires

| Nom | Classe | Flotte / rattachement | État (425at) |
|---|---|---|---|
| GRT Bannière ✝ | Croiseur | Flotte des Grothuls (la Garde dorée) | détruit par la Meute de Raeilla la Guerrière |
| GRT Couronne | Croiseur | Flotte des Grothuls — navire de la couronne | opérationnel (cérémoniel) |

#### Ligue des Pirates (sans préfixe) — 13 navires

| Nom | Classe | Flotte / rattachement | État (425at) |
|---|---|---|---|
| Le Purgatoire | Porte-vaisseaux | Meute de Virgile Le Maléfique | opérationnel (très vieux) |
| L'Enfer | Croiseur | Meute de Virgile Le Maléfique — croiseur chef | opérationnel |
| L'Étendard | Croiseur | Meute de Rajik Le Conquérant — croiseur chef | opérationnel |
| L'Ossuaire | Croiseur | Meute de Varga — croiseur amiral | opérationnel |
| La Carcasse | Croiseur | Meute de Brakk le Ferrailleur — croiseur chef | opérationnel (contre toute attente) |
| La Griffe | Croiseur | Meute des jumelles Osk — croiseur de Nyla | opérationnel |
| La Morgue | Croiseur | Meute de Circa — croiseur chef | opérationnel |
| La Récompense | Croiseur | Meute de Circa | opérationnel |
| La Révérence | Croiseur | Meute d'Arven le Doux — croiseur chef | opérationnel (impeccable) |
| La Serre | Croiseur | Meute des jumelles Osk — croiseur d'Ivra | opérationnel |
| Le Châtiment | Croiseur | Meute de Varga | opérationnel |
| Le Régicide | Croiseur | Meute de Varga | opérationnel |
| Le Silence | Croiseur | Meute de Nerra la Silencieuse — croiseur chef | opérationnel |

#### Pirates indépendants — Meute de Raeilla (sans préfixe) — 4 navires

| Nom | Classe | Flotte / rattachement | État (425at) |
|---|---|---|---|
| L'Inachevé | Porte-vaisseaux | Meute de Raeilla la Guerrière | opérationnel |
| L'Amalgame | « Croiseur » (classification contestée) | Meute de Raeilla la Guerrière | opérationnel |
| Le Défi | Croiseur | Meute de Raeilla la Guerrière — vaisseau-amiral | opérationnel |
| Le Sceptre-Brisé (ex-GRT Sceptre) | Croiseur | Meute de Raeilla la Guerrière | opérationnel — capturé au Royaume Grothul |

#### Écorcheurs ✝ (ECR) — 4 navires

| Nom | Classe | Flotte / rattachement | État (425at) |
|---|---|---|---|
| ECR Charognard ✝ | Croiseur (frégate convertie) | Flotte des Écorcheurs | détruit au piège de 403at |
| ECR Dépeceur ✝ | Croiseur | Flotte des Écorcheurs — navire-chef | détruit au piège de 403at |
| ECR Rançon ✝ | Croiseur | Flotte des Écorcheurs | détruit au piège de 403at |
| ECR Filoute ✝ | Frégate (nommée — exception historique) | Flotte des Écorcheurs | détruite au piège de 403at |

#### Flottille du Triomphe (0at) — frégates nommées — 4 navires

| Nom | Classe | Flotte / rattachement | État (425at) |
|---|---|---|---|
| L'Aurore ✝ | Frégate d'antimatière (nommée — exception historique) | — | sacrifiée au Triomphe (0at) |
| La Délivrance ✝ | Frégate d'antimatière (nommée — exception historique) | — | sacrifiée au Triomphe (0at) |
| La Résolue | Frégate d'antimatière (nommée — exception historique) | — | préservée |
| La Sentence ✝ | Frégate d'antimatière (nommée — exception historique) | — | sacrifiée au Triomphe (0at) |

---

## Annexe B — Pistes narratives NON CANON (dossier `99-Idees/`)

> ⚠️ **Rien de ce qui suit n'est canon.** Ce sont des idées d'histoires notées par l'auteur, à n'utiliser que comme inspiration et **jamais comme des faits du monde**. Le wiki canon ne les cite jamais. Elles montrent seulement vers où l'univers pourrait évoluer.

- **Les grandes menaces** : 01 Le réveil de l'Essaim · 14 La Grande Nuit approche (résurgence des Frères de l'Aube) · 15 Reliques de l'Essaim (le marché noir le plus dangereux) · 16 La première désassimilation.
- **Les secrets qui peuvent sauter** : 02 La découverte de Vesper · 03 Le mystère Corvan Sarkis · 05 Le général qui sert (l'offrande vivante sur Vashyra) · 06 Le masque des Hashashin · 12 Le prochain coup de Benedict · 25 Myrth dévoilée.
- **Guerre froide et crime** : 07 La guerre Tosear / Ligue · 17 La guerre des monnaies · 18 Affamer Cygnara · 21 L'assaut de Rade-Noire · 27 La conférence d'Hexis · 28 La guerre des Portails · 36 Le pari du siècle (le Casino parie sur la guerre) · 37 Les Brigands d'Hexis · 38 La Grande Faucheuse · 39 La cour de Rade-Noire · 44 La guerrière du Nord · 45 Le double jeu de la Guilde.
- **Industrie et escalade** : 08 Le chantier qui change tout (les cuirassés d'Hexa-Corp) · 09 Vorrh-V, le géant endormi · 26 Le procès de la neutralité · 35 Le duel des géants (NSU Liberté contre VIV Vorrhen).
- **Royaumes et marches** : 04 La vengeance des Golden Sword · 13 La restauration de Primaterre.
- **Trames personnages et décors** : 10 Pèlerinage solaire · 11 Les enfants volées de Myrth · 19 Expédition Aldène · 20 Les systèmes abandonnés (générateur d'épisodes « western spatial ») · 22 Le prodige Ethari · 23 Un héros de la Rébellion · 24 La succession de Sarkis · 29 Les planètes errantes · 30 Le secret du noyau (le trou noir et Hélion) · 31 La grande évasion d'Ostrag · 32 Le secret de Franchise · 33 L'escadrille fantôme (l'EESS vers l'Essaim) · 34 La sauveuse qui n'arriva jamais · 40 Les héritiers des Écorcheurs · 41 Les voies détournées · 42 Les voix ignorées · 43 Avant les étoiles · 46 Arven, le Frère parmi les loups.
- **Croisements notés par l'auteur** : tout ce qui touche l'Essaim converge (01, 02, 15, 16) ; le fil Hashashin relie Sarkis, Vashyra et Primaterre (05, 06, 13) ; la Ligue est au centre du jeu impérial (07, 18, 21) ; l'escalade industrielle mène au vaisseau capitale (08, 09, 26, 35) ; les deux programmes secrets explorent la même Zone (33, 02).
