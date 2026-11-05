# ADR-0008 — Configuration : trois sources, un seul modèle

- **Statut :** accepté
- **Date :** 5 novembre 2026
- **Contexte :** cahier des charges §7.1, revue du dépôt du 2026-11-05 (section 2).

## Contexte

Avant cette décision, la configuration vivait à trois endroits qui ne se parlaient pas : le
fichier TOML rechargé à chaud, une copie en mémoire modifiée par l'API d'administration, et les
tables `providers`, `provider_keys` et `routes` de SQLite, jamais lues. Un rechargement du
fichier effaçait les changements faits par l'API, et un redémarrage les perdait tous.

Le cahier (§7.1) fixe déjà le modèle : fichier, API d'administration (qui écrit en base) et
variables `BUNTU_*`, avec la priorité environnement > fichier > base, et un mode
`config.mode = "file"` qui rend la base en lecture seule pour le GitOps.

## Décision

- **Deux couches persistantes et une surcharge.** La couche *base* est écrite par l'API
  d'administration et persistée en SQLite. La couche *fichier* est `buntu.toml`, rechargée à
  chaud. L'environnement s'applique au moment de la compilation. `ConfigHandle` garde les deux
  couches et compile `base < fichier < environnement` (`compile_layers`).
- **Fusion par identifiant.** Un fournisseur (`id`) ou une route (`alias`) du fichier remplace
  l'entrée de même nom de la base. Les sections globales (`server`, `timeouts`, `hooks`,
  `config`) viennent du fichier seul. Les entrées du fichier gardent leurs positions, si bien
  que les index `BUNTU_PROVIDERS__0__…` visent toujours le fichier.
- **Le fichier est prioritaire.** L'API refuse de modifier une entrée définie dans le fichier :
  le changement serait invisible. Les listes de l'API indiquent la `source` de chaque entrée
  (`file` ou `database`).
- **Une mutation est validée, persistée, puis publiée.** Elle est validée par le même code que
  `buntu config validate`. Si l'une des étapes échoue, rien n'est publié. Un verrou d'écriture
  dans `ConfigHandle` empêche un rechargement et une mutation de publier une configuration
  compilée à partir d'une couche périmée.
- **Les secrets restent chiffrés au repos (T1.21).** Une clé littérale est chiffrée en
  AES-256-GCM. Une référence `env:VAR`, qui n'est pas un secret, est stockée telle quelle
  (migration `0003`). La clé maître vient de `BUNTU_MASTER_KEY` ou de `BUNTU_MASTER_KEY_FILE`.
  À défaut, un fichier `<base>.key` (0600, base64) est créé à côté de la base au premier
  démarrage. Elle n'est jamais écrite en base.
- **`[config] mode = "file"`** met l'API d'administration en lecture seule ; la base reste lue.

## Alternatives écartées

- **Le fichier comme source unique, réécrit par l'API.** C'est plus simple, mais cela s'écarte
  du §7.1, perd les commentaires du TOML à chaque écriture et ne fonctionne plus en cluster.
- **La base comme source unique, avec le fichier importé une fois.** Cela casse le GitOps : un
  fichier versionné qui n'a plus d'effet après le premier démarrage.
- **Tout stocker en un seul document de configuration en base.** Les tables existent déjà
  (T1.17) et préparent les jointures de gouvernance de la phase 2.

## Conséquences

- Les changements faits par l'API survivent aux rechargements et aux redémarrages.
- La clé maître doit être sauvegardée : sans elle, les clés chiffrées en base sont illisibles
  et le démarrage échoue explicitement.
- Une référence `env:VAR` enregistrée par l'API doit exister dans l'environnement : la
  validation commune la vérifie.
- En cluster (phase 3), la couche base passera sur PostgreSQL et sa diffusion entre nœuds
  reste à concevoir (`buntu-cluster`).
