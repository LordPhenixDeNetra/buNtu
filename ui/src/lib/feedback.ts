import { writable } from 'svelte/store';

/** Retours de l'interface : notifications éphémères et confirmations (MASTER, règles 3 et 7). */

export type ToastKind = 'success' | 'error';

export interface Toast {
  id: number;
  kind: ToastKind;
  message: string;
}

export const toasts = writable<Toast[]>([]);

let nextId = 1;

/** Durée d'affichage d'une notification de succès ; une erreur reste jusqu'à fermeture. */
const SUCCESS_MS = 4000;

export function dismissToast(id: number): void {
  toasts.update((list) => list.filter((toast) => toast.id !== id));
}

export function notify(kind: ToastKind, message: string): void {
  const id = nextId++;
  toasts.update((list) => [...list, { id, kind, message }]);
  if (kind === 'success') {
    setTimeout(() => dismissToast(id), SUCCESS_MS);
  }
}

export interface ConfirmRequest {
  message: string;
  confirmLabel: string;
  resolve: (confirmed: boolean) => void;
}

export const confirmRequest = writable<ConfirmRequest | null>(null);

/** Ouvre le dialogue de confirmation (remplace `window.confirm`) et attend la réponse. */
export function confirmAction(message: string, confirmLabel: string): Promise<boolean> {
  return new Promise((resolve) => {
    confirmRequest.set({ message, confirmLabel, resolve });
  });
}
