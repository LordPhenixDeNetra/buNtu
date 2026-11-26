# Système de design de la console — MASTER

Source de vérité visuelle de `ui/`. Une page qui s'en écarte le justifie dans
`design-system/buntu/pages/<page>.md`. Les jetons vivent dans `ui/src/app.css` : une page n'écrit
jamais de couleur, d'espacement ou de durée en dur.

## Direction (refonte du 2026-11-25)

**Console développeur sombre et dense, au niveau de Vercel, Linear, Railway ou Supabase.**

- Public : des développeurs et opérateurs qui configurent des fournisseurs, suivent le trafic et
  diagnostiquent des erreurs. Ils veulent lire vite, agir sans détour et faire confiance.
- Signature visuelle : fond presque noir, surfaces en couches fines séparées par des bordures
  de 1 px, une seule couleur d'action (le violet de la marque), typographie Geist, identifiants
  en Geist Mono, halo violet très discret en haut de page.
- « Cool » veut dire **précis et vivant**, pas décoratif : point d'état qui pulse quand la
  passerelle est saine, palette `Ctrl/⌘ + K`, raccourcis visibles (`kbd`), squelettes de
  chargement, transitions courtes. Aucun effet qui coûte en lisibilité ou en performance.

Issu du skill *UI UX Pro Max* (2026-11-25, `--design-system`, densité 8, variation 6,
mouvement 5) : style « Dark Mode (OLED) » et « Data-Dense Dashboard », produit « Developer
Tool » (moniteur temps réel), typographie technique. Choix du mainteneur : polices Geist et
Geist Mono, sombre par défaut, accent violet de la marque. **Écarté** : le motif de page
« FAQ/Documentation » proposé par le skill (hors sujet pour une console), le vert « run » comme
accent (il se confondait avec le statut 2xx), les animations GSAP (dépendance).

## Thèmes

Le **sombre est le thème par défaut**, quel que soit le thème du système ; le clair reste un
thème de premier rang, soigné et testé (axe en CI dans les deux). Le choix est mémorisé, et
`app.html` l'applique avant le premier rendu : jamais d'éclair de l'autre thème.

| Jeton | Sombre | Clair | Rôle |
| --- | --- | --- | --- |
| `--color-bg` | `#09090b` | `#fafafa` | fond de page |
| `--color-surface` | `#111113` | `#ffffff` | cartes, panneaux, champs |
| `--color-surface-2` | `#18181b` | `#f4f4f5` | en-têtes de tableau, pieds de carte, survol |
| `--color-surface-3` | `#222226` | `#ebebee` | élément actif, survol appuyé |
| `--color-border` | `#27272a` | `#e4e4e7` | séparateurs (décoratifs) |
| `--color-border-strong` | `#3a3a40` | `#d4d4d8` | boutons secondaires, pastilles d'icône |
| `--color-control` | `#63636d` | `#8a8a94` | bordure des champs (≥ 3:1) |
| `--color-text` | `#f4f4f5` | `#09090b` | texte principal |
| `--color-muted` | `#a1a1aa` | `#52525b` | texte secondaire |
| `--color-accent` | `#6d4aff` | `#6d4aff` | fond des actions principales |
| `--color-on-accent` | `#ffffff` | `#ffffff` | texte sur l'accent |
| `--color-accent-text` | `#a594ff` | `#5a37f0` | liens, focus, élément actif |
| `--color-success` | `#34d399` | `#116b34` | 2xx, passerelle saine |
| `--color-info` | `#60a5fa` | `#1d4ed8` | information |
| `--color-warning` | `#fbbf24` | `#8a5300` | 429, non enregistré |
| `--color-danger` | `#f87171` | `#c01d1d` | 4xx/5xx, suppression |

**Contrastes vérifiés (WCAG 2.2, calculés le 2026-11-25)** :

- textes et statuts ≥ 5,1:1 sur `bg`, `surface`, `surface-2` et `surface-3`, dans les deux
  thèmes (texte 14,4 à 19,9 ; secondaire 6,2 à 7,8) ;
- texte sur accent : 5,15:1 ;
- bordure de champ ≥ 3,18:1.

`--color-border` reste décoratif et ne délimite jamais seul un contrôle.

**Badges sur fond teinté** : le texte d'un badge est la couleur du statut mélangée à 80 % avec
`--color-text`, ce qui garde ≥ 5:1 sur toutes les surfaces.

## Typographie

- Texte : **Geist** ; code et identifiants (modèles, `fournisseur/modèle`, ID de requête,
  aperçus de clé, dates du journal) : **Geist Mono**, chiffres tabulaires.
- Auto-hébergées (`ui/src/lib/fonts/`, licence OFL 1.1 jointe), sous-ensemble latin seulement
  (accents français compris) : 52 Kio pour les deux, aucun appel à un service tiers, la
  console reste embarquée dans le binaire et fonctionne hors ligne.
- Échelle : 12 / **14 (base de l'interface)** / 16 / 18 / 24 / 30 px ; interligne 1,5 ;
  graisses 400, 500 et 600 ; titres resserrés (−0,015 à −0,03 em).
- Pas de texte sous 12 px ; 12 px réservé aux libellés, pieds d'indicateur et badges.

## Espacement, formes, profondeur

- Grille de 4 px, densité « tableau de bord » : `--space-1` 4, `-2` 8, `-3` 12, `-4` 16,
  `-5` 24, `-6` 32, `-7` 48.
- Contrôles de 36 px à la souris, **44 px au doigt** (`pointer: coarse`).
- Rayons : 8 (contrôles), 12 (cartes et tuiles), 999 (pastilles).
- Profondeur par couches et bordures fines ; en sombre, un liseré lumineux de 1 px en haut des
  cartes ; ombre portée réservée aux éléments flottants (dialogue, palette, toast).
- Largeur utile : 76 rem ; barre latérale de 15 rem à partir de 64 rem ; marge de 16 px sur
  mobile.

## Mouvement

- Jetons : `--motion-fast` 120 ms (survol, pression), `--motion-base` 200 ms (apparition),
  `--motion-slow` 320 ms (panneau, toast) ; courbe `--ease-out` (expo).
- N'animer que `opacity` et `transform`.
- Squelettes animés pendant le chargement ; point d'état qui pulse quand la passerelle est
  saine.
- `prefers-reduced-motion: reduce` : transitions coupées, squelettes et pulsation figés.

## Composants (classes partagées de `app.css`)

| Classe | Usage |
| --- | --- |
| `.page-header` | titre de page (`h1`), description, actions à droite |
| `.section-title` | titre de section avec icône et compteur |
| `.card`, `.card-header`, `.card-body`, `.card-footer`, `.card-pad` | panneau standard |
| `.bento` + `.tile` | grille de tuiles (indicateurs, raccourcis) |
| `.kpi-label`, `.kpi-value`, `.kpi-foot` | indicateur chiffré |
| `.icon-tile` | pastille d'icône (fournisseur, route, état vide) |
| `.badge` + `-success` / `-info` / `-warning` / `-danger` / `-brand` | statut : **toujours une icône ou un texte en plus de la couleur** |
| `.status-dot` (`.ok`, `.ko`) | point d'état, pulsation quand sain |
| `button` (action principale), `.btn` (lien en bouton), `.btn-secondary`, `.btn-ghost`, `.btn-danger`, `.btn-icon` | boutons |
| `.field`, `.field-hint`, `.field-error`, `.form-grid` | formulaires |
| `.table-wrap` + `table` | tableau dense, en-tête collant |
| `.empty` | état vide (pastille d'icône + phrase) |
| `.eyebrow`, `.mono`, `.muted`, `.small`, `.sr-only` | texte |
| `.skeleton` | bloc de chargement |
| `.stack`, `.cluster`, `.inline`, `.spacer`, `.toolbar` | mises en page courantes |

## Règles d'interaction (non négociables)

1. **Accessibilité** : focus visible partout (anneau de 2 px, ou halo de 3 px sur les
   champs) ; tout bouton qui n'est qu'une icône a un nom accessible ; **le nom accessible
   reprend le texte visible** (WCAG 2.5.3) ; navigation complète au clavier ; axe sans
   violation sérieuse en CI, en sombre et en clair.
2. **Couleur jamais seule** : un statut porte une icône et/ou un texte (`200`, `502`).
3. **Retour après chaque action** : bouton désactivé pendant l'envoi, puis toast de succès
   (annoncé par `aria-live`, qui disparaît seul) ou erreur sous le champ concerné.
4. **Pas de rechargement qui efface** : après une action, mettre à jour la liste sur place.
5. **Focus déplacé** vers ce qui s'ouvre (détail, dialogue, palette), puis rendu à l'élément
   d'origine.
6. **Une seule façon d'éditer** une même donnée ; un état non enregistré est visible.
7. **Destruction confirmée** (fournisseur, clé, route) par un dialogue accessible.
8. **Icônes SVG Lucide**, jamais d'emoji : `currentColor`, 14 à 16 px dans l'interface,
   décoratives quand un texte les accompagne.
9. **Aucune chaîne visible en dur** : tout passe par `$t()` (FR/EN), y compris les libellés
   ARIA et les unités.
10. **Entrées issues du fichier de configuration** (`source: "file"`) : badge « fichier » et
    actions de modification désactivées, avec une explication.

## Structure de la console

- **Barre latérale** (≥ 64 rem) :
  - marque ;
  - faux champ « Rechercher… Ctrl K » qui ouvre la palette ;
  - navigation, où l'élément actif est signalé par un trait violet ;
  - en pied, l'état de la passerelle en direct (`/ready`, toutes les 30 s), la langue et le
    thème.
- **Mobile** : barre du haut collante et floutée (marque, recherche, langue, thème), puis
  navigation en icônes.
- **Vue d'ensemble** :
  - tuiles santé, fournisseurs et routes ;
  - section « Trafic récent » de quatre indicateurs calculés par le serveur
    (`/admin/v1/stats`, 24 h) ;
  - carte des dernières erreurs.
- **Fournisseurs** : carte de création (en-tête, grille, pied avec rappel de sécurité), puis une
  carte par fournisseur (URL de base à gauche, clés et poids à droite).
- **Routes** : carte de création, puis chaque chaîne de repli en pipeline numéroté
  (« Principale », « Repli ») ; l'ajout de cible et l'enregistrement sont dans le pied.
- **Journal** : barre de filtres, tableau dense, détail en panneau latéral.

## Marque

Le nom affiché (« buNtu ») et le logo dépendent de la décision T0.1 (ADR-0001) : ils restent
isolés dans le composant de navigation et dans `--color-brand`, pour qu'un changement de nom ne
touche ni les jetons ni les pages.
