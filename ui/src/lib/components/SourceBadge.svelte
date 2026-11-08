<script lang="ts">
  import FileCog from '@lucide/svelte/icons/file-cog';
  import Database from '@lucide/svelte/icons/database';
  import { t } from '$lib/i18n';

  // Origine d'une entrée (ADR-0008) : le fichier est prioritaire et non modifiable ici.
  let { source }: { source: string } = $props();
</script>

{#if source === 'file'}
  <span class="badge badge-brand" title={$t('source.fileHint')}>
    <FileCog size={14} />
    {$t('source.file')}
  </span>
{:else}
  <span class="badge">
    <Database size={14} />
    {$t('source.database')}
  </span>
{/if}
