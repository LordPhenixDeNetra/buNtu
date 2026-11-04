<script lang="ts">
  import { onMount } from 'svelte';
  import '../app.css';
  import { restoreLocale } from '$lib/i18n';
  import { restoreTheme } from '$lib/theme';
  import Nav from '$lib/components/Nav.svelte';

  onMount(() => {
    restoreLocale();
    restoreTheme();
  });
</script>

<a class="skip-link" href="#contenu">Aller au contenu</a>
<Nav />
<main id="contenu" style="max-width: 72rem; margin: 0 auto; padding: 1.5rem;">
  <slot />
</main>
