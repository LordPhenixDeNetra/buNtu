import { derived, get, writable } from 'svelte/store';

export type Locale = 'fr' | 'en';

/** Aucune chaîne visible en dur dans `ui/` (FR-UI-6) : tout passe par ce dictionnaire. */
const dictionary: Record<Locale, Record<string, string>> = {
  fr: {
    'app.title': 'buNtu — console',
    'a11y.skip': 'Aller au contenu',
    'nav.label': 'Navigation principale',
    'nav.overview': 'Vue d’ensemble',
    'nav.providers': 'Fournisseurs',
    'nav.routes': 'Routes',
    'nav.logs': 'Journal',
    'theme.light': 'Thème clair',
    'theme.dark': 'Thème sombre',
    'lang.label': 'Langue',
    'loading': 'Chargement…',
    'error.title': 'Erreur',
    'error.unexpected': 'Erreur inattendue',
    'retry': 'Réessayer',
    'common.cancel': 'Annuler',
    'common.close': 'Fermer',
    'common.confirm': 'Confirmer',
    'common.saving': 'Enregistrement…',
    'common.dismiss': 'Fermer la notification',
    'source.file': 'fichier',
    'source.database': 'console',
    'source.fileHint':
      'Défini dans le fichier de configuration, prioritaire : modifiez le fichier pour le changer.',

    'overview.title': 'Vue d’ensemble',
    'overview.subtitle': 'État de la passerelle et trafic récent.',
    'overview.health': 'Santé',
    'overview.ready': 'Prête',
    'overview.notReady': 'Indisponible',
    'overview.providers': 'Fournisseurs',
    'overview.routes': 'Routes',
    'overview.fromFile': 'du fichier',
    'overview.fromConsole': 'de la console',
    'overview.traffic': 'Trafic récent',
    'overview.sample': 'sur les {n} dernières requêtes',
    'overview.requests': 'Requêtes',
    'overview.errorRate': 'Taux d’erreur',
    'overview.latencyP50': 'Latence médiane',
    'overview.latencyP95': 'Latence p95',
    'overview.noTraffic': 'Aucune requête pour le moment.',
    'overview.recentErrors': 'Dernières erreurs',
    'overview.noErrors': 'Aucune erreur récente.',
    'overview.seeLogs': 'Voir le journal',

    'logs.title': 'Journal des requêtes',
    'logs.subtitle':
      'Requêtes servies, filtrables et paginées côté serveur ; les filtres sont dans l’URL.',
    'logs.provider': 'Fournisseur',
    'logs.status': 'Statut',
    'logs.model': 'Modèle',
    'logs.limit': 'Taille de page',
    'logs.filter': 'Filtrer',
    'logs.date': 'Date',
    'logs.id': 'Identifiant',
    'logs.latency': 'Latence',
    'logs.attempts': 'Tentatives',
    'logs.tokens': 'Jetons entrée / sortie',
    'logs.tokensShort': 'Jetons E/S',
    'logs.detail': 'Détail',
    'logs.next': 'Page suivante',
    'logs.previous': 'Page précédente',
    'logs.empty': 'Aucune requête pour ces filtres.',

    'providers.title': 'Fournisseurs',
    'providers.subtitle': 'Fournisseurs configurés, clés et URL de base.',
    'providers.empty': 'Aucun fournisseur pour le moment.',
    'providers.create': 'Ajouter un fournisseur',
    'providers.id': 'Identifiant',
    'providers.type': 'Type',
    'providers.baseUrl': 'URL de base',
    'providers.secret': 'Clé API (saisie unique)',
    'providers.keys': 'Clés',
    'providers.noKeys': 'Aucune clé.',
    'providers.addKey': 'Ajouter une clé',
    'providers.newKey': 'Nouvelle clé',
    'providers.keyPreview': 'Aperçu',
    'providers.weight': 'Poids',
    'providers.delete': 'Supprimer',
    'providers.confirmDelete': 'Supprimer ce fournisseur et ses clés ?',
    'providers.confirmDeleteKey': 'Retirer cette clé ? Les requêtes ne l’utiliseront plus.',
    'providers.secretNotice':
      'La clé n’est jamais réaffichée : seuls un aperçu et son poids sont conservés.',
    'providers.save': 'Enregistrer',
    'providers.deleteKey': 'Retirer la clé',
    'providers.created': 'Fournisseur créé.',
    'providers.updated': 'URL de base enregistrée.',
    'providers.deleted': 'Fournisseur supprimé.',
    'providers.keyAdded': 'Clé ajoutée : elle ne sera plus jamais affichée en clair.',
    'providers.keyRemoved': 'Clé retirée.',

    'routes.title': 'Routes et alias',
    'routes.subtitle': 'Chaînes de repli par alias de modèle.',
    'routes.empty': 'Aucune route pour le moment.',
    'routes.create': 'Créer une route',
    'routes.alias': 'Alias',
    'routes.fallbacks': 'Chaîne de repli (un « fournisseur/modèle » par ligne)',
    'routes.chain': 'Chaîne de repli',
    'routes.moveUp': 'Monter',
    'routes.moveDown': 'Descendre',
    'routes.removeTarget': 'Retirer la cible',
    'routes.newTarget': 'Nouvelle cible (fournisseur/modèle)',
    'routes.addTarget': 'Ajouter la cible',
    'routes.unsaved': 'Modifications non enregistrées',
    'routes.discard': 'Annuler les modifications',
    'routes.delete': 'Supprimer',
    'routes.confirmDelete': 'Supprimer cette route ?',
    'routes.save': 'Enregistrer la chaîne',
    'routes.created': 'Route créée.',
    'routes.saved': 'Chaîne de repli enregistrée.',
    'routes.deleted': 'Route supprimée.',

    'auth.title': 'Jeton d’administration',
    'auth.help':
      'L’API d’administration est protégée. Collez le jeton affiché au premier démarrage de buntu (ou la valeur de BUNTU_ADMIN_TOKEN).',
    'auth.token': 'Jeton',
    'auth.submit': 'Valider'
  },
  en: {
    'app.title': 'buNtu — console',
    'a11y.skip': 'Skip to content',
    'nav.label': 'Main navigation',
    'nav.overview': 'Overview',
    'nav.providers': 'Providers',
    'nav.routes': 'Routes',
    'nav.logs': 'Request log',
    'theme.light': 'Light theme',
    'theme.dark': 'Dark theme',
    'lang.label': 'Language',
    'loading': 'Loading…',
    'error.title': 'Error',
    'error.unexpected': 'Unexpected error',
    'retry': 'Retry',
    'common.cancel': 'Cancel',
    'common.close': 'Close',
    'common.confirm': 'Confirm',
    'common.saving': 'Saving…',
    'common.dismiss': 'Dismiss notification',
    'source.file': 'file',
    'source.database': 'console',
    'source.fileHint':
      'Defined in the configuration file, which takes precedence: edit the file to change it.',

    'overview.title': 'Overview',
    'overview.subtitle': 'Gateway health and recent traffic.',
    'overview.health': 'Health',
    'overview.ready': 'Ready',
    'overview.notReady': 'Unavailable',
    'overview.providers': 'Providers',
    'overview.routes': 'Routes',
    'overview.fromFile': 'from the file',
    'overview.fromConsole': 'from the console',
    'overview.traffic': 'Recent traffic',
    'overview.sample': 'over the last {n} requests',
    'overview.requests': 'Requests',
    'overview.errorRate': 'Error rate',
    'overview.latencyP50': 'Median latency',
    'overview.latencyP95': 'p95 latency',
    'overview.noTraffic': 'No request yet.',
    'overview.recentErrors': 'Recent errors',
    'overview.noErrors': 'No recent error.',
    'overview.seeLogs': 'Open the request log',

    'logs.title': 'Request log',
    'logs.subtitle': 'Served requests, filtered and paginated server-side; filters live in the URL.',
    'logs.provider': 'Provider',
    'logs.status': 'Status',
    'logs.model': 'Model',
    'logs.limit': 'Page size',
    'logs.filter': 'Filter',
    'logs.date': 'Date',
    'logs.id': 'Identifier',
    'logs.latency': 'Latency',
    'logs.attempts': 'Attempts',
    'logs.tokens': 'Input / output tokens',
    'logs.tokensShort': 'Tokens in/out',
    'logs.detail': 'Detail',
    'logs.next': 'Next page',
    'logs.previous': 'Previous page',
    'logs.empty': 'No request for these filters.',

    'providers.title': 'Providers',
    'providers.subtitle': 'Configured providers, keys and base URLs.',
    'providers.empty': 'No provider yet.',
    'providers.create': 'Add a provider',
    'providers.id': 'Identifier',
    'providers.type': 'Type',
    'providers.baseUrl': 'Base URL',
    'providers.secret': 'API key (entered once)',
    'providers.keys': 'Keys',
    'providers.noKeys': 'No key.',
    'providers.addKey': 'Add a key',
    'providers.newKey': 'New key',
    'providers.keyPreview': 'Preview',
    'providers.weight': 'Weight',
    'providers.delete': 'Delete',
    'providers.confirmDelete': 'Delete this provider and its keys?',
    'providers.confirmDeleteKey': 'Remove this key? Requests will stop using it.',
    'providers.secretNotice':
      'The key is never displayed again: only a preview and its weight are kept.',
    'providers.save': 'Save',
    'providers.deleteKey': 'Remove key',
    'providers.created': 'Provider created.',
    'providers.updated': 'Base URL saved.',
    'providers.deleted': 'Provider deleted.',
    'providers.keyAdded': 'Key added: it will never be displayed in clear again.',
    'providers.keyRemoved': 'Key removed.',

    'routes.title': 'Routes and aliases',
    'routes.subtitle': 'Fallback chains per model alias.',
    'routes.empty': 'No route yet.',
    'routes.create': 'Create a route',
    'routes.alias': 'Alias',
    'routes.fallbacks': 'Fallback chain (one “provider/model” per line)',
    'routes.chain': 'Fallback chain',
    'routes.moveUp': 'Move up',
    'routes.moveDown': 'Move down',
    'routes.removeTarget': 'Remove target',
    'routes.newTarget': 'New target (provider/model)',
    'routes.addTarget': 'Add target',
    'routes.unsaved': 'Unsaved changes',
    'routes.discard': 'Discard changes',
    'routes.delete': 'Delete',
    'routes.confirmDelete': 'Delete this route?',
    'routes.save': 'Save chain',
    'routes.created': 'Route created.',
    'routes.saved': 'Fallback chain saved.',
    'routes.deleted': 'Route deleted.',

    'auth.title': 'Admin token',
    'auth.help':
      'The admin API is protected. Paste the token printed when buntu first started (or the value of BUNTU_ADMIN_TOKEN).',
    'auth.token': 'Token',
    'auth.submit': 'Submit'
  }
};

export const locale = writable<Locale>('fr');

export const t = derived(locale, ($locale) => {
  return (key: string, params: Record<string, string | number> = {}): string => {
    let text = dictionary[$locale][key] ?? key;
    for (const [name, value] of Object.entries(params)) {
      text = text.replace(`{${name}}`, String(value));
    }
    return text;
  };
});

/** Langue courante, pour `Intl` (dates, nombres, unités). */
export function currentLocale(): Locale {
  return get(locale);
}

function applyLang(next: Locale): void {
  if (typeof document !== 'undefined') {
    document.documentElement.lang = next;
  }
}

export function setLocale(next: Locale): void {
  locale.set(next);
  applyLang(next);
  try {
    localStorage.setItem('buntu.locale', next);
  } catch {
    // stockage local indisponible : la langue reste en mémoire
  }
}

export function restoreLocale(): void {
  try {
    const stored = localStorage.getItem('buntu.locale');
    if (stored === 'fr' || stored === 'en') {
      locale.set(stored);
      applyLang(stored);
    }
  } catch {
    // stockage local indisponible
  }
}
