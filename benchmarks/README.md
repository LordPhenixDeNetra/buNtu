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

## T0.4 — banc de benchmark scripté

Le banc vit dans la crate `xtask` et se lance en **une commande** :

```sh
cargo xtask bench
```

Options : `--runs` (défaut 2), `--qps` (défaut `500,1000,2000`), `--seconds` (défaut 5),
`--warmup` (défaut 2), `--tolerance` (défaut `0.10`), `--output` (défaut
`benchmarks/results/<date>`).

### Protocole

1. compilation en `--release` du simulateur et de la passerelle ;
2. démarrage sur des ports éphémères (attente de `/ready` pour la passerelle) ;
3. chauffe, puis charge en **boucle ouverte** (`oha -q`) sur chaque palier ;
4. collecte : débit, taux de succès, P50/P90/P99/P99,9, RSS et CPU (`/proc` sous Linux) ;
5. écriture de `results.json` et `report.md` dans `benchmarks/results/<date>/` ;
6. **garde-fou de reproductibilité** : l'écart relatif des P99 entre deux exécutions doit rester
   ≤ `--tolerance` ; sinon la commande sort en erreur (l'échec est volontaire, il ne doit pas
   être contourné).

Cible `gateway` : mesure `/ready` pour l'instant ; le coût du routage `/v1/chat/completions`
sera mesuré après T1.13.

### Campagne locale du 3 novembre 2026 — preuve d'exécution, pas la mesure publiée

Commande : `cargo xtask bench --runs 2 --qps 2000,5000 --seconds 20 --warmup 5`
(résultats bruts dans [results/2026-11-03-t0.4/](results/2026-11-03-t0.4/)).

| Cible | QPS | P99 (exécution 1 / 2) | Écart | Verdict |
| --- | --- | --- | --- | --- |
| simulateur | 2000 | 2,09 / 13,82 ms | 561 % | échec |
| simulateur | 5000 | 3,10 / 35,74 ms | 1052 % | échec |
| passerelle | 2000 | 3,26 / 2,78 ms | 17 % | échec |
| passerelle | 5000 | 35,24 / 3,42 ms | 931 % | échec |

**Verdict : non reproductible sur ce poste** — la commande sort en erreur, comme prévu.

Explication : le générateur et le serveur partagent la même machine Windows ; les médianes sont
de l'ordre de 0,3 à 1,7 ms et les P99 sont dominés par des à-coups système rares (synchronisation
OneDrive, antivirus, ordonnanceur), d'où des écarts extrêmes. Le critère ± 10 % de la fiche
suppose le protocole du cahier §11.2 (générateur, passerelle et fournisseur simulé sur des
machines séparées, paliers de 5 min, médiane de 3 exécutions) : il sera vérifié par la campagne
publiée de T1.26. Le banc, ses options et son garde-fou sont livrés et fonctionnent ; c'est la
mesure qui n'est pas valable sur un poste de développement.

