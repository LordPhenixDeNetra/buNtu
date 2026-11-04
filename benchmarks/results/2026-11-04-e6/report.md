# Rapport de benchmark

- Généré : `2026-11-04T19:24Z` (epoch 1793820249)
- Machine : windows x86_64 — 8 cœurs logiques
- Outils : oha 1.16.0 ; rustc 1.99.1 (58cf8f9c3 2026-10-13)
- Protocole : boucle ouverte (`oha -q`), chauffe 3 s, paliers 8 s, 3 exécution(s), QPS [500, 1000, 2000]

> Mesure locale : générateur et serveur sur la même machine. Ces chiffres sont un
> plancher, pas la mesure publiée du cahier §11.2 (machines séparées).

## Mesures

| Cible | QPS | Exécution | req/s | Succès | P50 | P90 | P99 | P99,9 | RSS | CPU |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| simulator | 500 | 1 | 499 | 100.00 % | 0.42 ms | 0.66 ms | 1.78 ms | 3.08 ms | n/a | n/a |
| simulator | 1000 | 1 | 1000 | 100.00 % | 0.51 ms | 0.90 ms | 1.55 ms | 3.05 ms | n/a | n/a |
| simulator | 2000 | 1 | 1995 | 100.00 % | 0.74 ms | 1.41 ms | 2.19 ms | 3.14 ms | n/a | n/a |
| gateway | 500 | 1 | 500 | 100.00 % | 1.09 ms | 1.68 ms | 2.89 ms | 16.65 ms | n/a | n/a |
| gateway | 1000 | 1 | 998 | 100.00 % | 1.61 ms | 2.48 ms | 3.51 ms | 5.00 ms | n/a | n/a |
| gateway | 2000 | 1 | 1996 | 100.00 % | 2.46 ms | 3.48 ms | 4.86 ms | 24.52 ms | n/a | n/a |
| simulator | 500 | 2 | 499 | 100.00 % | 0.42 ms | 0.65 ms | 1.11 ms | 3.98 ms | n/a | n/a |
| simulator | 1000 | 2 | 998 | 100.00 % | 0.50 ms | 0.86 ms | 1.44 ms | 3.51 ms | n/a | n/a |
| simulator | 2000 | 2 | 1998 | 100.00 % | 0.74 ms | 1.43 ms | 2.24 ms | 4.92 ms | n/a | n/a |
| gateway | 500 | 2 | 499 | 100.00 % | 1.06 ms | 1.70 ms | 2.44 ms | 4.09 ms | n/a | n/a |
| gateway | 1000 | 2 | 998 | 100.00 % | 1.57 ms | 2.44 ms | 3.18 ms | 4.23 ms | n/a | n/a |
| gateway | 2000 | 2 | 1996 | 100.00 % | 2.50 ms | 3.53 ms | 4.52 ms | 10.08 ms | n/a | n/a |
| simulator | 500 | 3 | 500 | 100.00 % | 0.42 ms | 0.68 ms | 1.70 ms | 2.89 ms | n/a | n/a |
| simulator | 1000 | 3 | 998 | 100.00 % | 0.53 ms | 0.95 ms | 1.78 ms | 3.09 ms | n/a | n/a |
| simulator | 2000 | 3 | 1999 | 100.00 % | 0.71 ms | 1.42 ms | 2.26 ms | 5.61 ms | n/a | n/a |
| gateway | 500 | 3 | 499 | 100.00 % | 1.12 ms | 1.70 ms | 2.73 ms | 6.61 ms | n/a | n/a |
| gateway | 1000 | 3 | 998 | 100.00 % | 1.76 ms | 2.44 ms | 3.67 ms | 10.72 ms | n/a | n/a |
| gateway | 2000 | 3 | 1996 | 100.00 % | 2.51 ms | 3.48 ms | 4.36 ms | 19.40 ms | n/a | n/a |

## Surcoût de la passerelle

Surcoût = latence via `/v1/chat/completions` − latence directe vers le simulateur, mêmes
requêtes et même palier. Cible du cahier (NFR-PERF-1) : P50 ≤ 20 µs, P99 ≤ 100 µs.

| QPS | P50 médian (µs) | P99 médian (µs) | dispersion P50 | dispersion P99 |
| --- | --- | --- | --- | --- |
| 500 | 666 | 1115 | 9 % | 27 % |
| 1000 | 1100 | 1885 | 15 % | 12 % |
| 2000 | 1758 | 2272 | 4 % | 25 % |

## Mesures statiques

| Métrique | Valeur | Cible cahier |
| --- | --- | --- |
| Taille du binaire passerelle | 4.5 Mio | ≤ 40 Mio (NFR-PERF-11) |
| Démarrage à froid jusqu'à `/ready` (médiane) | 523 ms | ≤ 200 ms (NFR-PERF-9) |

## Reproductibilité des P99

Tolérance : 10 % d'écart relatif entre la plus petite et la plus grande valeur.

| Cible | QPS | P99 par exécution (ms) | Écart | Verdict |
| --- | --- | --- | --- | --- |
| simulator | 500 | 1.78 / 1.11 / 1.70 | 60.4 % | échec |
| simulator | 1000 | 1.55 / 1.44 / 1.78 | 23.8 % | échec |
| simulator | 2000 | 2.19 / 2.24 / 2.26 | 3.4 % | ok |
| gateway | 500 | 2.89 / 2.44 / 2.73 | 18.8 % | échec |
| gateway | 1000 | 3.51 / 3.18 / 3.67 | 15.4 % | échec |
| gateway | 2000 | 4.86 / 4.52 / 4.36 | 11.4 % | échec |

**Verdict : non reproductible — voir les lignes en échec.**

## Limites

- Mesure locale sur une seule machine (générateur, simulateur et passerelle colocalisés) : ce n'est **pas** le protocole §11.2 du cahier (deux machines + simulateur sur une troisième). Les chiffres servent de plancher et de suivi de régression.
- Paliers élevés (5 000 à 20 000 req/s) non couverts par cette machine.
- RSS et CPU : collectés via `/proc` sous Linux ; `n/a` sur les autres systèmes.
