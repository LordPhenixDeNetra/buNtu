<script lang="ts">
  import LayoutDashboard from '@lucide/svelte/icons/layout-dashboard';
  import Server from '@lucide/svelte/icons/server';
  import Route from '@lucide/svelte/icons/route';
  import ScrollText from '@lucide/svelte/icons/scroll-text';
  import Sun from '@lucide/svelte/icons/sun';
  import Moon from '@lucide/svelte/icons/moon';
  import { page } from '$app/state';
  import { locale, setLocale, t } from '$lib/i18n';
  import { theme, toggleTheme } from '$lib/theme';

  const sections = [
    { href: '/', key: 'nav.overview', icon: LayoutDashboard },
    { href: '/providers', key: 'nav.providers', icon: Server },
    { href: '/routes', key: 'nav.routes', icon: Route },
    { href: '/logs', key: 'nav.logs', icon: ScrollText }
  ];

  function isCurrent(href: string): boolean {
    return href === '/' ? page.url.pathname === '/' : page.url.pathname.startsWith(href);
  }
</script>

<header class="sidebar">
  <a class="brand" href="/">
    <!-- Marque isolée ici (ADR-0001) : un changement de nom ne touche que ce bloc. -->
    <svg class="brand-mark" viewBox="0 0 32 32" aria-hidden="true">
      <defs>
        <linearGradient id="brand-gradient" x1="0" y1="0" x2="1" y2="1">
          <stop offset="0" stop-color="var(--color-brand)" />
          <stop offset="1" stop-color="var(--color-accent)" />
        </linearGradient>
      </defs>
      <rect width="32" height="32" rx="9" fill="url(#brand-gradient)" />
      <path
        d="M9 11h6a3 3 0 0 1 3 3v4a3 3 0 0 0 3 3h2M9 21h4M19 11h4"
        fill="none"
        stroke="#fff"
        stroke-width="2.2"
        stroke-linecap="round"
      />
    </svg>
    <span class="brand-name">buNtu</span>
  </a>

  <nav aria-label={$t('nav.label')}>
    <ul>
      {#each sections as section (section.href)}
        {@const Icon = section.icon}
        <li>
          <a
            href={section.href}
            class="nav-link"
            aria-current={isCurrent(section.href) ? 'page' : undefined}
          >
            <Icon size={18} />
            <span>{$t(section.key)}</span>
          </a>
        </li>
      {/each}
    </ul>
  </nav>

  <div class="settings">
    <label for="langue" class="sr-only">{$t('lang.label')}</label>
    <select
      id="langue"
      value={$locale}
      onchange={(event) => setLocale(event.currentTarget.value as 'fr' | 'en')}
    >
      <option value="fr">FR</option>
      <option value="en">EN</option>
    </select>
    <button
      class="btn-secondary btn-icon"
      type="button"
      onclick={toggleTheme}
      aria-pressed={$theme === 'dark'}
      aria-label={$theme === 'dark' ? $t('theme.light') : $t('theme.dark')}
    >
      {#if $theme === 'dark'}
        <Sun size={18} />
      {:else}
        <Moon size={18} />
      {/if}
    </button>
  </div>
</header>

<style>
  .sidebar {
    position: sticky;
    top: 0;
    z-index: 5;
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-2) var(--space-4);
    border-bottom: 1px solid var(--color-border);
    background: color-mix(in srgb, var(--color-surface) 88%, transparent);
    backdrop-filter: blur(12px);
  }

  .brand {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    color: var(--color-text);
    font-weight: 700;
    letter-spacing: -0.02em;
    text-decoration: none;
  }

  .brand-mark {
    width: 28px;
    height: 28px;
  }

  nav {
    flex: 1;
    overflow-x: auto;
  }

  ul {
    display: flex;
    gap: var(--space-1);
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .nav-link {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    min-height: var(--target-min);
    padding: 0 var(--space-3);
    border-radius: var(--radius-control);
    color: var(--color-muted);
    font-size: var(--text-sm);
    font-weight: 500;
    text-decoration: none;
    white-space: nowrap;
    transition:
      background-color var(--motion-fast) var(--ease-out),
      color var(--motion-fast) var(--ease-out);
  }

  .nav-link:hover {
    background: var(--color-surface-2);
    color: var(--color-text);
  }

  .nav-link[aria-current='page'] {
    background: color-mix(in srgb, var(--color-accent) var(--tint-strength), transparent);
    color: var(--color-text);
  }

  .nav-link[aria-current='page'] :global(svg) {
    color: var(--color-accent);
  }

  .settings {
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }

  .settings select {
    width: auto;
  }

  /* Ordinateur : barre latérale fixe, navigation verticale, réglages en pied. */
  @media (min-width: 64rem) {
    .sidebar {
      position: sticky;
      height: 100vh;
      flex-direction: column;
      align-items: stretch;
      gap: var(--space-5);
      padding: var(--space-5) var(--space-3);
      border-right: 1px solid var(--color-border);
      border-bottom: 0;
    }

    .brand {
      padding: 0 var(--space-2);
      font-size: var(--text-lg);
    }

    nav {
      overflow: visible;
    }

    ul {
      flex-direction: column;
    }

    .settings {
      justify-content: space-between;
      padding: 0 var(--space-2);
    }
  }

  @media (max-width: 30rem) {
    .brand-name,
    .nav-link span {
      position: absolute;
      width: 1px;
      height: 1px;
      overflow: hidden;
      clip: rect(0 0 0 0);
      white-space: nowrap;
    }
  }
</style>
