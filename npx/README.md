# `npx buntu` — lanceur

Télécharge le binaire natif correspondant à la plateforme depuis la release GitHub
`v<version>` (même version que ce paquet), **vérifie la somme SHA-256** publiée, puis exécute
le binaire avec les arguments reçus.

```sh
npx buntu --version
npx buntu serve --listen 0.0.0.0:8080
```

- Plateformes : Linux x86-64/ARM64 (musl), macOS x86-64/ARM64, Windows x86-64.
- Cache : `~/.cache/buntu/<version>/` ; un binaire dont la somme ne correspond plus est
  retéléchargé.
- Aucun binaire n'est exécuté avant la vérification de l'empreinte.
