# Rapport de benchmark

- Généré : `2026-11-03T16:51Z` (epoch 1793724715)
- Machine : windows x86_64 — 8 cœurs logiques
- Outils : oha 1.16.0 ; rustc 1.90.0 (1159e78c4 2025-09-14)
- Protocole : boucle ouverte (`oha -q`), chauffe 5 s, paliers 20 s, 2 exécution(s), QPS [2000, 5000]

> Mesure locale : générateur et serveur sur la même machine. Ces chiffres sont un
> plancher, pas la mesure publiée du cahier §11.2 (machines séparées).

## Mesures

| Cible | QPS | Exécution | req/s | Succès | P50 | P90 | P99 | P99,9 | RSS | CPU |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| simulator | 2000 | 1 | 1998 | 100.00 % | 0.69 ms | 1.37 ms | 2.09 ms | 5.62 ms | n/a | n/a |
| simulator | 5000 | 1 | 4999 | 100.00 % | 1.30 ms | 2.23 ms | 3.10 ms | 5.45 ms | n/a | n/a |
| gateway | 2000 | 1 | 1999 | 100.00 % | 0.71 ms | 1.40 ms | 3.26 ms | 5.51 ms | n/a | n/a |
| gateway | 5000 | 1 | 4998 | 100.00 % | 1.33 ms | 8.12 ms | 35.24 ms | 55.18 ms | n/a | n/a |
| simulator | 2000 | 2 | 1999 | 100.00 % | 0.64 ms | 1.39 ms | 13.82 ms | 37.39 ms | n/a | n/a |
| simulator | 5000 | 2 | 4996 | 100.00 % | 1.76 ms | 9.65 ms | 35.74 ms | 68.67 ms | n/a | n/a |
| gateway | 2000 | 2 | 1998 | 100.00 % | 0.66 ms | 1.63 ms | 2.78 ms | 9.45 ms | n/a | n/a |
| gateway | 5000 | 2 | 4996 | 100.00 % | 1.27 ms | 2.26 ms | 3.42 ms | 5.08 ms | n/a | n/a |

## Reproductibilité des P99

Tolérance : 10 % d'écart relatif entre la plus petite et la plus grande valeur.

| Cible | QPS | P99 par exécution (ms) | Écart | Verdict |
| --- | --- | --- | --- | --- |
| simulator | 2000 | 2.09 / 13.82 | 561.3 % | échec |
| simulator | 5000 | 3.10 / 35.74 | 1051.8 % | échec |
| gateway | 2000 | 3.26 / 2.78 | 17.2 % | échec |
| gateway | 5000 | 35.24 / 3.42 | 930.7 % | échec |

**Verdict : non reproductible — voir les lignes en échec.**

## Limites

- RSS et CPU : collectés via `/proc` sous Linux ; `n/a` sur les autres systèmes.
- Cible `gateway` : `/ready` pour l'instant ; le coût du routage `/v1/chat/completions` sera mesuré après T1.13.
