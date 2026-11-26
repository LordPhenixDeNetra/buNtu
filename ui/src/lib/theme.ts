import { writable } from 'svelte/store';

export type Theme = 'light' | 'dark';

const STORAGE_KEY = 'buntu.theme';

/** Sombre par défaut (MASTER) ; le choix de l'utilisateur est mémorisé. */
export const theme = writable<Theme>('dark');

function apply(next: Theme): void {
  if (typeof document === 'undefined') {
    return;
  }
  document.documentElement.dataset.theme = next;
}

function remember(next: Theme): void {
  try {
    localStorage.setItem(STORAGE_KEY, next);
  } catch {
    // stockage local indisponible : le thème reste en mémoire
  }
}

export function setTheme(next: Theme): void {
  theme.set(next);
  apply(next);
  remember(next);
}

export function toggleTheme(): void {
  theme.update((current) => {
    const next: Theme = current === 'light' ? 'dark' : 'light';
    apply(next);
    remember(next);
    return next;
  });
}

/** Choix mémorisé, sinon sombre (déjà appliqué par `app.html` avant le premier rendu). */
export function restoreTheme(): void {
  let next: Theme = 'dark';
  try {
    const stored = localStorage.getItem(STORAGE_KEY);
    if (stored === 'light' || stored === 'dark') {
      next = stored;
    }
  } catch {
    // stockage local indisponible
  }
  theme.set(next);
  apply(next);
}
