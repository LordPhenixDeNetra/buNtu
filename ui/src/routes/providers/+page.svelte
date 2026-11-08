<script lang="ts">
  import { onMount } from 'svelte';
  import Plus from '@lucide/svelte/icons/plus';
  import Trash2 from '@lucide/svelte/icons/trash-2';
  import Lock from '@lucide/svelte/icons/lock';
  import KeyRound from '@lucide/svelte/icons/key-round';
  import Save from '@lucide/svelte/icons/save';
  import { api, errorMessage } from '$lib/api';
  import type { components } from '$lib/api/schema';
  import { t } from '$lib/i18n';
  import { confirmAction, notify } from '$lib/feedback';
  import SourceBadge from '$lib/components/SourceBadge.svelte';

  type ProviderView = components['schemas']['ProviderView'];

  /** Miroir de `PROVIDER_KINDS` (buntu-core) : la validation serveur refuse tout autre type. */
  const types = ['openai', 'anthropic', 'gemini', 'compatible', 'openai_compatible', 'ollama', 'mock'];

  let providers = $state<ProviderView[]>([]);
  let loading = $state(true);
  let loadError = $state('');
  let createError = $state('');
  let saving = $state(false);
  let form = $state({ id: '', type: 'mock', base_url: '', secret: '' });
  let baseUrls = $state<Record<string, string>>({});
  let keyDrafts = $state<Record<string, string>>({});

  function failure(error: unknown): string {
    return errorMessage(error, $t('error.unexpected'));
  }

  /** Recharge la liste sur place : pas de squelette après le premier chargement. */
  async function load(): Promise<void> {
    const { data, error } = await api.GET('/admin/v1/providers', {
      params: { query: { limit: 100 } }
    });
    if (error) {
      loadError = failure(error);
    } else {
      loadError = '';
      providers = data?.data ?? [];
      baseUrls = Object.fromEntries(providers.map((p) => [p.id, p.base_url ?? '']));
    }
    loading = false;
  }

  async function createProvider(event: SubmitEvent): Promise<void> {
    event.preventDefault();
    saving = true;
    createError = '';
    const { error } = await api.POST('/admin/v1/providers', {
      body: {
        id: form.id.trim(),
        type: form.type,
        base_url: form.base_url.trim() === '' ? null : form.base_url.trim(),
        keys: form.secret.trim() === '' ? [] : [{ secret: form.secret.trim(), weight: 1 }]
      }
    });
    saving = false;
    if (error) {
      createError = failure(error);
      return;
    }
    notify('success', $t('providers.created'));
    form = { id: '', type: form.type, base_url: '', secret: '' };
    await load();
  }

  async function saveBaseUrl(id: string): Promise<void> {
    const value = (baseUrls[id] ?? '').trim();
    const { error } = await api.PATCH('/admin/v1/providers/{id}', {
      params: { path: { id } },
      body: { base_url: value === '' ? null : value }
    });
    if (error) {
      notify('error', failure(error));
    } else {
      notify('success', $t('providers.updated'));
    }
    await load();
  }

  async function deleteProvider(id: string): Promise<void> {
    if (!(await confirmAction($t('providers.confirmDelete'), $t('providers.delete')))) {
      return;
    }
    const { error } = await api.DELETE('/admin/v1/providers/{id}', { params: { path: { id } } });
    if (error) {
      notify('error', failure(error));
    } else {
      notify('success', $t('providers.deleted'));
    }
    await load();
  }

  async function addKey(event: SubmitEvent, id: string): Promise<void> {
    event.preventDefault();
    const secret = (keyDrafts[id] ?? '').trim();
    if (secret === '') {
      return;
    }
    const { error } = await api.POST('/admin/v1/providers/{id}/keys', {
      params: { path: { id } },
      body: { secret, weight: 1 }
    });
    if (error) {
      notify('error', failure(error));
    } else {
      keyDrafts[id] = '';
      notify('success', $t('providers.keyAdded'));
    }
    await load();
  }

  async function removeKey(id: string, index: number): Promise<void> {
    if (!(await confirmAction($t('providers.confirmDeleteKey'), $t('providers.deleteKey')))) {
      return;
    }
    const { error } = await api.DELETE('/admin/v1/providers/{id}/keys/{index}', {
      params: { path: { id, index } }
    });
    if (error) {
      notify('error', failure(error));
    } else {
      notify('success', $t('providers.keyRemoved'));
    }
    await load();
  }

  onMount(load);
</script>

<h1>{$t('providers.title')}</h1>
<p class="muted lead">{$t('providers.subtitle')}</p>

<form onsubmit={createProvider}>
  <fieldset>
    <legend>{$t('providers.create')}</legend>
    <div class="grid">
      <div class="field">
        <label for="provider-id">{$t('providers.id')}</label>
        <input id="provider-id" bind:value={form.id} autocomplete="off" required />
      </div>
      <div class="field">
        <label for="provider-type">{$t('providers.type')}</label>
        <select id="provider-type" bind:value={form.type}>
          {#each types as kind (kind)}
            <option value={kind}>{kind}</option>
          {/each}
        </select>
      </div>
      <div class="field">
        <label for="provider-base-url">{$t('providers.baseUrl')}</label>
        <input
          id="provider-base-url"
          bind:value={form.base_url}
          placeholder="https://api.example.com"
          inputmode="url"
        />
      </div>
      <div class="field">
        <label for="provider-secret">{$t('providers.secret')}</label>
        <input
          id="provider-secret"
          type="password"
          bind:value={form.secret}
          autocomplete="new-password"
          aria-describedby="provider-secret-hint"
        />
      </div>
    </div>
    <p id="provider-secret-hint" class="field-hint">{$t('providers.secretNotice')}</p>
    {#if createError}
      <p class="field-error" role="alert">{$t('error.title')} : {createError}</p>
    {/if}
    <button type="submit" disabled={saving}>
      <Plus size={18} />
      {saving ? $t('common.saving') : $t('providers.save')}
    </button>
  </fieldset>
</form>

{#if loadError}
  <p class="field-error" role="alert">{$t('error.title')} : {loadError}</p>
{/if}

{#if loading}
  <div class="stack" aria-busy="true">
    <div class="skeleton card-skeleton"></div>
    <div class="skeleton card-skeleton"></div>
  </div>
{:else if providers.length === 0}
  <p class="muted">{$t('providers.empty')}</p>
{:else}
  <div class="stack">
    {#each providers as provider (provider.id)}
      {@const readOnly = provider.source === 'file'}
      <section class="card" aria-labelledby={`provider-${provider.id}`}>
        <header class="card-header">
          <h2 id={`provider-${provider.id}`} class="mono">{provider.id}</h2>
          <span class="badge mono">{provider.type}</span>
          <SourceBadge source={provider.source} />
          <span class="spacer"></span>
          {#if readOnly}
            <span class="muted small read-only"><Lock size={14} />{$t('source.fileHint')}</span>
          {:else}
            <button class="btn-danger" type="button" onclick={() => deleteProvider(provider.id)}>
              <Trash2 size={16} />
              {$t('providers.delete')}
            </button>
          {/if}
        </header>

        <div class="card-body">
          <div class="field">
            <label for={`base-url-${provider.id}`}>{$t('providers.baseUrl')}</label>
            <div class="inline">
              <input
                id={`base-url-${provider.id}`}
                bind:value={baseUrls[provider.id]}
                placeholder="https://api.example.com"
                disabled={readOnly}
              />
              {#if !readOnly}
                <button
                  class="btn-secondary"
                  type="button"
                  onclick={() => saveBaseUrl(provider.id)}
                  disabled={(baseUrls[provider.id] ?? '') === (provider.base_url ?? '')}
                >
                  <Save size={16} />
                  {$t('providers.save')}
                </button>
              {/if}
            </div>
          </div>

          <div class="field">
            <h3 class="label">{$t('providers.keys')}</h3>
            {#if provider.keys.length === 0}
              <p class="muted small">{$t('providers.noKeys')}</p>
            {:else}
              <ul class="keys">
                {#each provider.keys as key, index (index)}
                  <li>
                    <KeyRound size={16} />
                    <code class="key-preview">{key.preview}</code>
                    <span class="badge">{$t('providers.weight')} {key.weight}</span>
                    {#if !readOnly}
                      <button
                        class="btn-ghost btn-icon"
                        type="button"
                        aria-label={`${$t('providers.deleteKey')} ${key.preview}`}
                        onclick={() => removeKey(provider.id, index)}
                      >
                        <Trash2 size={16} />
                      </button>
                    {/if}
                  </li>
                {/each}
              </ul>
            {/if}
            {#if !readOnly}
              <form class="inline" onsubmit={(event) => addKey(event, provider.id)}>
                <label for={`new-key-${provider.id}`} class="sr-only">{$t('providers.newKey')}</label>
                <input
                  id={`new-key-${provider.id}`}
                  type="password"
                  bind:value={keyDrafts[provider.id]}
                  autocomplete="new-password"
                  placeholder={$t('providers.newKey')}
                />
                <button class="btn-secondary" type="submit">
                  <Plus size={16} />
                  {$t('providers.addKey')}
                </button>
              </form>
            {/if}
          </div>
        </div>
      </section>
    {/each}
  </div>
{/if}

<style>
  .lead {
    margin: 0 0 var(--space-5);
  }

  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(13rem, 1fr));
    gap: var(--space-4);
    margin-bottom: var(--space-3);
  }

  fieldset .field-hint {
    margin: 0 0 var(--space-3);
  }

  .card-header {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-2);
  }

  .card-header h2 {
    margin: 0;
    font-size: var(--text-base);
  }

  .spacer {
    flex: 1;
  }

  .read-only {
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
  }

  .card-body {
    display: grid;
    align-items: start;
    grid-template-columns: repeat(auto-fit, minmax(18rem, 1fr));
    gap: var(--space-5);
    margin-top: var(--space-4);
  }

  .inline {
    display: flex;
    gap: var(--space-2);
  }

  .label {
    margin: 0 0 var(--space-1);
    font-size: var(--text-sm);
    font-weight: 600;
  }

  .keys {
    display: grid;
    gap: var(--space-1);
    margin: 0 0 var(--space-2);
    padding: 0;
    list-style: none;
  }

  .keys li {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    color: var(--color-muted);
  }

  .key-preview {
    flex: 1;
    color: var(--color-text);
  }

  .small {
    margin: 0;
    font-size: var(--text-sm);
  }

  .card-skeleton {
    height: 9rem;
  }
</style>
