<!-- buNtu pull request template. One pull request, one task. -->

## Task and requirements

<!-- Task ID (T0.2, T1.4…) and the FR-*/NFR-* requirements it covers. -->

## What changed

<!-- Summary, files touched, and what was deliberately left out. -->

## Verification

<!-- Exact commands run and their result; paste the output. -->

## Definition of done (guide §5)

- [ ] CI verte : `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`,
      `cargo nextest run`, `cargo deny check`, `cargo audit`, couverture
- [ ] Chaque critère d'acceptation de la fiche a un test nommé qui le vérifie
- [ ] Les commandes spécifiques de la fiche ont été exécutées, sortie collée ci-dessus
- [ ] Benchmarks micro sans régression > 10 % (si la tâche touche un chemin chaud)
- [ ] Aucun test supprimé, ignoré (`#[ignore]`) ou affaibli ; aucun lint désactivé
- [ ] Aucun `unwrap()` / `expect()` / `panic!` / `unsafe` ajouté hors tests et invariants documentés
- [ ] Aucune dépendance ajoutée sans accord explicite
- [ ] Fichiers limités à la fiche ; aucune API de fournisseur sans source officielle citée
- [ ] Aucun secret, clé ou donnée personnelle ajouté
- [ ] Commits signés (`git commit -s`, DCO) et co-auteur requis
- [ ] Une pull request, une tâche : le critère d'acceptation est démontré ci-dessus
