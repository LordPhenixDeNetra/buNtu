import createClient from 'openapi-fetch';
import type { paths } from './api/schema';

/**
 * Client d'API généré depuis `openapi.json` (openapi-typescript + openapi-fetch).
 * Aucun type de requête/réponse n'est écrit à la main.
 */
export const api = createClient<paths>({ baseUrl: '' });

export function errorMessage(error: unknown): string {
  if (typeof error === 'string') {
    return error;
  }
  if (error && typeof error === 'object' && 'error' in error) {
    const detail = (error as { error?: { message?: string } }).error;
    if (detail?.message) {
      return detail.message;
    }
  }
  return 'Erreur inattendue';
}
