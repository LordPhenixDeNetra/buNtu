<script lang="ts">
  import TriangleAlert from '@lucide/svelte/icons/triangle-alert';
  import { confirmRequest } from '$lib/feedback';
  import { t } from '$lib/i18n';

  // `<dialog>` natif : piège du focus, Échap et retour du focus fournis par le navigateur.
  let dialog: HTMLDialogElement | undefined = $state();
  let opener: Element | null = null;

  $effect(() => {
    if ($confirmRequest && dialog && !dialog.open) {
      opener = document.activeElement;
      dialog.showModal();
    }
  });

  function answer(confirmed: boolean): void {
    const request = $confirmRequest;
    confirmRequest.set(null);
    if (dialog?.open) {
      dialog.close();
    }
    if (opener instanceof HTMLElement) {
      opener.focus();
    }
    request?.resolve(confirmed);
  }
</script>

<dialog
  bind:this={dialog}
  aria-labelledby="confirm-title"
  aria-describedby="confirm-message"
  oncancel={(event) => {
    event.preventDefault();
    answer(false);
  }}
>
  {#if $confirmRequest}
    <div class="body">
      <TriangleAlert size={22} />
      <div>
        <h2 id="confirm-title">{$t('common.confirm')}</h2>
        <p id="confirm-message">{$confirmRequest.message}</p>
      </div>
    </div>
    <div class="actions">
      <button class="btn-secondary" type="button" onclick={() => answer(false)}>
        {$t('common.cancel')}
      </button>
      <button class="btn-danger" type="button" onclick={() => answer(true)}>
        {$confirmRequest.confirmLabel}
      </button>
    </div>
  {/if}
</dialog>

<style>
  dialog {
    width: min(28rem, calc(100vw - 2 * var(--space-4)));
    padding: var(--space-5);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-tile);
    background: var(--color-surface);
    color: var(--color-text);
    box-shadow: var(--shadow-raised);
  }

  dialog::backdrop {
    background: rgb(0 0 0 / 0.45);
  }

  dialog[open] {
    animation: dialog-in var(--motion-base) var(--ease-out);
  }

  .body {
    display: flex;
    gap: var(--space-3);
  }

  .body :global(svg) {
    flex: none;
    color: var(--color-danger);
  }

  h2 {
    margin: 0 0 var(--space-1);
    font-size: var(--text-lg);
  }

  p {
    margin: 0;
    color: var(--color-muted);
  }

  .actions {
    display: flex;
    justify-content: flex-end;
    gap: var(--space-2);
    margin-top: var(--space-5);
  }

  @keyframes dialog-in {
    from {
      opacity: 0;
      transform: scale(0.97);
    }
  }
</style>
