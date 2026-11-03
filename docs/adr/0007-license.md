# ADR-0007 — Licence Apache 2.0 et DCO

- **Statut :** accepté
- **Date :** 3 novembre 2026
- **Contexte :** cahier des charges §2 (contraintes) et §16 (gouvernance), T0.6.

## Contexte

Le projet promet que **toutes** les fonctionnalités resteront gratuites, y compris celles que
les concurrents réservent à une offre entreprise. Il doit attirer des contributeurs externes et
rester utilisable en entreprise sans friction juridique.

## Décision

- **Code sous Apache License 2.0** (`LICENSE`), avec un fichier `NOTICE`.
- **DCO** (`Signed-off-by`, `git commit -s`) plutôt qu'un CLA : plus simple, et il empêche un
  relicenciement propriétaire sans l'accord de tous les contributeurs.
- **Dépendances limitées aux licences compatibles** : MIT, Apache-2.0, BSD, ISC, Zlib — plus
  `Unicode-3.0` pour les données Unicode exigées par `serde_derive`. Vérification automatique
  par `cargo deny check` en CI (`deny.toml`), et `cargo audit` pour les vulnérabilités.
- `GOVERNANCE.md` (T0.7) inscrit l'engagement « tout gratuit » : aucune fonctionnalité derrière
  une clé de licence, jamais.

## Alternatives écartées

- **MIT seule** : pas de clause explicite de brevets, moins protectrice pour les contributeurs.
- **MIT/Apache-2.0 dual** : complexité de conformité sans bénéfice ici.
- **AGPL** : incompatible avec l'adoption en entreprise visée par le cahier (O5, O6).
- **MPL-2.0** : copyleft par fichier, contrainte supplémentaire pour les fournisseurs.

## Conséquences

- Toute contribution doit porter `Signed-off-by` ; les commits de l'agent et de l'humain le font.
- Le fichier `NOTICE` est conservé par toute redistribution.
- Un ajout de dépendance non listée dans `deny.toml` échoue en CI : c'est une décision
  explicite, pas un accident.
