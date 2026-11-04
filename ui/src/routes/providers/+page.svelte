<script lang="ts">
  import { onMount } from 'svelte';
  import { api, errorMessage } from '$lib/api';
  import type { components } from '$lib/api/schema';
  import { t } from '$lib/i18n';

  type ProviderView = components['schemas']['ProviderView'];

  let providers: ProviderView[] = [];
  let loading = true;
  let error = '';
  let notice = '';
  let saving = false;
  let form = { id: '', type: 'mock', base_url: '', secret: '' };
  let keyDrafts: Record<string, string> = {};

  const types = ['openai', 'anthropic', 'gemini', 'compatible', 'mock'];

  async function load(): Promise<void> {
    loading = true;
    error = '';
    const { data, error: requestError } = await api.GET('/admin/v1/providers');
    if (requestError) {
      error = errorMessage(requestError);
    } else {
      providers = data?.data ?? [];
    }
    loading = false;
  }

  async function createProvider(): Promise<void> {
    saving = true;
    error = '';
    notice = '';
    const { error: requestError } = await api.POST('/admin/v1/providers', {
      body: {
        id: form.id.trim(),
        type: form.type,
        base_url: form.base_url.trim() === '' ? null : form.base_url.trim(),
        keys: form.secret.trim() === '' ? [] : [{ secret: form.secret.trim(), weight: 1 }]
      }
    });
    saving = false;
    if (requestError) {
      error = errorMessage(requestError);
      return;
    }
    notice = 'Fournisseur créé.';
    form = { id: '', type: 'mock', base_url: '', secret: '' };
    await load();
  }

  async function saveBaseUrl(provider: ProviderView): Promise<void> {
    const { error: requestError } = await api.PATCH('/admin/v1/providers/{id}', {
      params: { path: { id: provider.id } },
      body: { base_url: provider.base_url ?? null }
    });
    if (requestError) {
      error = errorMessage(requestError);
    }
    await load();
  }

  async function deleteProvider(id: string): Promise<void> {
    if (!confirm($t('providers.confirmDelete'))) {
      return;
    }
    const { error: requestError } = await api.DELETE('/admin/v1/providers/{id}', {
      params: { path: { id } }
    });
    if (requestError) {
      error = errorMessage(requestError);
    }
    await load();
  }

  async function addKey(id: string): Promise<void> {
    const secret = (keyDrafts[id] ?? '').trim();
    if (secret === '') {
      return;
    }
    const { error: requestError } = await api.POST('/admin/v1/providers/{id}/keys', {
      params: { path: { id } },
      body: { secret, weight: 1 }
    });
    if (requestError) {
      error = errorMessage(requestError);
    } else {
      keyDrafts[id] = '';
      notice = 'Clé ajoutée : elle ne sera plus jamais affichée en clair.';
    }
    await load();
  }

  async function removeKey(id: string, index: number): Promise<void> {
    const { error: requestError } = await api.DELETE('/admin/v1/providers/{id}/keys/{index}', {
      params: { path: { id, index } }
    });
    if (requestError) {
      error = errorMessage(requestError);
    }
    await load();
  }

  onMount(load);
</script>

<h1>{$t('providers.title')}</h1>
<p style="color: var(--muted);">{$t('providers.subtitle')}</p>

{#if notice}
  <p role="status" style="color: var(--accent);">{notice}</p>
{/if}
{#if error}
  <p role="alert" style="color: var(--danger);">{$t('error.title')} : {error}</p>
{/if}

<fieldset>
  <legend>{$t('providers.create')}</legend>
  <div style="display: grid; grid-template-columns: repeat(auto-fit, minmax(12rem, 1fr)); gap: 0.9rem;">
    <div>
      <label for="provider-id">{$t('providers.id')}</label>
      <input id="provider-id" bind:value={form.id} autocomplete="off" />
    </div>
    <div>
      <label for="provider-type">{$t('providers.type')}</label>
      <select id="provider-type" bind:value={form.type}>
        {#each types as kind (kind)}
          <option value={kind}>{kind}</option>
        {/each}
      </select>
    </div>
    <div>
      <label for="provider-base-url">{$t('providers.baseUrl')}</label>
      <input id="provider-base-url" bind:value={form.base_url} placeholder="http://127.0.0.1:9000" />
    </div>
    <div>
      <label for="provider-secret">{$t('providers.secret')}</label>
      <input id="provider-secret" type="password" bind:value={form.secret} autocomplete="new-password" />
    </div>
  </div>
  <p style="color: var(--muted); font-size: 0.9rem;">{$t('providers.secretNotice')}</p>
  <button type="button" on:click={createProvider} disabled={saving}>{$t('providers.save')}</button>
</fieldset>

{#if loading}
  <p>{$t('loading')}</p>
{:else if providers.length === 0}
  <p>{$t('providers.empty')}</p>
{:else}
  <div style="display: grid; gap: 1rem;">
    {#each providers as provider (provider.id)}
      <section
        style="border: 1px solid var(--border); border-radius: 8px; padding: 1rem; background: var(--panel);"
      >
        <header style="display: flex; gap: 0.75rem; align-items: baseline; flex-wrap: wrap;">
          <h2 style="margin: 0; font-size: 1.1rem;">{provider.id}</h2>
          <span style="color: var(--muted);">{provider.type}</span>
          <span style="margin-left: auto;">
            <button class="danger" type="button" on:click={() => deleteProvider(provider.id)}>
              {$t('providers.delete')}
            </button>
          </span>
        </header>

        <div style="margin-top: 0.75rem; display: grid; gap: 1rem; grid-template-columns: repeat(auto-fit, minmax(18rem, 1fr));">
          <div>
            <label for={`base-url-${provider.id}`}>{$t('providers.baseUrl')}</label>
            <div style="display: flex; gap: 0.5rem;">
              <input id={`base-url-${provider.id}`} bind:value={provider.base_url} placeholder="http://127.0.0.1:9000" />
              <button type="button" on:click={() => saveBaseUrl(provider)}>{$t('providers.save')}</button>
            </div>
          </div>

          <div>
            <strong>{$t('providers.keyPreview')}</strong>
            <ul style="list-style: none; padding: 0;">
              {#each provider.keys as key, index (index)}
                <li style="display: flex; gap: 0.5rem; align-items: center;">
                  <code style="flex: 1;">{key.preview}</code>
                  <span>{$t('providers.weight')} : {key.weight}</span>
                  <button class="danger" type="button" on:click={() => removeKey(provider.id, index)}>
                    {$t('providers.deleteKey')}
                  </button>
                </li>
              {/each}
            </ul>
            <label for={`new-key-${provider.id}`}>{$t('providers.addKey')}</label>
            <div style="display: flex; gap: 0.5rem;">
              <input
                id={`new-key-${provider.id}`}
                type="password"
                bind:value={keyDrafts[provider.id]}
                autocomplete="new-password"
              />
              <button type="button" on:click={() => addKey(provider.id)}>{$t('providers.addKey')}</button>
            </div>
          </div>
        </div>
      </section>
    {/each}
  </div>
{/if}
