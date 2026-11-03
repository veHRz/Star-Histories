# Star Histories — Document de conception (GDD)

> **Statut** : conception proposée par l'IA à partir de `_context/description.md` et du canon. Tout ce qui n'est pas canon est une **proposition** que l'auteur peut changer. Les éléments d'univers sont tirés de `_context/STAR-HISTORIES-export-IA.md` (sections citées entre parenthèses).
> Ce document décrit **où va le jeu**. Ce qui est construit, et dans quel ordre, est dans `docs/ROADMAP.md`.

---

## 1. Pitch

**Astréa, 425at.** Deux blocs de taille égale se regardent par-dessus un No Man's Land aux Portails détruits. Tout le monde sait que la paix n'est qu'un entracte. Toi, tu n'es personne : un capitaine, un vaisseau, des dettes. Mais la galaxie vit sans toi, et elle peut basculer à cause de toi.

**Genre** : RPG solo à la première personne, monde ouvert systémique, space opera.
**Références de ressenti** (pas d'emprunts) : la liberté et la carrière d'un *Mount & Blade* ou d'un *X4*, l'immersion à la première personne d'un *Starfield* ou d'un *Prey*, la densité politique de *Star Wars* et *Star Trek*.

---

## 2. Piliers

| # | Pilier | Ce que ça veut dire concrètement |
|---|---|---|
| 1 | **Un monde qui vit sans toi** | Les flottes se déplacent, se réparent, se battent ; les prix montent et descendent ; Primaterre se révolte ; Le Berceau avance ; l'Arène de Tharkast se tient. Tu l'apprends par les nouvelles, les rumeurs et ce que tu vois. |
| 2 | **Tu pèses** | Chaque action est injectée dans la simulation : un convoi détruit affame Cygnara, une information vendue à Hexis déplace une flotte, un amiral tué change la doctrine d'une flotte. |
| 3 | **Plusieurs vies dans une vie** | À pied, pilote, capitaine, marchand, contrebandier, mercenaire, espion, et plus tard officier de flotte. On change de métier sans changer de partie. |
| 4 | **Loyauté et trahison** | Rejoindre une organisation, y monter en grade, s'allier à une autre, jouer double jeu, trahir. Les organisations se souviennent, et chacune à sa manière. |
| 5 | **Brutal et court** | Pas de boucliers : seul le blindage protège (§2.4). Les combats, à pied comme dans l'espace, sont rapides, létaux, décidés par la préparation et la position. |
| 6 | **Le mystère respecté** | Les secrets de l'univers existent et agissent. Le joueur peut en voir les effets (la Ligue épargne curieusement les convois impériaux) sans jamais en lire l'explication. |

---

## 3. Le joueur

### 3.1 Départ de la v1 (proposition)

**Capitaine indépendant à Hexis.** Tu possèdes un petit vaisseau marchand (taille corvette, ≈ 60–80 m, équipage minimal), endetté auprès de la Banque d'Hexis (division Finance d'Hexa-Corp, §4.4). Hexis est le meilleur départ « bac à sable » : terrain neutre, seul hub de Portails reliant l'UGL et l'Empire (§3.7), contrats Hexa-Corp ouverts à tous, espions de tous les camps.

Les **départs préconfigurés** par faction et par rôle (`_context/futur.md`) viendront après la v1. Techniquement, un départ est un fichier de données : lieu, vaisseau, argent, dettes, réputations, appartenance, grade, connaissances initiales, mission d'ouverture.

### 3.2 Personnage

- **Espèce** : humain en v1 (l'Empire réserve de fait les hautes responsabilités aux humains, §4.1 : jouer un non-humain changera beaucoup de choses, c'est pour plus tard).
- **Compétences** (progression par l'usage, sans classes) : Tir, Combat rapproché, Pilotage, Navigation (calcul des sauts), Ingénierie (réparations, modules), Médecine, Commerce, Négociation, Intimidation, Discrétion, Piratage, Commandement.
- **Traits** gagnés par les actes (proposition) : « Parole tenue », « Traître notoire », « Ami des Jaskari », « Bourreau »… Ils modifient les dialogues et les réactions.
- **Mort** : à pied ou dans l'espace, la mort est réelle (chargement de sauvegarde). Mode « Homme de fer » (sauvegarde unique) en option plus tard.

### 3.3 Argent

- **Crédit** : monnaie universelle de fait. **Wuro** : monnaie d'État impériale, imposée dans l'Empire, **interdite dans l'UGL** (§2.8). Le **taux d'Hexis** fixe le change officieux.
- Payer en Wuro te marque comme impérial ; en refuser dans l'Empire est un délit (sévère pour les gradés). Changer au marché noir (Tosear) est rentable et risqué.
- **Dettes** : la Banque d'Hexis prête (CC), rachète des dettes (CRD), hypothèque (CH) — et saisit automatiquement : ne pas payer, c'est voir arriver la **Flotte Beta**, « l'ennemie des criminels » (§6.5).

---

## 4. Boucles de jeu

| Échelle | Durée | Boucle |
|---|---|---|
| **Instant** | secondes | Explorer, parler, fouiller, combattre à pied, piloter, viser une section de coque |
| **Session** | 10–60 min | Accepter un contrat → préparer le vaisseau → voyager → affronter l'imprévu → livrer / échouer → être payé ou poursuivi |
| **Carrière** | heures | Monter en grade, choisir ses allégeances, s'enrichir, améliorer son vaisseau, peser sur la guerre, trahir au bon moment |

---

## 5. Systèmes

### 5.1 Réputation, appartenance, grades

- **Réputation** par organisation, de −100 à +100, en paliers : *Ennemi juré*, *Hostile*, *Méfiant*, *Neutre*, *Apprécié*, *Allié*, *Héros*. Organisations suivies à plusieurs niveaux : Empire (et ses flottes), UGL (FOU, FDU, gouverneurs), Hexa-Corp (**ses six divisions sont en concurrence**, §4.4), Syndicat Tosear, Ligue des Pirates (et ses huit meutes, §6.6), Royaume Grothul, Fédération, Franchise, Frères de l'Aube, Guilde des Minerais, Jaskari, Mercenaires de Cendres…
- **Rejoindre** une organisation ouvre une **échelle de grades** (données par organisation). Les grades élevés canon existent (Amiral, sous-amiral, contre-amiral, Grand Amiral, §2.5) ; les grades inférieurs ne sont pas définis : **à proposer à l'auteur**.
- **Chaque organisation promeut à sa façon** : l'Empire par le mérite prouvé (méritocratie) ; l'UGL par le service et la confiance politique ; Hexa-Corp par les contrats rentables ; la Ligue par le butin et la peur ; Tosear par les dettes honorées.
- **Double jeu** : on peut appartenir à une organisation tout en servant une autre (identité de couverture, risque d'exposition qui monte à chaque action).
- **Trahison** : agir contre son organisation de l'intérieur (fuite d'informations, sabotage, désertion avec le vaisseau, assassinat, livraison d'un supérieur). Conséquences selon l'organisation : dégradation, prime, traque, et des mémoires particulières (proposition fondée sur le canon) :
  - **Tosear** : « la dette est sacrée » (§5) — une dette trahie ne s'efface jamais.
  - **Jaskari** : escroquer un Jaskari ferme la guilde entière, partout, pour toujours (§5).
  - **Hexa-Corp** : deux contrats Hexa-Corp ne s'affrontent jamais (§4.4) ; le mauvais payeur est traqué par la Flotte Beta.
  - **Arven le Doux** : « qui se rend vit » — se rendre à sa meute est une vraie option.

### 5.2 Économie et commerce

- Marchandises par catégorie : nourriture, **carburant lourd**, minerais, terres rares, pièces détachées, armes, équipements, médicaments, produits de luxe, et contrebande (Wuro au noir, armes en zone interdite, épaves de l'Essaim, et le rarissime **combustible nucléaire**, que seuls des vaisseaux militaires transportent légalement, §2.3).
- Prix par lieu, issus de la simulation (production, consommation, blocus, raids). Les talons d'Achille canon sont des opportunités : Cygnara ne produit rien à manger (§3.3), Bastia vit des convois, Concordia dépend des collecteurs d'Hélion.
- **Légalité par juridiction** : jeux d'argent interdits dans l'Empire, magie interdite dans l'Empire et tolérée dans l'UGL, Wuro interdit dans l'UGL, douane de façade à Grotha (on inspecte ce qui entre, jamais ce qui sort).

### 5.3 Missions

1. **Contrats Hexa-Corp** : les types canon (§13 de l'export) deviennent des missions jouables : transport (CT), ravitaillement (CR), remorquage (CRS), minage (CMS), équipage (CE : servir à bord d'un vaisseau loué), mercenariat au sol (CM).
2. **Missions systémiques** générées par la simulation : escorter un convoi d'Ambroise vers Cygnara, chasser un pirate qui pille la route, livrer des pièces à un chantier en pénurie, passer le No Man's Land, récupérer une cargaison dans une épave.
3. **Missions écrites** (plus tard, avec l'auteur) : la « Trame Actuelle » de l'auteur est l'endroit où naissent les histoires de 425at.

### 5.4 Vaisseaux

- **Ton vaisseau** est un lieu (tu y vis, tu y marches) et un outil. Sections de coque avec **blindage** localisé (pas de bouclier), modules (soute, réacteur, moteurs, armes, capteurs, quartiers), **deux énergies** : carburant lourd (déplacements et bord) et réacteur nucléaire (hyperpropulsion, §2.3). L'autonomie se mesure en carburant lourd.
- **Équipage** : PNJ recrutés (Cendres recrute pour tout le monde), avec rôles, salaire, moral et loyauté.
- **Entretien** : réparations (CRV à Hexis ou chantiers locaux), assurance de flotte (CAV), achat d'occasion (CVV), location (CVL / CVLM).
- **Armement canon** : canons à projectiles magnétiques, missiles, torpilles (§2.4). Classes, tailles et équipages canon (tableau §2.4) servent de référence à toutes les coques.

### 5.5 Combat

- **À pied** : armes à projectiles, armures, couverture, alertes, peu de points de vie. Les Saluviens soignent (plus tard, avec leur coût : trop soigner tue le soigneur).
- **Spatial** : pilotage depuis le cockpit (mode assisté et mode découplé), visée de **sections**, missiles et contre-mesures, abordage. Un impact se ressent à l'intérieur (alarmes, incendies, dépressurisation). Combats courts et brutaux.
- **Commandement** (plus tard, aux grades élevés) : ordres à une escadrille, puis à une flottille. Le gameplay change d'échelle avec la carrière.

### 5.6 Voyage

- **Portails** : quasi instantanés ; péages, contrôles et autorisations de la faction qui tient le système ; blocus possibles. **À Kholvar, quiconque emprunte un Portail est abattu sans sommation** (§2.2). Hubs et paires fixes du canon.
- **Hyperespace** : lent (≈ 3 jours entre voisins), calculs de navigation (routes cartographiées plus rapides et plus sûres que l'improvisation), **nébuleuses infranchissables** (le Croissant) à contourner ou traverser en vitesse normale. Chasseurs et bombardiers sans hyperpropulsion.
- **Systèmes abandonnés** (jamais rebranchés aux Portails) : refuges, cachettes, épaves, secrets.

### 5.7 Information

- **L'information est une marchandise** : la division Renseignement d'Hexa-Corp (« Le Facteur ») achète et vend au plus offrant (§4.4).
- **Brouillard d'information** sur la carte galactique : positions de flottes connues par les nouvelles, les rumeurs, le renseignement acheté, l'observation directe. Une information vieillit.
- **Nouvelles** : fil d'actualité généré à partir des événements publics de la simulation, avec la couleur de l'émetteur (presse unioniste, propagande impériale, rumeurs de taverne).
- **Codex** : encyclopédie en jeu des informations **publiques** du canon, débloquées par la découverte. Aucun secret 🔒.

### 5.8 Dialogues

Arbres en données (RON) : conditions (réputation, grade, compétences, connaissances, traits, objets) et effets (commandes de simulation, réputation, missions, connaissances). Répliques ambiantes des PNJ selon le lieu et l'actualité.

---

## 6. Lieux de la tranche verticale (proposition)

| Lieu | Vue | Rôle dans la tranche | Statut |
|---|---|---|---|
| Le vaisseau du joueur | Sectorielle + Locale | Foyer, tutoriel de déplacement, cockpit | Proposition (vaisseau générique) |
| Spatioport d'Hexis (un quartier) | Sectorielle + Locale | Bureau de contrats Hexa-Corp, marché, taverne, chantier de réparation | Lieu canon, détails à proposer (les quartiers d'Hexis sont un vide connu, §11.1) |
| Système d'Hexis | Globale | Décollage, Portail, L'Enclume en construction, Flotte Alpha en patrouille | Canon (étoile non nommée ❓) |
| Carte d'Astréa | Galaxie | Trajet par Portail ou hyperespace | Canon |
| Système Cygnara | Globale | Livraison, raid de la Ligue (les pirates remontent jusqu'à Cygnara, §9.1), Flotte de la Libération en garde | Canon (étoile non nommée ❓) |

Fil conducteur proposé, « **Le contrat d'Hexis** » : rembourser sa première échéance à la Banque en acceptant un contrat de transport (CT) d'Hexis vers Cygnara, à travers une galaxie qui bouge pendant le trajet.

---

## 7. Interface (intentions)

- HUD diégétique autant que possible (visière, consoles du vaisseau), minimal à pied.
- Terminaux en jeu pour le commerce, les contrats, les nouvelles.
- Carte galactique fidèle à la carte canon (grille A–T × 1–13, zones, Zone Inexplorée en silence).
- Langue : français. Textes en données pour permettre d'autres langues plus tard.

---

## 8. Hors périmètre de la v1

- Débuts multiples et factions jouables (`futur.md`).
- Personnage non humain, magie jouable, pouvoirs des espèces.
- Commandement de flottes, conquête de territoire.
- L'Essaim actif, révélations de secrets.
- Multijoueur (jamais : jeu solo).
