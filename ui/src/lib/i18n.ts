import { derived, writable } from 'svelte/store';

export type Locale = 'fr' | 'en';

const dictionary: Record<Locale, Record<string, string>> = {
  fr: {
    'app.title': 'buNtu — console',
    'nav.providers': 'Fournisseurs',
    'nav.routes': 'Routes',
    'nav.logs': 'Journal',
    'logs.title': 'Journal des requêtes',
    'logs.subtitle': 'Requêtes servies, filtrables et paginées côté serveur ; les filtres sont dans l’URL.',
    'logs.provider': 'Fournisseur',
    'logs.status': 'Statut',
    'logs.model': 'Modèle',
    'logs.limit': 'Taille de page',
    'logs.filter': 'Filtrer',
    'logs.date': 'Requête',
    'logs.latency': 'Latence',
    'logs.attempts': 'Tentatives',
    'logs.tokens': 'Jetons entrée / sortie',
    'logs.detail': 'Détail',
    'logs.next': 'Page suivante',
    'logs.empty': 'Aucune requête pour ces filtres.',    'theme.light': 'Thème clair',
    'theme.dark': 'Thème sombre',
    'lang.label': 'Langue',
    'loading': 'Chargement…',
    'error.title': 'Erreur',
    'retry': 'Réessayer',
    'providers.title': 'Fournisseurs',
    'providers.subtitle': 'Fournisseurs configurés, clés et URL de base.',
    'providers.empty': 'Aucun fournisseur pour le moment.',
    'providers.create': 'Ajouter un fournisseur',
    'providers.id': 'Identifiant',
    'providers.type': 'Type',
    'providers.baseUrl': 'URL de base',
    'providers.secret': 'Clé API (saisie unique)',
    'providers.addKey': 'Ajouter une clé',
    'providers.keyPreview': 'Aperçu',
    'providers.weight': 'Poids',
    'providers.delete': 'Supprimer',
    'providers.confirmDelete': 'Supprimer ce fournisseur et ses clés ?',
    'providers.secretNotice': 'La clé n’est jamais réaffichée : seuls un aperçu et son poids sont conservés.',
    'providers.save': 'Enregistrer',
    'providers.edit': 'Modifier',
    'providers.deleteKey': 'Retirer la clé',
    'routes.title': 'Routes et alias',
    'routes.subtitle': 'Chaînes de repli par alias de modèle.',
    'routes.empty': 'Aucune route pour le moment.',
    'routes.create': 'Créer une route',
    'routes.alias': 'Alias',
    'routes.fallbacks': 'Chaîne de repli (un « fournisseur/modèle » par ligne)',
    'routes.moveUp': 'Monter',
    'routes.moveDown': 'Descendre',
    'routes.delete': 'Supprimer',
    'routes.confirmDelete': 'Supprimer cette route ?',
    'routes.save': 'Enregistrer la chaîne',
    'common.cancel': 'Annuler',
    'common.close': 'Fermer'
  },
  en: {
    'app.title': 'buNtu — console',
    'nav.providers': 'Providers',
    'nav.routes': 'Routes',
    'nav.logs': 'Request log',
    'logs.title': 'Request log',
    'logs.subtitle': 'Served requests, filtered and paginated server-side; filters live in the URL.',
    'logs.provider': 'Provider',
    'logs.status': 'Status',
    'logs.model': 'Model',
    'logs.limit': 'Page size',
    'logs.filter': 'Filter',
    'logs.date': 'Request',
    'logs.latency': 'Latency',
    'logs.attempts': 'Attempts',
    'logs.tokens': 'Input / output tokens',
    'logs.detail': 'Detail',
    'logs.next': 'Next page',
    'logs.empty': 'No request for these filters.',    'theme.light': 'Light theme',
    'theme.dark': 'Dark theme',
    'lang.label': 'Language',
    'loading': 'Loading…',
    'error.title': 'Error',
    'retry': 'Retry',
    'providers.title': 'Providers',
    'providers.subtitle': 'Configured providers, keys and base URLs.',
    'providers.empty': 'No provider yet.',
    'providers.create': 'Add a provider',
    'providers.id': 'Identifier',
    'providers.type': 'Type',
    'providers.baseUrl': 'Base URL',
    'providers.secret': 'API key (entered once)',
    'providers.addKey': 'Add a key',
    'providers.keyPreview': 'Preview',
    'providers.weight': 'Weight',
    'providers.delete': 'Delete',
    'providers.confirmDelete': 'Delete this provider and its keys?',
    'providers.secretNotice': 'The key is never displayed again: only a preview and its weight are kept.',
    'providers.save': 'Save',
    'providers.edit': 'Edit',
    'providers.deleteKey': 'Remove key',
    'routes.title': 'Routes and aliases',
    'routes.subtitle': 'Fallback chains per model alias.',
    'routes.empty': 'No route yet.',
    'routes.create': 'Create a route',
    'routes.alias': 'Alias',
    'routes.fallbacks': 'Fallback chain (one “provider/model” per line)',
    'routes.moveUp': 'Move up',
    'routes.moveDown': 'Move down',
    'routes.delete': 'Delete',
    'routes.confirmDelete': 'Delete this route?',
    'routes.save': 'Save chain',
    'common.cancel': 'Cancel',
    'common.close': 'Close'
  }
};

export const locale = writable<Locale>('fr');

export const t = derived(locale, ($locale) => {
  return (key: string): string => dictionary[$locale][key] ?? key;
});

export function setLocale(next: Locale): void {
  locale.set(next);
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
    }
  } catch {
    // stockage local indisponible
  }
}
