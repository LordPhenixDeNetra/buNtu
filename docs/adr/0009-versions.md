# ADR-0009 — Gestion des versions : script maison + git-cliff, cargo-dist inchangé

- **Statut :** accepté
- **Date :** 8 novembre 2026
- **Contexte :** cahier des charges §12.2 et §12.3, préparation de la v0.1.0.

## Contexte

Le cahier prévoit que « `cargo-release` monte la version et génère le changelog depuis les
commits conventionnels (`git-cliff`) », puis que cargo-dist publie sur un tag `vX.Y.Z`
(en place depuis T1.25).

Dans ce dépôt, la version est écrite à 11 endroits : `[workspace.package]`, les neuf
dépendances internes de `[workspace.dependencies]` (leur `version` est exigée par la règle
`bans.wildcards` de cargo-deny) et `npx/package.json`. La documentation de référence de
cargo-release (consultée le 2026-11-08) ne garantit la mise à jour que des « path dependency
versions within the workspace », sans rien dire de `[workspace.dependencies]`. Elle ne connaît
pas non plus `npx/package.json`.

## Décision

- **`scripts/release.mjs`** (Node, sans dépendance, testé par `scripts/release.test.mjs`) :
  - `bump X.Y.Z` réécrit les 11 emplacements, met à jour `Cargo.lock`
    (`cargo update --workspace`) et lance git-cliff ;
  - `check [vX.Y.Z]` vérifie que tout est cohérent (et égal au tag s'il est donné). Il est
    exécuté en CI à chaque PR.
  Le script ne commite, ne tague et ne pousse rien : la publication reste un acte humain.
- **git-cliff** génère `CHANGELOG.md` (`cliff.toml`, titres en français, fusions exclues).
- **cargo-dist inchangé** : il reprend les notes de la version dans `CHANGELOG.md` et classe
  les pré-releases d'après le numéro.
- crates.io n'est pas publié tant que le nom n'est pas validé (T0.1).

## Alternatives écartées

- **cargo-release** : le comportement sur `[workspace.dependencies]` n'est pas documenté, et
  `npx/package.json` demanderait des remplacements par motif dans chaque crate. On pourra y
  revenir pour publier sur crates.io, à condition de vérifier ce point.
- **Versions indépendantes par crate** : sans intérêt tant que les crates sont publiées
  ensemble ; source d'erreurs pour une personne seule.
- **CHANGELOG écrit à la main** : les commits sont déjà conventionnels (132/132 hors fusions),
  la génération est fiable et relue avant chaque release.

## Conséquences

- Une release se prépare en une commande, puis passe par une PR relue comme le reste.
- `git-cliff` est un outil de développement à installer (`cargo install git-cliff --locked`),
  pas une dépendance du projet.
- Ajouter un emplacement de version (nouvelle crate, nouveau paquet) impose de mettre à jour le
  script ; le contrôle de CI le signale sinon.
