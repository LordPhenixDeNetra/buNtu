# Versions et releases

Procédure de publication de buNtu. Règles : cahier §12 ; outillage : [ADR-0009](adr/0009-versions.md).

## Règles de version

- **SemVer strict** (`MAJEUR.MINEUR.CORRECTIF`), une seule version pour tout le dépôt : workspace
  Cargo, dépendances internes et lanceur npm (`npx/package.json`). `node scripts/release.mjs
  check` le vérifie, en CI à chaque PR.
- **Avant 1.0**, une mineure (`0.x.0`) peut casser l'API ou la configuration ; elle est alors
  accompagnée d'un guide de migration dans le CHANGELOG. Un correctif (`0.x.y`) ne casse jamais.
- **À partir de 1.0** : API HTTP publique et format de configuration stables sur toute la série
  1.x ; une dépréciation est annoncée au moins deux mineures à l'avance.
- **Pré-releases** : `X.Y.Z-alpha.N` (fonctionnalités incomplètes), `X.Y.Z-rc.N` (candidate).
  cargo-dist les publie comme *pre-release* GitHub, d'après le numéro de version.
- **Rythme visé** : un correctif quand nécessaire, une mineure toutes les 6 à 8 semaines.
- **Support** : dernière mineure, plus les correctifs de sécurité sur la précédente pendant 3 mois.

Jalons (cahier §14) : **v0.1** alpha publique (OpenAI, Anthropic, compatibles OpenAI) ;
**v0.5** MVP ; **v1.0** premier engagement de stabilité.

## Ce qui est automatique

Un tag `vX.Y.Z` poussé sur GitHub déclenche :

1. **Release** (`release.yml`, généré par cargo-dist) : binaires Linux x86-64/ARM64 (musl),
   macOS ARM64 et Windows x86-64, installateurs shell et PowerShell, sommes SHA-256, release
   GitHub avec les notes reprises de `CHANGELOG.md`.
2. **Release delivery** (`release-delivery.yml`, à la fin de Release) : image multi-arch sur
   GHCR (`X.Y.Z`, `X.Y`, `X`, `latest`) avec SBOM et provenance, signature cosign, binaires
   bruts pour le lanceur npm, test de fumée du binaire publié (`--version`, `/ready`),
   publication de `npx buntu` (si `NPM_TOKEN` est configuré).

crates.io n'est **pas** publié pour l'instant (décision du 2026-11-08 : un nom publié ne se
libère pas, on attend la validation du nom, T0.1).

## Publier une version

### 1. Prérequis (bloquants)

- [ ] `main` est verte en CI.
- [ ] **Avant la v0.1.0 : nom validé (T0.1, [ADR-0001](adr/0001-nom.md))** — recherche de marques
  faite, risque « Ubuntu » tranché.
- [ ] Mineure : campagne de benchmark macro passée (cahier §11), sans recul > 10 %.
- [ ] `docs/status.md` et la case du cahier à jour pour les tâches livrées.

### 2. Préparer (sur une branche `release/vX.Y.Z`)

```sh
git switch main && git pull
git switch -c release/vX.Y.Z
node scripts/release.mjs bump X.Y.Z    # versions, Cargo.lock, CHANGELOG.md (git-cliff)
```

Relire `CHANGELOG.md` : reformuler si besoin, ajouter le guide de migration d'une mineure qui
casse. `git-cliff` s'installe avec `cargo install git-cliff --locked`.

Ouvrir la PR `chore(release): vX.Y.Z` et la fusionner quand la CI est verte.

### 3. Taguer (après fusion)

```sh
git switch main && git pull
node scripts/release.mjs check vX.Y.Z
git tag -a vX.Y.Z -m "vX.Y.Z"
git push origin vX.Y.Z
```

### 4. Vérifier

- [ ] Workflows **Release** puis **Release delivery** verts.
- [ ] Release GitHub : 5 archives, sommes `.sha256` et leurs signatures `.sigstore.json`.
- [ ] `docker run --rm ghcr.io/lordphenixdenetra/buntu:X.Y.Z --version`.
- [ ] `npx buntu@X.Y.Z --version` (si le lanceur npm est publié).
- [ ] `docs/status.md` : ligne de la release dans l'historique.

## Correctif et retrait

- **Correctif** : brancher depuis le tag (`git switch -c release/vX.Y.(Z+1) vX.Y.Z`), corriger,
  puis suivre la même procédure. Le correctif est aussi porté sur `main`.
- **Release défectueuse** : ne jamais réutiliser un numéro. Publier un correctif ; marquer la
  release GitHub fautive comme *pre-release* avec un avertissement en tête des notes.
- **Tag poussé par erreur** (workflow pas encore terminé) : annuler le workflow, puis
  `git push --delete origin vX.Y.Z` et supprimer la release brouillon.
