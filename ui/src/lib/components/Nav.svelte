<script lang="ts">
  import { onMount } from 'svelte';
  import Sun from '@lucide/svelte/icons/sun';
  import Moon from '@lucide/svelte/icons/moon';
  import Search from '@lucide/svelte/icons/search';
  import { page } from '$app/state';
  import { locale, setLocale, t } from '$lib/i18n';
  import { theme, toggleTheme } from '$lib/theme';
  import { sections } from '$lib/sections';
  import { paletteRequested } from '$lib/palette';

  /** État de la passerelle (`/ready`), rafraîchi toutes les 30 s ; `null` tant qu'inconnu. */
  let ready = $state<boolean | null>(null);
  const isMac = typeof navigator !== 'undefined' && /Mac|iPhone|iPad/.test(navigator.platform);

  function isCurrent(href: string): boolean {
    return href === '/' ? page.url.pathname === '/' : page.url.pathname.startsWith(href);
  }

  async function probe(): Promise<void> {
    ready = await fetch('/ready')
      .then((response) => response.ok)
      .catch(() => false);
  }

  onMount(() => {
    void probe();
    const timer = setInterval(probe, 30_000);
    return () => clearInterval(timer);
  });
</script>

<header class="sidebar">
  <div class="top">
    <a class="brand" href="/">
      <!-- Marque isolée ici (ADR-0001) : un changement de nom ne touche que ce bloc. -->
      <svg class="brand-mark" viewBox="0 0 32 32" aria-hidden="true">
        <defs>
          <linearGradient id="brand-gradient" x1="0" y1="0" x2="1" y2="1">
            <stop offset="0" stop-color="#8b6bff" />
            <stop offset="1" stop-color="#5a37f0" />
          </linearGradient>
        </defs>
        <rect width="32" height="32" rx="8" fill="url(#brand-gradient)" />
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

    <button
      class="search"
      type="button"
      aria-keyshortcuts="Control+K Meta+K"
      onclick={() => paletteRequested.set(true)}
    >
      <Search size={16} />
      <span class="search-label">{$t('nav.search')}</span>
      <span class="search-keys" aria-hidden="true"><kbd>{isMac ? '⌘' : 'Ctrl'}</kbd><kbd>K</kbd></span>
    </button>
  </div>

  <nav aria-label={$t('nav.label')}>
    <p class="nav-heading eyebrow" aria-hidden="true">{$t('nav.console')}</p>
    <ul>
      {#each sections as section (section.href)}
        {@const Icon = section.icon}
        <li>
          <a
            href={section.href}
            class="nav-link"
            aria-current={isCurrent(section.href) ? 'page' : undefined}
          >
            <Icon size={16} />
            <span>{$t(section.key)}</span>
          </a>
        </li>
      {/each}
    </ul>
  </nav>

  <div class="footer">
    <p class="gateway" aria-live="polite">
      <span class="status-dot" class:ok={ready === true} class:ko={ready === false}></span>
      <span class="gateway-label">
        {ready === false ? $t('nav.gatewayDown') : $t('nav.gatewayReady')}
      </span>
    </p>
    <div class="settings">
      <label for="langue" class="sr-only">{$t('lang.label')}</label>
      <select
        id="langue"
        value={$locale}
        onchange={(event) => setLocale(event.currentTarget.value as 'fr' | 'en')}
      >
        <!-- Nom de chaque langue dans cette langue, avec `lang` : bien prononcé par les lecteurs d'écran. -->
        <option value="fr" lang="fr">{$t('lang.fr')}</option>
        <option value="en" lang="en">{$t('lang.en')}</option>
      </select>
      <button
        class="btn-ghost btn-icon"
        type="button"
        onclick={toggleTheme}
        aria-pressed={$theme === 'dark'}
        aria-label={$theme === 'dark' ? $t('theme.light') : $t('theme.dark')}
      >
        {#if $theme === 'dark'}
          <Sun size={16} />
        {:else}
          <Moon size={16} />
        {/if}
      </button>
    </div>
  </div>
</header>

<style>
  /* Mobile d'abord : barre du haut compacte ; la barre latérale arrive à 64 rem. */
  .sidebar {
    position: sticky;
    top: 0;
    z-index: 10;
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-2) var(--space-3);
    padding: var(--space-2) var(--space-3);
    border-bottom: 1px solid var(--color-border);
    background: color-mix(in srgb, var(--color-bg) 82%, transparent);
    backdrop-filter: blur(14px) saturate(1.4);
  }

  .top {
    display: flex;
    flex: 1;
    align-items: center;
    gap: var(--space-2);
    min-width: 0;
  }

  .brand {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    min-height: var(--target-min);
    color: var(--color-text);
    font-size: var(--text-base);
    font-weight: 600;
    letter-spacing: -0.02em;
  }

  .brand:hover {
    text-decoration: none;
  }

  .brand-mark {
    width: 26px;
    height: 26px;
    flex: none;
    filter: drop-shadow(0 4px 12px rgb(109 74 255 / 0.35));
  }

  /* Faux champ de recherche : ouvre la palette de commandes. */
  .search {
    width: var(--control-height);
    margin-left: auto;
    padding: 0;
    justify-content: center;
    border-color: var(--color-border-strong);
    background: var(--color-surface);
    color: var(--color-muted);
    box-shadow: none;
  }

  .search:hover:not(:disabled) {
    background: var(--color-surface-2);
    color: var(--color-text);
  }

  /* Mobile : icône seule ; le libellé reste le nom accessible (WCAG 2.5.3). */
  .search-label {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip: rect(0 0 0 0);
    white-space: nowrap;
  }

  .search-keys {
    display: none;
  }

  .search-keys {
    gap: 2px;
    margin-left: auto;
  }

  nav {
    order: 3;
    width: 100%;
    overflow-x: auto;
  }

  .nav-heading {
    display: none;
  }

  ul {
    display: flex;
    gap: var(--space-1);
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .nav-link {
    position: relative;
    display: flex;
    align-items: center;
    gap: var(--space-2);
    min-height: var(--target-min);
    padding: 0 var(--space-3);
    border-radius: var(--radius-control);
    color: var(--color-muted);
    font-weight: 500;
    white-space: nowrap;
    transition:
      background-color var(--motion-fast) var(--ease-out),
      color var(--motion-fast) var(--ease-out);
  }

  .nav-link:hover {
    background: var(--color-surface-2);
    color: var(--color-text);
    text-decoration: none;
  }

  .nav-link[aria-current='page'] {
    background: var(--color-surface-3);
    color: var(--color-text);
  }

  .nav-link[aria-current='page'] :global(svg) {
    color: var(--color-accent-text);
  }

  .footer {
    display: contents;
  }

  .gateway {
    display: none;
  }

  .settings {
    display: flex;
    align-items: center;
    gap: var(--space-1);
  }

  .settings select {
    width: auto;
    border-color: var(--color-border-strong);
  }

  /* Ordinateur : barre latérale fixe, navigation verticale, état et réglages en pied. */
  @media (min-width: 64rem) {
    .sidebar {
      position: sticky;
      height: 100vh;
      flex-direction: column;
      flex-wrap: nowrap;
      align-items: stretch;
      gap: var(--space-5);
      padding: var(--space-4) var(--space-3);
      border-right: 1px solid var(--color-border);
      border-bottom: 0;
      background: var(--color-bg);
      backdrop-filter: none;
    }

    .top {
      flex: none;
      flex-direction: column;
      align-items: stretch;
      gap: var(--space-4);
    }

    .brand {
      padding: 0 var(--space-2);
    }

    .search {
      width: 100%;
      margin: 0;
      padding: 0 var(--space-2) 0 var(--space-3);
      justify-content: flex-start;
    }

    .search-label {
      position: static;
      width: auto;
      height: auto;
      overflow: visible;
      clip: auto;
    }

    .search-keys {
      display: inline-flex;
    }

    nav {
      order: 0;
      flex: 1;
      overflow: visible;
    }

    .nav-heading {
      display: block;
      margin: 0 0 var(--space-2);
      padding: 0 var(--space-3);
    }

    ul {
      flex-direction: column;
      gap: 2px;
    }

    .nav-link[aria-current='page']::before {
      content: '';
      position: absolute;
      left: calc(-1 * var(--space-3));
      top: 25%;
      bottom: 25%;
      width: 2px;
      border-radius: 0 2px 2px 0;
      background: var(--color-accent-text);
    }

    .footer {
      display: grid;
      gap: var(--space-3);
      padding-top: var(--space-3);
      border-top: 1px solid var(--color-border);
    }

    .gateway {
      display: flex;
      align-items: center;
      gap: var(--space-2);
      padding: 0 var(--space-2);
      color: var(--color-muted);
      font-size: var(--text-xs);
    }

    .settings {
      justify-content: space-between;
    }

    .settings select {
      flex: 1;
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
