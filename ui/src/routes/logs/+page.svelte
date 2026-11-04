<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { page } from '$app/state';
  import { api, errorMessage } from '$lib/api';
  import type { components } from '$lib/api/schema';
  import { t } from '$lib/i18n';

  type LogView = components['schemas']['RequestLogView'];

  let logs: LogView[] = [];
  let detail: LogView | null = null;
  let nextCursor: number | null = null;
  let loading = false;
  let error = '';
  let filters = { provider: '', status: '', model: '' };
  let limit = '20';

  function applyUrl(): void {
    const params = page.url.searchParams;
    filters = {
      provider: params.get('provider') ?? '',
      status: params.get('status') ?? '',
      model: params.get('model') ?? ''
    };
    limit = params.get('limit') ?? '20';
  }

  function urlFor(cursor: number | null): string {
    const params = new URLSearchParams();
    if (filters.provider.trim() !== '') params.set('provider', filters.provider.trim());
    if (filters.status.trim() !== '') params.set('status', filters.status.trim());
    if (filters.model.trim() !== '') params.set('model', filters.model.trim());
    params.set('limit', limit);
    if (cursor !== null) params.set('cursor', String(cursor));
    return `/logs?${params.toString()}`;
  }

  async function load(cursor: number | null): Promise<void> {
    loading = true;
    error = '';
    const query: Record<string, string | number> = { limit: Number(limit) || 20 };
    if (filters.provider.trim() !== '') query.provider = filters.provider.trim();
    if (filters.status.trim() !== '') query.status = Number(filters.status);
    if (filters.model.trim() !== '') query.model = filters.model.trim();
    if (cursor !== null) query.cursor = cursor;
    const { data, error: requestError } = await api.GET('/admin/v1/requests', {
      params: { query: query as never }
    });
    loading = false;
    if (requestError) {
      error = errorMessage(requestError);
      return;
    }
    logs = data?.data ?? [];
    nextCursor = data?.next_cursor ?? null;
  }

  async function showDetail(id: string): Promise<void> {
    detail = null;
    const { data, error: requestError } = await api.GET('/admin/v1/requests/{id}', {
      params: { path: { id } }
    });
    if (requestError) {
      error = errorMessage(requestError);
      return;
    }
    detail = data ?? null;
  }

  async function submit(): Promise<void> {
    await goto(urlFor(null), { replaceState: false });
    await load(null);
  }

  onMount(async () => {
    applyUrl();
    await load(null);
  });
</script>

<h1>{$t('logs.title')}</h1>
<p style="color: var(--muted);">{$t('logs.subtitle')}</p>

{#if error}
  <p role="alert" style="color: var(--danger);">{$t('error.title')} : {error}</p>
{/if}

<form
  style="display: grid; grid-template-columns: repeat(auto-fit, minmax(11rem, 1fr)); gap: 0.75rem; align-items: end; margin-bottom: 1rem;"
  on:submit|preventDefault={submit}
>
  <div>
    <label for="filter-provider">{$t('logs.provider')}</label>
    <input id="filter-provider" bind:value={filters.provider} />
  </div>
  <div>
    <label for="filter-status">{$t('logs.status')}</label>
    <input id="filter-status" inputmode="numeric" bind:value={filters.status} />
  </div>
  <div>
    <label for="filter-model">{$t('logs.model')}</label>
    <input id="filter-model" bind:value={filters.model} />
  </div>
  <div>
    <label for="filter-limit">{$t('logs.limit')}</label>
    <input id="filter-limit" inputmode="numeric" bind:value={limit} />
  </div>
  <button type="submit">{$t('logs.filter')}</button>
</form>

{#if loading}
  <p>{$t('loading')}</p>
{:else if logs.length === 0}
  <p>{$t('logs.empty')}</p>
{:else}
  <table>
    <thead>
      <tr>
        <th>{$t('logs.date')}</th>
        <th>{$t('logs.provider')}</th>
        <th>{$t('logs.model')}</th>
        <th>{$t('logs.status')}</th>
        <th>{$t('logs.latency')}</th>
        <th>{$t('logs.attempts')}</th>
        <th>{$t('logs.tokens')}</th>
        <th></th>
      </tr>
    </thead>
    <tbody>
      {#each logs as entry (entry.id)}
        <tr>
          <td><code>{entry.id}</code></td>
          <td>{entry.provider}</td>
          <td>{entry.model}</td>
          <td>{entry.status}</td>
          <td>{entry.latency_ms} ms</td>
          <td>{entry.attempts}</td>
          <td>{entry.input_tokens} / {entry.output_tokens}</td>
          <td>
            <button class="secondary" type="button" on:click={() => showDetail(entry.id)}>
              {$t('logs.detail')}
            </button>
          </td>
        </tr>
      {/each}
    </tbody>
  </table>
  {#if nextCursor !== null}
    <p style="margin-top: 1rem;">
      <button class="secondary" type="button" on:click={() => load(nextCursor)}>
        {$t('logs.next')}
      </button>
    </p>
  {/if}
{/if}

{#if detail !== null}
  <section
    style="margin-top: 1.5rem; border: 1px solid var(--border); border-radius: 8px; padding: 1rem; background: var(--panel);"
    aria-label={$t('logs.detail')}
  >
    <h2 style="font-size: 1.05rem;">{$t('logs.detail')} — <code>{detail.id}</code></h2>
    <dl style="display: grid; grid-template-columns: repeat(auto-fit, minmax(12rem, 1fr)); gap: 0.5rem;">
      <div><dt>{$t('logs.provider')}</dt><dd>{detail.provider}</dd></div>
      <div><dt>{$t('logs.model')}</dt><dd>{detail.model}</dd></div>
      <div><dt>{$t('logs.status')}</dt><dd>{detail.status}</dd></div>
      <div><dt>{$t('logs.latency')}</dt><dd>{detail.latency_ms} ms</dd></div>
      <div><dt>{$t('logs.attempts')}</dt><dd>{detail.attempts}</dd></div>
      <div><dt>{$t('logs.tokens')}</dt><dd>{detail.input_tokens} / {detail.output_tokens}</dd></div>
    </dl>
    <button class="secondary" type="button" on:click={() => (detail = null)}>{$t('common.close')}</button>
  </section>
{/if}
