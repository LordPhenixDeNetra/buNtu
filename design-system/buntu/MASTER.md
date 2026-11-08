# Système de design de la console — MASTER

Source de vérité visuelle de `ui/`. Une page qui s'en écarte le justifie dans
`design-system/buntu/pages/<page>.md`. Les jetons vivent dans `ui/src/app.css` : une page n'écrit
jamais de couleur, d'espacement ou de durée en dur.

## Direction

**Console d'ingénieur, calme et dense : « Swiss minimal » + grille bento + moniteur temps réel.**

- Public : des développeurs et opérateurs qui configurent des fournisseurs, suivent le trafic et
  diagnostiquent des erreurs. Ils veulent lire vite, agir sans détour et faire confiance.
- Référence d'ambiance : les consoles d'infrastructure modernes. Fond neutre, surfaces en couches
  fines, une seule couleur d'accent, typographie technique pour les identifiants, mouvements
  courts qui confirment une action.
- « Extraordinaire » ici signifie **impeccable et vivant**, pas décoratif : états de chargement
  en squelette, retours immédiats, raccourcis clavier, détails soignés (focus, survol,
  transitions). Aucun effet qui coûte en lisibilité ou en performance.

Issu du skill *UI UX Pro Max* (2026-11-08) : style « Minimalism & Swiss » et « Bento Box Grid »
(tableaux de bord, outils professionnels), produit « Developer Tool » (moniteur temps réel,
priorité au sombre), règles UX « couleur jamais seule », « retour après envoi », « mouvement
réduit ». **Écarté** : motifs de page marketing (FAQ, hero + CTA), « sombre uniquement » (le
clair reste un thème de premier rang), glassmorphism généralisé (lisibilité), animations GSAP
(dépendance).

## Thèmes

Les deux thèmes sont de premier rang. Par défaut, la console suit la préférence du système, et
le choix de l'utilisateur est mémorisé. Le sombre est soigné en priorité, car c'est l'usage
dominant des outils de développement.

| Jeton | Clair | Sombre | Rôle |
| --- | --- | --- | --- |
| `--color-bg` | `#f6f7f9` | `#0b0d10` | fond de page |
| `--color-surface` | `#ffffff` | `#12161b` | cartes, panneaux |
| `--color-surface-2` | `#eef1f4` | `#191e25` | zones secondaires, survol, en-têtes de tableau |
| `--color-border` | `#dfe3e8` | `#262c35` | séparateurs décoratifs |
| `--color-control` | `#7d8796` | `#5f6a79` | bordure des champs (≥ 3:1) |
| `--color-text` | `#111827` | `#e7eaee` | texte principal |
| `--color-muted` | `#525c6b` | `#9aa4b2` | texte secondaire |
| `--color-accent` | `#0b7350` | `#3ddc97` | action principale, lien, focus |
| `--color-on-accent` | `#ffffff` | `#04241a` | texte sur l'accent |
| `--color-brand` | `#6d4aff` | `#a78bff` | marque (logo, halo) — jamais pour une action |
| `--color-success` | `#0b7350` | `#3ddc97` | 2xx, fournisseur sain |
| `--color-info` | `#1f5fd1` | `#86b0ff` | information, réessai |
| `--color-warning` | `#8a5300` | `#f2b84b` | 429, dégradé |
| `--color-danger` | `#b4232b` | `#ff8080` | 4xx/5xx, suppression |

**Contrastes vérifiés (WCAG 2.2)** : tous les textes ≥ 5,1:1 sur `bg`, `surface` et
`surface-2`, dans les deux thèmes ; texte sur accent 5,86 (clair) et 9,34 (sombre) ; bordure
de champ ≥ 3,3:1 ; marque 5,15 (clair) et 7,21 (sombre). `--color-border` (1,29:1) reste
décoratif et ne délimite jamais seul un contrôle.

## Typographie

Polices du système, sans téléchargement : l'interface reste embarquée dans le binaire, sous
1 Mio gzip, et fonctionne hors ligne.

- Texte : `system-ui, -apple-system, "Segoe UI", Roboto, sans-serif`.
- Code et identifiants (modèles, `fournisseur/modèle`, ID de requête, aperçus de clé) :
  `ui-monospace, "Cascadia Code", "SF Mono", Consolas, monospace`, avec chiffres tabulaires.
- Échelle : 12 / 14 / **16 (base)** / 20 / 24 / 32 px ; interligne 1,5 (texte), 1,2 (titres) ;
  graisses 400, 500 et 600. Pas de texte courant sous 14 px ; 12 px réservé aux libellés.
- Option, avec accord (ajout d'environ 100 Kio) : Inter et JetBrains Mono auto-hébergées. Le
  skill propose IBM Plex Sans + JetBrains Mono, ou Fira Sans + Fira Code.

## Espacement, formes, profondeur

- Grille de 4 px, densité « tableau de bord » : `--space-1` 4, `-2` 8, `-3` 12, `-4` 16, `-5`
  24, `-6` 32, `-7` 48.
- Rayons : 6 (contrôles), 10 (cartes), 16 (tuiles bento), 999 (pastilles).
- Profondeur par couches et bordures fines ; ombres très légères en clair, quasi nulles en
  sombre, où l'on joue sur la luminosité des surfaces.
- Largeur utile : 80 rem maximum ; marge latérale de 16 px sur mobile.

## Mouvement

- Jetons : `--motion-fast` 120 ms (survol, pression), `--motion-base` 200 ms (apparition,
  repli), `--motion-slow` 320 ms (panneau, toast) ; courbe `--ease-out`.
- N'animer que `opacity` et `transform`, jamais la taille ni la position de mise en page.
- Squelettes animés pendant le chargement, à la place de « Chargement… » qui fait sauter la
  page.
- `prefers-reduced-motion: reduce` : transitions coupées, squelettes figés.

## Composants (classes partagées de `app.css`)

| Classe | Usage |
| --- | --- |
| `.card` | panneau standard (surface, bordure, rayon 10, padding 16–24) |
| `.bento` + `.tile` | grille de tuiles de l'accueil (1×1, `.tile-wide` 2×1) |
| `.badge` + `.badge-success` / `-info` / `-warning` / `-danger` / `-neutral` | statut : **toujours une icône ou un texte en plus de la couleur** |
| `.btn-secondary`, `.btn-danger`, `.btn-ghost`, `.btn-icon` | variantes ; `button` seul = action principale |
| `.field` | libellé + contrôle + aide/erreur sous le champ |
| `.mono` | identifiants, modèles, clés (police mono, chiffres tabulaires) |
| `.muted`, `.sr-only` | texte secondaire ; texte réservé aux lecteurs d'écran |
| `.skeleton` | bloc de chargement |
| `.toolbar`, `.stack`, `.cluster` | mises en page courantes |

## Règles d'interaction (non négociables)

1. **Accessibilité** : focus visible partout (anneau de 2 px, décalé de 2 px) ; tout bouton qui
   n'est qu'une icône a un nom accessible ; cible tactile ≥ 40 px ; navigation complète au
   clavier ; axe sans violation sérieuse en CI.
2. **Couleur jamais seule** : un statut porte une icône et/ou un texte (`200 OK`, `502`).
3. **Retour après chaque action** : bouton désactivé et indicateur pendant l'envoi, puis toast
   de succès (annoncé par `aria-live`, qui disparaît seul) ou erreur sous le champ concerné.
4. **Pas de rechargement qui efface** : après une action, mettre à jour la liste sur place,
   garder le défilement et rendre le focus à un endroit logique.
5. **Focus déplacé** vers ce qui s'ouvre (détail, dialogue), puis rendu à l'élément d'origine.
6. **Une seule façon d'éditer** une même donnée ; un état non enregistré est visible.
7. **Destruction confirmée** (fournisseur, clé, route) par un dialogue accessible, pas par
   `window.confirm`.
8. **Icônes SVG**, jamais d'emoji : trait 1,5 px, grille 24 × 24, `currentColor`,
   `aria-hidden="true"` quand elles sont décoratives. Dessin maison ou bibliothèque (Lucide,
   Phosphor) seulement avec accord (règle « aucune dépendance sans accord »).
9. **Aucune chaîne visible en dur** : tout passe par `$t()` (FR/EN), y compris les libellés
   ARIA et les unités.
10. **Entrées issues du fichier de configuration** (`source: "file"`) : badge « fichier » et
    actions de modification désactivées, avec une explication.

## Structure de la console (cible)

- **Barre latérale** sur ordinateur (marque, navigation, thème, langue) ; barre supérieure
  repliable sur mobile.
- **Vue d'ensemble** (accueil, bento) : santé, requêtes récentes, taux d'erreur, latence,
  fournisseurs et routes, dernières erreurs cliquables vers le journal.
- **Fournisseurs**, **Routes** (éditeur de chaîne unique, glisser ou ↑↓ au clavier), **Journal**
  (tableau dense, date lisible, statut en badge, détail en panneau latéral).
- Plus tard : palette de commandes `Ctrl/⌘ + K` (aller à une page, créer un fournisseur ou une
  route).

## Marque

Le nom affiché (« buNtu ») et le logo dépendent de la décision T0.1 (ADR-0001) : ils restent
isolés dans le composant de navigation et dans `--color-brand`, pour qu'un changement de nom ne
touche ni les jetons ni les pages.
