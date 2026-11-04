// See https://svelte.dev/docs/kit/types#app
declare global {
  namespace App {}

  interface Window {
    __BUNTU_LOCALE__?: 'fr' | 'en';
  }
}

export {};
