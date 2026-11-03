# ADR-0005 — Stockage : SQLite par défaut, PostgreSQL en cluster

- **Statut :** accepté
- **Date :** 3 novembre 2026
- **Contexte :** cahier des charges §6.3 et §7 (données, stockage et configuration), T0.6.

## Contexte

Un nœud unique doit démarrer sans rien installer, tandis que plusieurs nœuds (phase 3) doivent
partager configuration et compteurs. La base ne doit jamais être lue sur le chemin chaud : on lit
au démarrage et au rechargement, tout le reste vit en mémoire.

## Décision

- **`sqlx`** avec requêtes vérifiées à la compilation (`cargo sqlx prepare`, mode hors ligne en
  CI), sans ORM.
- **SQLite en mode WAL** par défaut (un nœud, `busy_timeout`, pool), **PostgreSQL** dès que
  plusieurs nœuds partagent les données.
- **Migrations versionnées, en avant uniquement**, appliquées au démarrage sous verrou
  consultatif ; test de montée de version depuis chaque version publiée.
- Interface `Store` minimale ; les secrets sont chiffrés **au niveau applicatif**
  (AES-256-GCM) et la clé maître ne vit jamais en base.

## Alternatives écartées

- **sled / RocksDB** : pas de SQL, adéquation faible aux jointures de gouvernance et d'audit.
- **ORM complet** : magie d'exécution, requêtes non vérifiées, complexité inutile.
- **Base uniquement distante** : viole l'exigence « démarre sans rien » (NFR d'utilisabilité).

## Conséquences

- `Cargo.lock` doit rester à jour pour le mode hors ligne de SQLx.
- Les compteurs de budget seront incrémentés en mémoire puis vidés en base (cahier §7.3) ; en
  cluster, compteurs atomiques Redis.
- Les journaux de requêtes sont écrits hors du chemin critique (T1.18).
