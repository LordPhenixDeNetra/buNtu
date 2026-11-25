import createClient from 'openapi-fetch';
import { get, writable } from 'svelte/store';
import type { paths } from './api/schema';
import { t } from './i18n';

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

/**
 * Message d'une erreur d'API ; `fallback` (traduit par l'appelant) sinon.
 *
 * L'API répond en anglais ; la console traduit le `code` stable de l'erreur et garde le détail
 * du serveur, privé de son préfixe (`invalid request: …` pour `invalid_request`).
 */
export function errorMessage(error: unknown, fallback: string): string {
  if (typeof error === 'string' && error !== '') {
    return error;
  }
  if (error && typeof error === 'object' && 'error' in error) {
    const detail = (error as { error?: { message?: string; code?: string } }).error;
    const label = detail?.code ? get(t)(`apiError.${detail.code}`) : '';
    if (!label || label === `apiError.${detail?.code}`) {
      return detail?.message || fallback;
    }
    const prefix = `${detail?.code?.replace(/_/g, ' ')}: `;
    const rest = detail?.message?.startsWith(prefix)
      ? detail.message.slice(prefix.length)
      : (detail?.message ?? '');
    return rest ? `${label} — ${rest}` : label;
  }
  return fallback;
}
