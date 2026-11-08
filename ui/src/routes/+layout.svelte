<script lang="ts">
  import { onMount, type Snippet } from 'svelte';
  import KeyRound from '@lucide/svelte/icons/key-round';
  import '../app.css';
  import { restoreLocale, t } from '$lib/i18n';
  import { restoreTheme } from '$lib/theme';
  import { saveAdminToken, tokenRequired } from '$lib/api';
  import Nav from '$lib/components/Nav.svelte';
  import Toasts from '$lib/components/Toasts.svelte';
  import ConfirmDialog from '$lib/components/ConfirmDialog.svelte';

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
        <KeyRound size={28} />
        <h1 id="titre-jeton">{$t('auth.title')}</h1>
        <p class="muted">{$t('auth.help')}</p>
        <div class="field">
          <label for="jeton-admin">{$t('auth.token')}</label>
          <input id="jeton-admin" type="password" autocomplete="off" bind:value={token} required />
        </div>
        <button type="submit">{$t('auth.submit')}</button>
      </form>
    {:else}
      {@render children()}
    {/if}
  </main>
</div>
<Toasts />
<ConfirmDialog />

<style>
  .shell {
    min-height: 100vh;
  }

  main {
    width: 100%;
    max-width: var(--content-width);
    margin: 0 auto;
    padding: var(--space-5) var(--space-4) var(--space-7);
  }

  main:focus {
    outline: none;
  }

  @media (min-width: 64rem) {
    .shell {
      display: grid;
      grid-template-columns: 15rem minmax(0, 1fr);
    }

    main {
      padding: var(--space-6) var(--space-6) var(--space-7);
    }
  }

  .token {
    display: grid;
    gap: var(--space-3);
    max-width: 30rem;
    margin: var(--space-7) auto 0;
  }

  .token :global(svg) {
    color: var(--color-accent);
  }

  .token h1 {
    margin: 0;
  }

  .token p {
    margin: 0;
  }
</style>
