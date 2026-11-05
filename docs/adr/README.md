# Décisions d'architecture (ADR)

Un fichier par décision structurante, au format de l'annexe B du cahier des charges :
contexte, décision, alternatives écartées, conséquences, date, statut.

## Index

| Numéro | Titre | Tâche | Statut |
| --- | --- | --- | --- |
| [0001](0001-nom.md) | Nom du projet | T0.1 | proposé (marques et réservations à faire, côté humain) |
| [0002](0002-schema.md) | Schéma canonique v0 | T0.5 | accepté |
| [0003](0003-runtime.md) | Runtime asynchrone : tokio | T0.6 | accepté |
| [0004](0004-http.md) | HTTP : hyper/axum et client amont | T0.6 | accepté |
| [0005](0005-storage.md) | Stockage : SQLite puis PostgreSQL | T0.6 | accepté |
| [0006](0006-ui.md) | Interface : SvelteKit statique | T0.6 | accepté sous réserve de prototype |
| [0007](0007-license.md) | Licence Apache 2.0 et DCO | T0.6 | accepté |
| [0008](0008-config-source.md) | Configuration : trois sources, un seul modèle | revue 2026-11-05 | accepté |

## Conventions

- Numérotation à quatre chiffres, dans l'ordre de création ; `0001` est réservé à la décision
  sur le nom (T0.1) : recherche de marques EUIPO/USPTO et réservations (crates.io, GitHub, npm,
  Docker Hub, domaine), avec captures.
- Un ADR ne se réécrit pas : une décision qui change donne un nouvel ADR qui remplace l'ancien
  (le statut de l'ancien devient « remplacé par 000N »).
- Les ADR sont rédigés en français ; le code et les commits en anglais.
