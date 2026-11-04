<script lang="ts">
  import { page } from '$app/state';
  import { locale, setLocale, t } from '$lib/i18n';
  import { theme, toggleTheme } from '$lib/theme';

  const sections = [
    { href: '/providers', key: 'nav.providers' },
    { href: '/routes', key: 'nav.routes' },
    { href: '/logs', key: 'nav.logs' }
  ];
</script>

<header
  style="border-bottom: 1px solid var(--border); background: var(--panel); position: sticky; top: 0; z-index: 5;"
>
  <div
    style="max-width: 72rem; margin: 0 auto; padding: 0.75rem 1.5rem; display: flex; align-items: center; gap: 1.25rem; flex-wrap: wrap;"
  >
    <a href="/" style="font-weight: 700; text-decoration: none; color: var(--text);">buNtu</a>
    <nav aria-label="Navigation principale" style="display: flex; gap: 1rem; flex: 1;">
      {#each sections as section (section.href)}
        <a
          href={section.href}
          style="text-decoration: none; padding: 0.2rem 0; border-bottom: 2px solid {page.url
            .pathname === section.href
            ? 'var(--accent)'
            : 'transparent'};"
        >
          {$t(section.key)}
        </a>
      {/each}
    </nav>
    <div style="display: flex; gap: 0.6rem; align-items: center;">
      <label for="langue" class="sr-only" style="position: absolute; left: -999px;">
        {$t('lang.label')}
      </label>
      <select
        id="langue"
        value={$locale}
        on:change={(event) => setLocale((event.currentTarget as HTMLSelectElement).value as 'fr' | 'en')}
        style="width: auto;"
      >
        <option value="fr">FR</option>
        <option value="en">EN</option>
      </select>
      <button class="secondary" type="button" on:click={toggleTheme} aria-pressed={$theme === 'dark'}>
        {$theme === 'dark' ? '☀︎' : '☾'}
        <span class="sr-only" style="position: absolute; left: -999px;">
          {$theme === 'dark' ? $t('theme.light') : $t('theme.dark')}
        </span>
      </button>
    </div>
  </div>
</header>
