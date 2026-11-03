# sh_import — import de la carte canon

Convertit `_context/star-histories-carte.json` (exporté du wiki de l'auteur) en
`assets/data/canon/galaxy.ron`, lu par le jeu. Le fichier RON est **généré** : on ne l'édite
jamais à la main.

## Usage

Depuis la racine du dépôt :

```powershell
cargo run -p sh_import            # régénère assets/data/canon/galaxy.ron
cargo run -p sh_import -- --check # vérifie seulement qu'il est à jour (utilisé par tools/check.ps1)
```

Options : `--input <json>` et `--output <ron>` (chemins relatifs à la racine du dépôt).

## Quand l'auteur régénère l'export

1. `cargo run -p sh_import`
2. `cargo t` (validation des données : identifiants, références, cases, secrets)
3. Lire le diff de `galaxy.ron` et signaler les changements de canon qui touchent le jeu.

Si l'export gagne un champ inconnu, l'import **échoue** (`deny_unknown_fields`) : il faut décider
quoi en faire dans `src/lib.rs` plutôt que de l'ignorer en silence.

## Règles d'interprétation (toutes dans `src/lib.rs`)

- **Identifiants** : `sys-concordia` → `sys_concordia` (tirets → soulignés).
- **Rattachement aux factions** : le texte d'affiliation commence par le nom de l'organisation
  (`UGL — capitale` → `ugl`, `UGL (FOU)` → `fou`, `Empire (annexion)` → `empire`…) ; « aucune »,
  « personne », « indépendante » ne rattachent à rien. Les identifiants de faction doivent exister
  dans `assets/data/canon/factions.ron`.
- **Secrets 🔒** : un passage marqué 🔒 est retiré du texte public et rangé dans `secret_notes`.
  Entre parenthèses, toute la parenthèse est secrète ; sinon, tout ce qui suit le marqueur jusqu'à
  la fin du champ (lecture prudente). La validation vérifie qu'aucun autre champ ne contient 🔒.
- **Sections sources** : zones §3.2, systèmes et leurs astres §3.3, astres isolés §3.4, stations
  §3.5, phénomènes §3.6, hubs §3.7, flottes §6.1.
- **Divers** : `etat` commençant par « détruite » → `destroyed` ; « en construction » →
  `under_construction` ; un phénomène dont le `type` dit « bloque l'hyperespace » →
  `blocks_hyperspace` ; rang orbital « dernier » → `Outermost`.
- Les champs `*_note` gardent le texte canon d'origine, en français : internes, jamais affichés.
