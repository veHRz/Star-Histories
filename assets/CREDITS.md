# Crédits des assets externes

> Tout asset qui n'a pas été produit par un script de `tools/blender/` est consigné ici : fichier, source (URL), auteur, licence. Pas de licence claire = pas d'asset.

| Fichier | Source | Auteur | Licence |
|---|---|---|---|
| `assets/fonts/Inter-Regular.ttf`, `assets/fonts/Inter-SemiBold.ttf` | [Inter](https://rsms.me/inter/), fournie avec Blender 5.2 (`datafiles/fonts/Inter.woff2`), convertie en TTF statiques par `tools/fonts/convert_inter.py` | The Inter Project Authors (Rasmus Andersson) | SIL Open Font License 1.1 (`assets/fonts/OFL-Inter.txt`) |
| `assets/audio/**/*.wav` (musiques, ambiances, effets) | Générés par `tools/sh_audio` (synthèse déterministe, sans échantillon extérieur) | Projet Star Histories | Création du projet, aucune licence tierce |
| `assets/fonts/DejaVuSansMono.ttf` | [DejaVu Sans Mono](https://github.com/dejavu-fonts/dejavu-fonts) 2.37, fournie avec Blender 5.2 (`datafiles/fonts/DejaVuSansMono.woff2`), convertie en TTF par `tools/fonts/convert_dejavu_mono.py` (glyphes intacts, seul le format change) | Bitstream, Inc. (glyphes Bitstream Vera), Tavmjong Bah (glyphes Arev) ; modifications DejaVu dans le domaine public | Bitstream Vera Fonts License + Arev Fonts License (`assets/fonts/LICENSE-DejaVuSansMono.txt`) |
| `assets/ui/*.png` (cadres d'interface en 9 tranches) | Générés par `tools/ui/frames.py` | Projet Star Histories | Création du projet, aucune licence tierce |
