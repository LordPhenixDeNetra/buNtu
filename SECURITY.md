# Politique de sécurité

## Statut du projet

buNtu est en **pré-alpha** : aucune version n'est publiée et aucune fonctionnalité n'est
utilisable en production. Le socle actuel (workspace, CI, simulateur de fournisseur, schéma
canonique) ne traite ni clé API réelle ni trafic utilisateur.

## Signaler une vulnérabilité

N'ouvrez **pas** d'issue publique. Utilisez la fonctionnalité *Report a vulnerability* de
l'onglet **Security** du dépôt GitHub
(<https://github.com/LordPhenixDeNetra/buNtu/security/advisories/new>).

Merci d'inclure : version ou commit concerné, description, scénario de reproduction, impact
estimé, et toute preuve utile. Ne joignez jamais de secret réel.

## Nos engagements

Conformément au cahier des charges (§8.2) :

- accusé de réception sous **72 heures** ;
- correctif pour une vulnérabilité critique sous **7 jours** ;
- divulgation coordonnée : nous convenons ensemble d'une date publique, généralement après
  publication du correctif.

## Périmètre

Sont concernés, dès qu'ils existeront : le binaire `buntu`, l'API HTTP, l'interface web, le
stockage (clés fournisseurs, clés virtuelles, journaux) et la chaîne de publication
(dépendances, images conteneur, artefacts signés).

Les dépendances amont sont suivies par `cargo audit` et `cargo deny` en CI ; merci de signaler
aussi toute dépendance douteuse.
