# Questions pour l'auteur

> L'IA note ici ce qui revient à l'auteur. En attendant la réponse, elle avance sur l'hypothèse indiquée (réversible).
> **Auteur** : réponds directement sous la question (ligne `→ Réponse :`), l'IA intégrera ta réponse et déplacera la question dans « Tranchées ».

## Ouvertes

## Tranchées

1. **Départ de la v1.** Capitaine indépendant à Hexis, endetté auprès de la Banque (P1) : ça te va ?
   → **Oui.** (03/11/2026)

2. **Rythme du temps.** En jeu à pied ou en vol, à quelle vitesse passe le temps de la galaxie ?
   → **Réglable dans les données, valeur de départ ×30.** (03/11/2026)

3. **Calendrier.** L'année stellaire est-elle découpée (jours, mois, semaines) ? Combien de jours ?
   → **Affichage « jour N, 425at », 365 jours.** (03/11/2026) — l'interface étant en français (Q11), l'affichage est « Jour N, 425at ».

4. **Grades inférieurs.** Quels grades sous Amiral, pour l'Empire, l'UGL, Hexa-Corp, la Ligue, Tosear ?
   → **Échelles provisoires marquées Proposition.** (03/11/2026)

5. **Direction artistique.** Réaliste « usé », propre et lumineux, ou stylisé ?
   → **Réaliste stylisé, géométrie simple et lisible.** (03/11/2026)

6. **Secrets.** Le joueur pourra-t-il un jour découvrir certains secrets 🔒 par le jeu ?
   → **Pour la v1 : jamais.** (03/11/2026)

7. **Mort et sauvegarde.** Sauvegarde libre, ou mode « une seule sauvegarde » (Homme de fer) ?
   → **Sauvegarde libre + option Homme de fer.** (03/11/2026)

8. **Langues.** Français seulement, ou prévoir l'anglais ?
   → **Uniquement l'anglais pour le début, textes en données pour pouvoir traduire.** (03/11/2026) — **remplacée par Q11** : l'interface est en français.

9. **Étoiles sans nom.** Comment afficher les étoiles de Cygnara et d'Hexis ?
   → **« Étoile de Cygnara », « Étoile d'Hexis ».** (03/11/2026)

11. **Noms canon dans une interface en anglais.** Traduire les noms d'institutions et de groupes, ou les garder en français ?
   → **« Repasse l'interface en français, j'avais oublié que tous les noms propres étaient français. »** (03/11/2026) — intégré le 03/11/2026 : la langue du jeu est le français (`assets/data/lang/fr.ron`, nombres « 12 500,5 ») ; `en.ron` est gardé à jour avec les mêmes clés (test), pour une traduction future. La date s'affiche « Jour N, 425at » (Q3).

12. **Issues politiques dans la simulation.** Faut-il que la simulation tranche l'Arène et la succession de Nyra Maren ?
   → **« Pour l'instant la simulation annonce juste les échéances ; dans le futur elle pourra déterminer automatiquement les événements des personnages nommés. »** (03/11/2026) — c'est le comportement actuel (M4) ; l'architecture le permettra (événements de personnages nommés, plus tard).

13. **Disposition du clavier.** Joues-tu en AZERTY ? Faut-il afficher les touches selon la disposition réelle ?
   → **« Oui je joue en AZERTY, il faudra afficher la disposition réelle du clavier ; le remappage complet arrive avec les options (M7). »** (03/11/2026) — intégré : les aides citent des jetons de touche (`[KeyE]`) remplacés par l'étiquette de la disposition (`gameplay/keyboard.ron`, AZERTY supposée au départ), corrigée à chaque appui par le vrai caractère de la touche. Les liaisons restent physiques jusqu'au remappage de M7 (la carte est donc sur la touche « , » en AZERTY).

14. **Direction artistique après M7.** La tranche est jouable : la direction « réaliste stylisé, géométrie simple » suffit-elle ?
   → **« Il est moche. Pas vraiment moche, mais les murs sont plats, il n'y a aucun détail ; il faudrait un vrai tuto qui explique le jeu et guide les premières minutes (désactivable dans les options). La prochaine partie : la refonte totale du visuel pour un style futuriste (inspiré de Star Wars). »** (04/11/2026) — remplace la direction validée en Q5 ; intégré comme jalon M8 (`docs/refonte-visuelle.md`). La direction sonore (P27) est validée « pour un début ».
