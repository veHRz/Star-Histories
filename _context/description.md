# Star Histories (description initiale)

## Description rapide

Star histories est un jeux RPG solo en mode Space Opera.
Dans ce jeu on évolue dans la galaxie d'Astréa0 en pleine guerre.
Ce jeu nous permet de prendre en grade, d'évoluer, de faire plusieurs gameplay, de s'allié à des groupes, de rejoindre des groupes, de de trahir, etc ....

Dans ce jeux ou peut jouer un personnage qui à plusieurs début mais ils ont tous en commun le fait de pouvoir évoluer dans leurs mondes.

Le monde évolue tout seul sans l'intréraction du joueur mais celui-ci à un grand impact sur les évènement qui peuvent s'y passer.

Toutes les parties en (futur) ne sont pas forcement à intégrer dès la première version et sont des objectif pour le futur et qui sont souvent toujours pas fini d'être écrit.

## Fonctionnalitées

Je te laisse libre cours à ton imagination pour le

## Description Technique

Caméra FPS et monde en 3D.

### Moteur de jeux

Bevy avec sûrement quelques addons.

### Modèles graphique

Blender avec le MCP.

### Système de "vues" différentes

On a 4 différrntes vues qui permettent de répartir la charge et de charger que certaines quand il le faut. Un temps/écran de chargement entre la vue Galaxie et Globale est accepté.
Un temps de chargement entre la vue Globale et sectorielle est aussi accepté. Par contre pas de temps de chargement visible entre vue sectorielle et locale. J'ai imaginé ce système rapidemenet et je suppose qu'il a des façon de l'optimiser ou de carrement tout changerr et faire mieux, je te fait confiance.

#### Vue Galaxie

Lorsque le vaisseaux se déplace on est placé sur la carte de la Galaxie et alors les autres vue sont déchargé.

#### Vue Globale

Vue du système / planète au globale ou du gros vaisseaux dans son ensembl qui est charger sur l'écran lorsqu'on est dans le système ou sur une planètes ou dans un gros vaisseaux.

#### Vue Sectorielle

C'est la vue affichée lorsqu'on est dans un secteur d'une planète / gros vaisseaux ou carrement tout le vaisseau pour les petits vaisseau et qui est chargée en meilleur qualitée que la vue globale.

#### Vue Locale

C'est la vue d'une pièce/couloir dans lequel/laquelle on est et qui est afficher à la meilleure qualitée.




