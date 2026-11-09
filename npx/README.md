# `@buntuu/cli` — lanceur npm

Télécharge le binaire natif correspondant à la plateforme depuis la release GitHub
`v<version>` (même version que ce paquet), **vérifie la somme SHA-256** publiée, puis exécute
le binaire avec les arguments reçus.

```sh
npx @buntuu/cli --version
npx @buntuu/cli serve --listen 0.0.0.0:8080

# ou installation globale : la commande s'appelle `buntu`
npm install -g @buntuu/cli
buntu --version
```

- Plateformes publiées : Linux x86-64/ARM64 (musl), macOS ARM64, Windows x86-64. Il n'y a pas
  de binaire macOS Intel.
- Cache : `~/.cache/buntu/<version>/` ; un binaire dont la somme ne correspond plus est
  retéléchargé.
- Aucun binaire n'est exécuté avant la vérification de l'empreinte.
- Paquet préfixé (`@buntuu/`) : npm refuse le nom `buntu` seul, jugé trop proche de `bun`.
