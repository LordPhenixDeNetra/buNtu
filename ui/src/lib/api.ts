import createClient from 'openapi-fetch';
import { get, writable } from 'svelte/store';
import type { paths } from './api/schema';

/**
 * Client d'API généré depuis `openapi.json` (openapi-typescript + openapi-fetch).
 * Aucun type de requête/réponse n'est écrit à la main.
 */
export const api = createClient<paths>({ baseUrl: '' });

const TOKEN_KEY = 'buntu.adminToken';

function readToken(): string {
  try {
    return sessionStorage.getItem(TOKEN_KEY) ?? '';
  } catch {
    return '';
  }
}

/** Jeton de l'API d'administration, gardé pour la durée de l'onglet seulement. */
export const adminToken = writable<string>(readToken());
/** Passe à `true` quand le serveur répond 401 : la console demande alors le jeton. */
export const tokenRequired = writable<boolean>(false);

export function saveAdminToken(token: string): void {
  adminToken.set(token);
  tokenRequired.set(false);
  try {
    sessionStorage.setItem(TOKEN_KEY, token);
  } catch {
    // stockage de session indisponible : le jeton reste en mémoire
  }
}

api.use({
  onRequest({ request }) {
    const token = get(adminToken);
    if (token) {
      request.headers.set('Authorization', `Bearer ${token}`);
    }
    return request;
  },
  onResponse({ response }) {
    if (response.status === 401) {
      tokenRequired.set(true);
    }
    return response;
  }
});

/** Message d'une erreur d'API ; `fallback` (traduit par l'appelant) sinon. */
export function errorMessage(error: unknown, fallback: string): string {
  if (typeof error === 'string' && error !== '') {
    return error;
  }
  if (error && typeof error === 'object' && 'error' in error) {
    const detail = (error as { error?: { message?: string } }).error;
    if (detail?.message) {
      return detail.message;
    }
  }
  return fallback;
}
