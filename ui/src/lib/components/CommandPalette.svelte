<script lang="ts">
  import Search from '@lucide/svelte/icons/search';
  import SunMoon from '@lucide/svelte/icons/sun-moon';
  import Languages from '@lucide/svelte/icons/languages';
  import { goto } from '$app/navigation';
  import { locale, setLocale, t } from '$lib/i18n';
  import { toggleTheme } from '$lib/theme';
  import { sections } from '$lib/sections';
  import { paletteRequested } from '$lib/palette';

  /**
   * Palette de commandes (`Ctrl/⌘ + K`, revue [7.3]) : aller à un écran, changer de thème ou de
   * langue. `<dialog>` natif pour le piège du focus et Échap ; motif combobox + listbox ARIA.
   */
  let dialog: HTMLDialogElement | undefined = $state();
  let query = $state('');
  let active = $state(0);
  let opener: Element | null = null;

  const commands = $derived([
    ...sections.map((section) => ({
      id: section.href,
      label: $t(section.key),
      icon: section.icon,
      run: () => goto(section.href)
    })),
    { id: 'theme', label: $t('palette.theme'), icon: SunMoon, run: toggleTheme },
    {
      id: 'language',
      label: $t('palette.language'),
      icon: Languages,
      run: () => setLocale($locale === 'fr' ? 'en' : 'fr')
    }
  ]);

  /** Minuscules sans accents : « theme » trouve « thème ». */
  function fold(text: string): string {
    return text.normalize('NFD').replace(/\p{Diacritic}/gu, '').toLocaleLowerCase();
  }

  const matches = $derived(
    commands.filter((command) => fold(command.label).includes(fold(query.trim())))
  );

  function open(): void {
    if (!dialog || dialog.open) {
      return;
    }
    opener = document.activeElement;
    query = '';
    active = 0;
    dialog.showModal();
  }

  $effect(() => {
    if ($paletteRequested) {
      paletteRequested.set(false);
      open();
    }
  });

  function close(): void {
    dialog?.close();
    if (opener instanceof HTMLElement) {
      opener.focus();
    }
  }

  function run(index: number): void {
    const command = matches[index];
    if (command) {
      close();
      command.run();
    }
  }

  function onGlobalKey(event: KeyboardEvent): void {
    if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === 'k') {
      event.preventDefault();
      open();
    }
  }

  function onInputKey(event: KeyboardEvent): void {
    if (event.key === 'ArrowDown') {
      event.preventDefault();
      active = matches.length === 0 ? 0 : (active + 1) % matches.length;
    } else if (event.key === 'ArrowUp') {
      event.preventDefault();
      active = matches.length === 0 ? 0 : (active - 1 + matches.length) % matches.length;
    } else if (event.key === 'Enter') {
      event.preventDefault();
      run(active);
    }
  }
</script>

<svelte:window onkeydown={onGlobalKey} />

<dialog
  bind:this={dialog}
  class="palette"
  aria-label={$t('palette.title')}
  onclick={(event) => {
    // Clic sur le fond (hors du contenu) : fermeture.
    if (event.target === dialog) close();
  }}
>
  <div class="search">
    <Search size={18} />
    <input
      type="text"
      role="combobox"
      aria-expanded="true"
      aria-controls="palette-list"
      aria-activedescendant={matches[active] ? `palette-${matches[active].id}` : undefined}
      aria-label={$t('palette.placeholder')}
      placeholder={$t('palette.placeholder')}
      autocomplete="off"
      bind:value={query}
      oninput={() => (active = 0)}
      onkeydown={onInputKey}
    />
    <kbd>Esc</kbd>
  </div>
  <ul id="palette-list" role="listbox" aria-label={$t('palette.title')}>
    {#each matches as command, index (command.id)}
      {@const Icon = command.icon}
      <!-- Le clavier passe par le champ (combobox, `aria-activedescendant`) : les options ne
           prennent jamais le focus, le clic n'est qu'un raccourci pour la souris. -->
      <!-- svelte-ignore a11y_click_events_have_key_events -->
      <li
        id={`palette-${command.id}`}
        role="option"
        aria-selected={index === active}
        class:active={index === active}
        onclick={() => run(index)}
        onmousemove={() => (active = index)}
      >
        <Icon size={16} />
        {command.label}
      </li>
    {:else}
      <li class="empty muted" role="option" aria-selected="false" aria-disabled="true">
        {$t('palette.empty')}
      </li>
    {/each}
  </ul>
</dialog>

<style>
  .palette {
    width: min(32rem, calc(100vw - 2 * var(--space-4)));
    margin: 12vh auto auto;
    padding: 0;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-tile);
    background: var(--color-surface);
    color: var(--color-text);
    box-shadow: var(--shadow-raised);
  }

  .palette::backdrop {
    background: rgb(0 0 0 / 0.35);
  }

  .search {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-3) var(--space-4);
    border-bottom: 1px solid var(--color-border);
    color: var(--color-muted);
  }

  .search input {
    flex: 1;
    min-width: 0;
    border: none;
    background: transparent;
    box-shadow: none;
  }

  .search input:focus-visible {
    outline: none;
  }

  kbd {
    padding: 0 var(--space-1);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-control);
    font-size: var(--text-xs);
  }

  ul {
    max-height: 50vh;
    margin: 0;
    padding: var(--space-2);
    overflow-y: auto;
    list-style: none;
  }

  li {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-2) var(--space-3);
    border-radius: var(--radius-control);
    cursor: pointer;
  }

  li.active {
    background: color-mix(in srgb, var(--color-accent) var(--tint-strength), transparent);
    color: var(--color-text);
  }

  li.empty {
    cursor: default;
  }
</style>
