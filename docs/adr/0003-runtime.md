# ADR-0003 — Runtime asynchrone : tokio

- **Statut :** accepté
- **Date :** 3 novembre 2026
- **Contexte :** cahier des charges §6.3 (choix techniques), T0.6.

## Contexte

La passerelle est limitée par le réseau et les connexions ouvertes, pas par le calcul : elle
doit tenir des dizaines de milliers de flux simultanés, avec des timeouts distincts et des
annulations propres. Le runtime doit être mature, compatible avec l'écosystème HTTP et ne pas
introduire de ramasse-miettes.

## Décision

**tokio multi-thread**, un thread worker par cœur, via `#[tokio::main]` et l'API standard de
tokio (`spawn`, `select!`, `time`, `sync`). Aucun runtime maison, aucun executor alternatif.

## Alternatives écartées

- **monoio / glommio (io_uring)** : performants sur Linux mais non portables (cibles macOS et
  Windows), écosystème incompatible ; réévalués en phase 4 (T4.3), et gardés seulement si le
  gain mesuré dépasse 15 %.
- **async-std** : maintenance arrêtée.
- **Threads + I/O bloquantes** : ne tient pas les flux simultanés du cahier (§5.1, NFR-PERF-4).

## Conséquences

- Toute la pile (hyper, axum, sqlx, tracing) est compatible tokio.
- Discipline requise : pas de blocage dans une tâche, pas de verrou tenu à travers un `.await`,
  futur `Send + 'static` pour les tâches détachées.
- L'annulation est coopérative : chaque boucle de streaming doit réagir à la fermeture du client.
- Le surcoût du runtime sera mesuré par le banc de T0.4/T1.26, pas supposé.
