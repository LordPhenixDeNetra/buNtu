# Benchmarks

## T0.3 — plafond du simulateur de fournisseur

Le simulateur ne doit jamais être le goulot des mesures (cahier §11.2). Ce fichier consigne son
débit mesuré et le protocole exact, pour que toute campagne ultérieure puisse vérifier que la
charge demandée reste sous ce plafond.

### Environnement de mesure

- Windows 11 Professionnel 10.0.26200
- 8 processeurs logiques, 32 Go de RAM
- Générateur `oha` 1.16.0 et simulateur **sur la même machine** (le client consomme donc une
  partie des cœurs)
- Binaire `mock-provider` compilé en `--release` (profil du workspace : LTO fat,
  `codegen-units = 1`, `panic = "abort"`), taille ≈ 759 Ko

### Commande

```sh
MOCK_LISTEN=127.0.0.1:18080 target/release/mock-provider &
oha -z 3s -c 64 -m POST -H "content-type: application/json" \
  -d '{"stream":false}' --no-tui http://127.0.0.1:18080/v1/chat/completions   # chauffe
oha -z 10s -c 64 -w -m POST -H "content-type: application/json" \
  -d '{"stream":false}' --no-tui http://127.0.0.1:18080/v1/chat/completions
```

### Résultat (3 novembre 2026)

| Métrique | Valeur |
| --- | --- |
| Débit | **30 024 req/s** |
| Succès | **100 %**, 300 244 réponses `200` |
| Erreurs | 0 |
| P50 | 1,95 ms |
| P99 | 5,33 ms |
| P99,9 | 8,09 ms |

Concurrence retenue : 64 connexions. Mesures complémentaires : `c=256` → 24 004 req/s,
`c=512` → 19 870 req/s (générateur et serveur se disputent les mêmes cœurs).

### Limite de l'exercice

La fiche T0.3 vise **50 000 req/s sur 4 vCPU** avec un générateur séparé. Ici le générateur et
le simulateur partagent une seule machine de 8 cœurs logiques : les 30 024 req/s sont un
**plancher** mesuré dans ce contexte, pas la mesure officielle. La campagne publiée (T0.4, puis
T1.26) devra rejouer le protocole du cahier §11.2 (deux machines identiques dans la même zone,
fournisseur simulé sur une troisième) et publier la médiane de 3 exécutions.
