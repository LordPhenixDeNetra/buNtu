<script lang="ts">
  import { onMount } from 'svelte';
  import Plus from '@lucide/svelte/icons/plus';
  import Trash2 from '@lucide/svelte/icons/trash-2';
  import Lock from '@lucide/svelte/icons/lock';
  import KeyRound from '@lucide/svelte/icons/key-round';
  import Save from '@lucide/svelte/icons/save';
  import Server from '@lucide/svelte/icons/server';
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
    notify('success', { key: 'providers.created' });
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
      notify('error', { error });
    } else {
      notify('success', { key: 'providers.updated' });
    }
    await load();
  }

  async function deleteProvider(id: string): Promise<void> {
    if (!(await confirmAction($t('providers.confirmDelete'), $t('providers.delete')))) {
      return;
    }
    const { error } = await api.DELETE('/admin/v1/providers/{id}', { params: { path: { id } } });
    if (error) {
      notify('error', { error });
    } else {
      notify('success', { key: 'providers.deleted' });
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
      notify('error', { error });
    } else {
      keyDrafts[id] = '';
      notify('success', { key: 'providers.keyAdded' });
    }
    await load();
  }

  async function setWeight(id: string, index: number, weight: number): Promise<void> {
    if (!Number.isInteger(weight) || weight < 1) {
      await load(); // valeur invalide : on réaffiche le poids enregistré
      return;
    }
    const { error } = await api.PATCH('/admin/v1/providers/{id}/keys/{index}', {
      params: { path: { id, index } },
      body: { weight }
    });
    if (error) {
      notify('error', { error });
    } else {
      notify('success', { key: 'providers.weightSaved' });
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
      notify('error', { error });
    } else {
      notify('success', { key: 'providers.keyRemoved' });
    }
    await load();
  }

  onMount(load);
</script>

<header class="page-header">
  <div>
    <h1>{$t('providers.title')}</h1>
    <p>{$t('providers.subtitle')}</p>
  </div>
</header>

<form class="card create" onsubmit={createProvider}>
  <fieldset>
    <legend class="card-header create-title">
      <span class="icon-tile"><Plus size={16} /></span>{$t('providers.create')}
    </legend>
    <div class="card-body">
      <div class="form-grid">
        <div class="field">
          <label for="provider-id">{$t('providers.id')}</label>
          <input id="provider-id" class="mono" bind:value={form.id} autocomplete="off" required />
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
            class="mono"
            bind:value={form.base_url}
            placeholder="https://api.example.com"
            inputmode="url"
          />
        </div>
        <div class="field">
          <label for="provider-secret">{$t('providers.secret')}</label>
          <input
            id="provider-secret"
            class="mono"
            type="password"
            bind:value={form.secret}
            autocomplete="new-password"
            aria-describedby="provider-secret-hint"
          />
        </div>
      </div>
      {#if createError}
        <p class="field-error create-error" role="alert">{$t('error.title')} : {createError}</p>
      {/if}
    </div>
    <div class="card-footer">
      <p id="provider-secret-hint" class="field-hint spacer">
        <Lock size={12} />{$t('providers.secretNotice')}
      </p>
      <button type="submit" disabled={saving}>
        <Plus size={16} />
        {saving ? $t('common.saving') : $t('providers.save')}
      </button>
    </div>
  </fieldset>
</form>

{#if loadError}
  <p class="field-error" role="alert">{$t('error.title')} : {loadError}</p>
{/if}

<h2 class="section-title">
  <Server size={16} />{$t('providers.title')}
  {#if !loading}<span class="badge">{providers.length}</span>{/if}
</h2>

{#if loading}
  <div class="stack" aria-busy="true">
    <div class="skeleton card-skeleton"></div>
    <div class="skeleton card-skeleton"></div>
  </div>
{:else if providers.length === 0}
  <div class="empty">
    <span class="icon-tile"><Server size={18} /></span>
    <strong>{$t('providers.empty')}</strong>
  </div>
{:else}
  <div class="stack">
    {#each providers as provider (provider.id)}
      {@const readOnly = provider.source === 'file'}
      <section class="card" aria-labelledby={`provider-${provider.id}`}>
        <header class="card-header">
          <span class="icon-tile"><Server size={16} /></span>
          <h2 id={`provider-${provider.id}`} class="mono">{provider.id}</h2>
          <span class="badge mono">{provider.type}</span>
          <SourceBadge source={provider.source} />
          <span class="spacer"></span>
          {#if readOnly}
            <span class="muted small read-only"><Lock size={12} />{$t('source.fileHint')}</span>
          {:else}
            <button class="btn-danger" type="button" onclick={() => deleteProvider(provider.id)}>
              <Trash2 size={14} />
              {$t('providers.delete')}
            </button>
          {/if}
        </header>

        <div class="card-body columns">
          <div class="field">
            <label for={`base-url-${provider.id}`}>{$t('providers.baseUrl')}</label>
            <div class="inline">
              <input
                id={`base-url-${provider.id}`}
                class="mono"
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
                  <Save size={14} />
                  {$t('providers.save')}
                </button>
              {/if}
            </div>
          </div>

          <div class="field">
            <h3 class="keys-title">
              {$t('providers.keys')}<span class="badge">{provider.keys.length}</span>
            </h3>
            {#if provider.keys.length === 0}
              <p class="muted no-keys">{$t('providers.noKeys')}</p>
            {:else}
              <ul class="keys">
                {#each provider.keys as key, index (index)}
                  <li>
                    <KeyRound size={14} />
                    <code class="key-preview">{key.preview}</code>
                    {#if readOnly}
                      <span class="badge">{$t('providers.weight')} {key.weight}</span>
                    {:else}
                      <label class="weight">
                        <span aria-hidden="true">{$t('providers.weight')}</span>
                        <input
                          type="number"
                          class="mono"
                          min="1"
                          step="1"
                          inputmode="numeric"
                          value={key.weight}
                          aria-label={`${$t('providers.weight')} ${key.preview}`}
                          onchange={(event) =>
                            setWeight(provider.id, index, event.currentTarget.valueAsNumber)}
                        />
                      </label>
                      <button
                        class="btn-ghost btn-icon"
                        type="button"
                        aria-label={`${$t('providers.deleteKey')} ${key.preview}`}
                        onclick={() => removeKey(provider.id, index)}
                      >
                        <Trash2 size={14} />
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
                  class="mono"
                  type="password"
                  bind:value={keyDrafts[provider.id]}
                  autocomplete="new-password"
                  placeholder={$t('providers.newKey')}
                />
                <button class="btn-secondary" type="submit">
                  <Plus size={14} />
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
  .create-title {
    width: 100%;
    margin: 0;
    font-size: var(--text-sm);
    font-weight: 600;
  }

  .create-title .icon-tile {
    width: 1.75rem;
    height: 1.75rem;
    color: var(--color-accent-text);
  }

  .create-error {
    margin-top: var(--space-4);
  }

  .card-footer .field-hint {
    display: flex;
    align-items: center;
    gap: var(--space-1);
    min-width: 14rem;
  }

  .card-header h2 {
    font-size: var(--text-base);
  }

  .read-only {
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
  }

  .columns {
    display: grid;
    align-items: start;
    grid-template-columns: repeat(auto-fit, minmax(18rem, 1fr));
    gap: var(--space-5);
  }

  .keys-title {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    font-weight: 500;
  }

  .no-keys {
    padding: var(--space-3);
    border: 1px dashed var(--color-border-strong);
    border-radius: var(--radius-control);
    text-align: center;
  }

  .keys {
    margin: 0;
    padding: 0;
    overflow: hidden;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-control);
    list-style: none;
  }

  .keys li {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-2) var(--space-3);
    border-bottom: 1px solid var(--color-border);
    background: var(--color-surface-2);
    color: var(--color-muted);
  }

  .keys li:last-child {
    border-bottom: 0;
  }

  .key-preview {
    flex: 1;
    color: var(--color-text);
  }

  .weight {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    color: var(--color-muted);
    font-size: var(--text-xs);
    font-weight: 400;
  }

  .weight input {
    width: 4rem;
    min-height: 1.875rem;
    padding: 0 var(--space-2);
  }

  .card-skeleton {
    height: 10rem;
  }
</style>
