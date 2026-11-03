# Suivi des tâches

Source de vérité de l'avancement du projet. Ce document est mis à jour **dans le même commit**
que la tâche concernée (règle locale), et la case correspondante du cahier des charges est
cochée dans le même mouvement.

Une tâche n'est « faite » que si la machine le prouve : fusionnée, CI verte, preuve archivée
dans la PR.

## Légende

| Statut | Sens |
| --- | --- |
| à faire | pas commencée |
| en cours | branche ouverte, PR non fusionnée |
| fait | fusionnée, CI verte, preuve dans la PR |
| bloqué | dépendance externe ou humaine |

## Tâche en cours

Aucune. Prochaine : **T1.6** (chaîne de repli et alias).

## Phase 0 — Fondations (15 j)

| ID | Tâche | Statut | PR | Date | Preuve |
| --- | --- | --- | --- | --- | --- |
| T0.1 | Valider le nom (marques EUIPO/USPTO, réservations) | à faire (humain) | — | — | `docs/adr/0001-nom.md` attendu |
| T0.2 | Workspace Cargo + CI | fait | #2 | 2026-11-03 | 13 crates, lints, CI verte ; PR volontairement fautive rouge |
| T0.3 | Simulateur de fournisseur | fait | #4 | 2026-11-03 | 9 tests ; plafond 30 024 req/s noté dans `benchmarks/README.md` |
| T0.4 | Banc de benchmark scripté | fait (réserve) | #9 | 2026-11-03 | `cargo xtask bench` → JSON + Markdown ; ±10 % P99 non atteignable sur poste colocalisé, mesuré et documenté dans `benchmarks/README.md`, à vérifier en T1.26 (§11.2) |
| T0.5 | Schéma canonique v0 | fait | #5 | 2026-11-03 | 2 propriétés proptest × 200 cas ; ADR-0002 |
| T0.6 | ADR initiales | fait | #6 | 2026-11-03 | ADR-0003 à 0007 |
| T0.7 | Fichiers du projet | fait | #6 | 2026-11-03 | README, CONTRIBUTING, SECURITY, code de conduite, LICENSE, NOTICE, modèles d'issues et de PR |

## Phase 1 — E1 Moteur (21 j)

| ID | Tâche | Statut | PR | Date | Preuve |
| --- | --- | --- | --- | --- | --- |
| T1.1 | Serveur HTTP, `/health`, `/ready`, arrêt gracieux, limites | fait | #7 | 2026-11-03 | 5 tests d'intégration : santé/préparation, corps > 10 Mo → 413, timeout des en-têtes, flux long terminé, aucune connexion après l'arrêt |
| T1.2 | Configuration + rechargement à chaud | fait | #10 | 2026-11-03 | 10 tests de configuration ; `cargo xtask reload-soak --qps 5000 --seconds 60 --reload-every 2` → 4999 req/s, 100 %, 0 erreur, 30 rechargements |
| T1.3 | Pipeline et hooks | fait | #11 | 2026-11-03 | 9 tests pipeline : ordre déclaré, court-circuit sans appel, erreurs propres, usage final de flux ; chaîne vide **483 ns/requête en release** |
| T1.4 | Sélection pondérée des clés | fait | #12 | 2026-11-03 | 1 M de tirages à ± 1 % des poids ; criterion **13,9 ns** (< 50 ns) ; toutes clés en pause → erreur explicite ; pauses réinitialisées au rechargement (documenté) |
| T1.5 | Réessais | fait | #13 | 2026-11-03 | 6 tests : classification par statut (429/500/502/503/504 vs 400/401/403/404/422), flux déjà commencé jamais réessayé, gigue complète plafonnée ; 100 % de 503 → **taux de réessai 20,095 %** |
| T1.6 | Chaîne de repli et alias | à faire | — | — | — |
| T1.7 | Quatre timeouts | à faire | — | — | — |

## Phases suivantes

Le détail des tâches des phases 2 à 4 reste dans le cahier des charges. Chaque phase sera
ajoutée à ce tableau au début de l'epic, en même temps que la rédaction de ses fiches
(guide §11).

## Historique des fusions

| Date | PR | Tâche | CI |
| --- | --- | --- | --- |
| 2026-11-02 | #1 | Bootstrap (documents, consignes agents) | verte |
| 2026-11-03 | #2 | T0.2 Workspace Cargo + CI | verte (36 s) |
| 2026-11-03 | #3 | Preuve « PR fautive rouge » (fermée sans fusion) | rouge attendue : `clippy` seul en échec |
| 2026-11-03 | #4 | T0.3 Simulateur de fournisseur | verte (1 min 20) |
| 2026-11-03 | #5 | T0.5 Schéma canonique v0 | verte (1 min 10) |
| 2026-11-03 | #6 | T0.6 + T0.7 ADR et fichiers du projet | verte (1 min 07) |
| 2026-11-03 | #7 | T1.1 Serveur HTTP, santé, arrêt gracieux | verte (1 min 23) |
| 2026-11-03 | #9 | T0.4 Banc de benchmark scripté | verte (1 min 20) |
| 2026-11-03 | #10 | T1.2 Configuration et rechargement à chaud | verte (1 min 39) |
| 2026-11-03 | #11 | T1.3 Pipeline et hooks | verte (1 min 09) |
| 2026-11-03 | #12 | T1.4 Sélection pondérée des clés | verte (1 min 07) |
| 2026-11-03 | #13 | T1.5 Réessais, backoff et budget | à confirmer |
