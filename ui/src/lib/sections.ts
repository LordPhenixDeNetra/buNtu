import LayoutDashboard from '@lucide/svelte/icons/layout-dashboard';
import Server from '@lucide/svelte/icons/server';
import Route from '@lucide/svelte/icons/route';
import ScrollText from '@lucide/svelte/icons/scroll-text';

/** Écrans de la console : barre de navigation et palette de commandes. */
export const sections = [
  { href: '/', key: 'nav.overview', icon: LayoutDashboard },
  { href: '/providers', key: 'nav.providers', icon: Server },
  { href: '/routes', key: 'nav.routes', icon: Route },
  { href: '/logs', key: 'nav.logs', icon: ScrollText }
];
