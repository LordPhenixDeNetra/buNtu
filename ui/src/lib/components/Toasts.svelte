<script lang="ts">
  import CircleCheck from '@lucide/svelte/icons/circle-check';
  import CircleAlert from '@lucide/svelte/icons/circle-alert';
  import X from '@lucide/svelte/icons/x';
  import { dismissToast, toasts } from '$lib/feedback';
  import { t } from '$lib/i18n';
</script>

<!-- Région toujours présente : les lecteurs d'écran annoncent chaque nouveau message. -->
<div class="toasts" role="status" aria-live="polite">
  {#each $toasts as toast (toast.id)}
    <div class="toast" class:toast-error={toast.kind === 'error'}>
      {#if toast.kind === 'success'}
        <CircleCheck size={18} />
      {:else}
        <CircleAlert size={18} />
      {/if}
      <span class="toast-message">{toast.message}</span>
      <button
        class="btn-ghost btn-icon"
        type="button"
        aria-label={$t('common.dismiss')}
        onclick={() => dismissToast(toast.id)}
      >
        <X size={16} />
      </button>
    </div>
  {/each}
</div>

<style>
  .toasts {
    position: fixed;
    right: var(--space-4);
    bottom: var(--space-4);
    z-index: 20;
    display: grid;
    gap: var(--space-2);
    width: min(24rem, calc(100vw - 2 * var(--space-4)));
  }

  .toast {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-2) var(--space-2) var(--space-2) var(--space-3);
    border: 1px solid var(--color-border);
    border-left: 3px solid var(--color-success);
    border-radius: var(--radius-card);
    background: var(--color-surface);
    box-shadow: var(--shadow-raised);
    color: var(--color-text);
    animation: toast-in var(--motion-slow) var(--ease-out);
  }

  .toast :global(svg:first-child) {
    flex: none;
    color: var(--color-success);
  }

  .toast-error {
    border-left-color: var(--color-danger);
  }

  .toast-error :global(svg:first-child) {
    color: var(--color-danger);
  }

  .toast-message {
    flex: 1;
    font-size: var(--text-sm);
  }

  @keyframes toast-in {
    from {
      opacity: 0;
      transform: translateY(8px);
    }
  }
</style>
