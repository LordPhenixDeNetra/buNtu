<script lang="ts">
  import { onMount, type Snippet } from 'svelte';
  import KeyRound from '@lucide/svelte/icons/key-round';
  import ArrowRight from '@lucide/svelte/icons/arrow-right';
  import '../app.css';
  import { restoreLocale, t } from '$lib/i18n';
  import { restoreTheme } from '$lib/theme';
  import { saveAdminToken, tokenRequired } from '$lib/api';
  import Nav from '$lib/components/Nav.svelte';
  import Toasts from '$lib/components/Toasts.svelte';
  import ConfirmDialog from '$lib/components/ConfirmDialog.svelte';
  import CommandPalette from '$lib/components/CommandPalette.svelte';

  let { children }: { children: Snippet } = $props();
  let token = $state('');

  onMount(() => {
    restoreLocale();
    restoreTheme();
  });

  function submitToken(event: SubmitEvent): void {
    event.preventDefault();
    if (token.trim()) {
      saveAdminToken(token.trim());
      // Recharge l'écran courant pour rejouer ses appels avec le jeton.
      location.reload();
    }
  }
</script>

<svelte:head>
  <title>{$t('app.title')}</title>
</svelte:head>

<a class="skip-link" href="#contenu">{$t('a11y.skip')}</a>
<div class="shell">
  <Nav />
  <main id="contenu" tabindex="-1">
    {#if $tokenRequired}
      <form class="card token" onsubmit={submitToken} aria-labelledby="titre-jeton">
        <span class="icon-tile token-icon"><KeyRound size={20} /></span>
        <h1 id="titre-jeton">{$t('auth.title')}</h1>
        <p class="muted">{$t('auth.help')}</p>
        <div class="field">
          <label for="jeton-admin">{$t('auth.token')}</label>
          <input
            id="jeton-admin"
            class="mono"
            type="password"
            autocomplete="off"
            bind:value={token}
            required
          />
        </div>
        <button type="submit">{$t('auth.submit')}<ArrowRight size={16} /></button>
      </form>
    {:else}
      {@render children()}
    {/if}
  </main>
</div>
<Toasts />
<ConfirmDialog />
<CommandPalette />

<style>
  .shell {
    min-height: 100vh;
  }

  main {
    position: relative;
    isolation: isolate;
    width: 100%;
    max-width: var(--content-width);
    margin: 0 auto;
    padding: var(--space-5) var(--space-4) var(--space-7);
  }

  /* Halo de marque très discret en haut de page : profondeur sans gêner la lecture. */
  main::before {
    content: '';
    position: absolute;
    inset: 0 0 auto;
    height: 28rem;
    z-index: -1;
    background: var(--glow);
    pointer-events: none;
  }

  main:focus {
    outline: none;
  }

  @media (min-width: 64rem) {
    .shell {
      display: grid;
      grid-template-columns: var(--sidebar-width) minmax(0, 1fr);
    }

    main {
      padding: var(--space-7) var(--space-6) var(--space-7);
    }
  }

  .token {
    display: grid;
    gap: var(--space-4);
    max-width: 26rem;
    margin: 10vh auto 0;
    padding: var(--space-6);
    box-shadow: var(--shadow-raised);
  }

  .token-icon {
    width: 2.75rem;
    height: 2.75rem;
    color: var(--color-accent-text);
  }

  .token button {
    width: 100%;
  }
</style>
