import { writable } from 'svelte/store';

/** Demande d'ouverture de la palette de commandes (bouton de la barre de navigation). */
export const paletteRequested = writable(false);
