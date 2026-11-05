<script lang="ts">
  import { onMount } from 'svelte';
  import '../app.css';
  import { restoreLocale, t } from '$lib/i18n';
  import { restoreTheme } from '$lib/theme';
  import { saveAdminToken, tokenRequired } from '$lib/api';
  import Nav from '$lib/components/Nav.svelte';

  let token = '';

  onMount(() => {
    restoreLocale();
    restoreTheme();
  });

  function submitToken(): void {
    if (token.trim()) {
      saveAdminToken(token.trim());
      // Recharge l'écran courant pour rejouer ses appels avec le jeton.
      location.reload();
    }
  }
</script>

<a class="skip-link" href="#contenu">Aller au contenu</a>
<Nav />
<main id="contenu" style="max-width: 72rem; margin: 0 auto; padding: 1.5rem;">
  {#if $tokenRequired}
    <form on:submit|preventDefault={submitToken} aria-labelledby="titre-jeton">
      <h1 id="titre-jeton">{$t('auth.title')}</h1>
      <p>{$t('auth.help')}</p>
      <label for="jeton-admin">{$t('auth.token')}</label>
      <input id="jeton-admin" type="password" autocomplete="off" bind:value={token} required />
      <button type="submit">{$t('auth.submit')}</button>
    </form>
  {:else}
    <slot />
  {/if}
</main>
