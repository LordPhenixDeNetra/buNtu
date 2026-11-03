# Contribuer à buNtu

Merci de votre intérêt. Le projet est en pré-alpha : les contributions les plus utiles
aujourd'hui sont les corrections de documentation, les revues d'ADR et les tests.

En participant, vous acceptez le [code de conduite](CODE_OF_CONDUCT.md).

## Avant de commencer

1. Lisez [AGENTS.md](AGENTS.md) : il fixe les règles du dépôt (périmètre, tests d'abord,
   vérifications machine). Elles s'appliquent aux humains comme aux agents IA.
2. Lisez le [cahier des charges](<buNtu — Cahier des charges et cahier des tâches.md>) et le
   [guide d'exécution](<Guide d'exécution (agents IA).md>) pour le contexte.
3. Ouvrez une issue avant un changement de fond : une tâche par pull request.

## Environnement

- Rust stable, version fixée par `rust-toolchain.toml` (MSRV dans `Cargo.toml`).
- Outils : `cargo-nextest`, `cargo-deny`, `cargo-audit`, composant `llvm-tools-preview`.
- Vérification complète :

```sh
cargo fmt --check && cargo clippy --all-targets -- -D warnings \
  && cargo nextest run --no-tests=pass && cargo deny check && cargo audit
```

## Commits et signature (DCO)

- **Un commit par sujet**, messages en anglais au format Conventional Commits, avec la portée
  (`feat(core):`, `fix(gateway):`, `docs:`) et l'identifiant de tâche (`T1.4`, `T0.5`…) dans le
  corps.
- **Signature DCO obligatoire** : `git commit -s` ajoute `Signed-off-by`, qui atteste que vous
  avez le droit de contribuer sous la licence du projet. Aucun CLA n'est demandé.
- **Aucune attribution d'outil IA** dans les commits (`Co-Authored-By` d'un assistant, mention
  « generated with… ») : vous êtes l'auteur de votre contribution.
- Ne modifiez que le périmètre annoncé : une pull request, une tâche.

## Pull requests

- Suivez le [modèle de pull request](.github/pull_request_template.md) : tâche et exigences,
  ce qui a changé, vérifications lancées et leur résultat, liste cochée.
- La CI doit être verte ; un test qui échoue sans le changement est la règle (tests d'abord).
- Une PR qui désactive un lint, supprime un test ou baisse un seuil est refusée.

## Licence des contributions

En contribuant, vous acceptez que votre contribution soit distribuée sous Apache License 2.0
(voir [LICENSE](LICENSE) et [NOTICE](NOTICE)). Toute dépendance ajoutée doit passer
`cargo deny check` ; une licence hors liste est une décision explicite, à discuter en amont.
