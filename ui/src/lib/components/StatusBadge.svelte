<script lang="ts">
  import CircleCheck from '@lucide/svelte/icons/circle-check';
  import CircleAlert from '@lucide/svelte/icons/circle-alert';
  import TriangleAlert from '@lucide/svelte/icons/triangle-alert';

  // Statut HTTP : couleur + icône + code, jamais la couleur seule (MASTER, règle 2).
  let { status }: { status: number } = $props();

  const kind = $derived(
    status >= 500 ? 'danger' : status === 429 ? 'warning' : status >= 400 ? 'danger' : 'success'
  );
</script>

<span class="badge badge-{kind}">
  {#if kind === 'success'}
    <CircleCheck size={14} />
  {:else if kind === 'warning'}
    <TriangleAlert size={14} />
  {:else}
    <CircleAlert size={14} />
  {/if}
  <span class="mono">{status}</span>
</span>
