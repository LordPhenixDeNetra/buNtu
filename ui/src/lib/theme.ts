import { writable } from 'svelte/store';

export type Theme = 'light' | 'dark';

const STORAGE_KEY = 'buntu.theme';

export const theme = writable<Theme>('light');

function apply(next: Theme): void {
  if (typeof document === 'undefined') {
    return;
  }
  document.documentElement.dataset.theme = next;
  document.body.dataset.theme = next;
}

export function setTheme(next: Theme): void {
  theme.set(next);
  apply(next);
  try {
    localStorage.setItem(STORAGE_KEY, next);
  } catch {
    // stockage local indisponible : le thème reste en mémoire
  }
}

export function toggleTheme(): void {
  theme.update((current) => {
    const next: Theme = current === 'light' ? 'dark' : 'light';
    apply(next);
    try {
      localStorage.setItem(STORAGE_KEY, next);
    } catch {
      // stockage local indisponible
    }
    return next;
  });
}

export function restoreTheme(): void {
  let next: Theme = 'light';
  try {
    const stored = localStorage.getItem(STORAGE_KEY);
    if (stored === 'light' || stored === 'dark') {
      next = stored;
    } else if (window.matchMedia('(prefers-color-scheme: dark)').matches) {
      next = 'dark';
    }
  } catch {
    // stockage local indisponible
  }
  theme.set(next);
  apply(next);
}
