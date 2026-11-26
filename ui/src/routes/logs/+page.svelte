<script lang="ts">
  import { onMount, tick } from 'svelte';
  import Filter from '@lucide/svelte/icons/filter';
  import ChevronLeft from '@lucide/svelte/icons/chevron-left';
  import ChevronRight from '@lucide/svelte/icons/chevron-right';
  import PanelRightOpen from '@lucide/svelte/icons/panel-right-open';
  import X from '@lucide/svelte/icons/x';
  import ScrollText from '@lucide/svelte/icons/scroll-text';
  import { goto } from '$app/navigation';
  import { page } from '$app/state';
  import { api, errorMessage } from '$lib/api';
  import type { components } from '$lib/api/schema';
  import { locale, t } from '$lib/i18n';
  import { formatCount, formatDate, formatMs } from '$lib/format';
  import StatusBadge from '$lib/components/StatusBadge.svelte';

  type LogView = components['schemas']['RequestLogView'];

  let logs = $state<LogView[]>([]);
  let detail = $state<LogView | null>(null);
  let nextCursor = $state<number | null>(null);
  /** Curseurs des pages déjà vues, pour revenir en arrière (`null` = première page). */
  let history = $state<(number | null)[]>([]);
  let current: number | null = null;
  let loading = $state(true);
  let error = $state('');
  let filters = $state({ provider: '', status: '', model: '' });
  let limit = $state('20');
  let detailHeading: HTMLHeadingElement | undefined = $state();
  let detailOpener: HTMLElement | null = null;

  function applyUrl(): void {
    const params = page.url.searchParams;
    filters = {
      provider: params.get('provider') ?? '',
      status: params.get('status') ?? '',
      model: params.get('model') ?? ''
    };
    limit = params.get('limit') ?? '20';
  }

  function urlFor(): string {
    const params = new URLSearchParams();
    if (filters.provider.trim() !== '') params.set('provider', filters.provider.trim());
    if (filters.status.trim() !== '') params.set('status', filters.status.trim());
    if (filters.model.trim() !== '') params.set('model', filters.model.trim());
    params.set('limit', limit);
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
      error = errorMessage(requestError, $t('error.unexpected'));
      return;
    }
    current = cursor;
    logs = data?.data ?? [];
    nextCursor = data?.next_cursor ?? null;
  }

  async function nextPage(): Promise<void> {
    history = [...history, current];
    await load(nextCursor);
  }

  async function previousPage(): Promise<void> {
    const previous = history.at(-1) ?? null;
    history = history.slice(0, -1);
    await load(previous);
  }

  async function showDetail(entry: LogView, event: MouseEvent): Promise<void> {
    detailOpener = event.currentTarget as HTMLElement;
    const { data, error: requestError } = await api.GET('/admin/v1/requests/{id}', {
      params: { path: { id: entry.id } }
    });
    if (requestError) {
      error = errorMessage(requestError, $t('error.unexpected'));
      return;
    }
    detail = data ?? null;
    // Le focus suit l'ouverture du détail (MASTER, règle 5).
    await tick();
    detailHeading?.focus();
  }

  function closeDetail(): void {
    detail = null;
    detailOpener?.focus();
  }

  async function submit(event: SubmitEvent): Promise<void> {
    event.preventDefault();
    history = [];
    await goto(urlFor(), { replaceState: false, keepFocus: true });
    await load(null);
  }

  onMount(async () => {
    applyUrl();
    await load(null);
  });
</script>

<svelte:window
  onkeydown={(event) => {
    if (event.key === 'Escape' && detail !== null) {
      closeDetail();
    }
  }}
/>

<header class="page-header">
  <div>
    <h1>{$t('logs.title')}</h1>
    <p>{$t('logs.subtitle')}</p>
  </div>
</header>

{#if error}
  <p class="field-error" role="alert">{$t('error.title')} : {error}</p>
{/if}

<form class="filters card" onsubmit={submit}>
  <div class="field">
    <label for="filter-provider">{$t('logs.provider')}</label>
    <input id="filter-provider" class="mono" bind:value={filters.provider} />
  </div>
  <div class="field">
    <label for="filter-status">{$t('logs.status')}</label>
    <input id="filter-status" class="mono" inputmode="numeric" bind:value={filters.status} />
  </div>
  <div class="field">
    <label for="filter-model">{$t('logs.model')}</label>
    <input id="filter-model" class="mono" bind:value={filters.model} />
  </div>
  <div class="field">
    <label for="filter-limit">{$t('logs.limit')}</label>
    <select id="filter-limit" bind:value={limit}>
      <option value="20">20</option>
      <option value="50">50</option>
      <option value="100">100</option>
    </select>
  </div>
  <button type="submit"><Filter size={14} />{$t('logs.filter')}</button>
</form>

<div class="layout" class:with-detail={detail !== null}>
  <div class="results">
    {#if loading}
      <div class="stack" aria-busy="true">
        {#each [0, 1, 2, 3, 4] as row (row)}
          <div class="skeleton row-skeleton"></div>
        {/each}
      </div>
    {:else if logs.length === 0}
      <div class="empty">
        <span class="icon-tile"><ScrollText size={18} /></span>
        <strong>{$t('logs.empty')}</strong>
      </div>
    {:else}
      <div class="table-wrap">
        <table>
          <thead>
            <tr>
              <th scope="col">{$t('logs.date')}</th>
              <th scope="col">{$t('logs.provider')}</th>
              <th scope="col">{$t('logs.model')}</th>
              <th scope="col">{$t('logs.status')}</th>
              <th scope="col" class="num">{$t('logs.latency')}</th>
              <th scope="col" class="num">{$t('logs.attempts')}</th>
              <th scope="col" class="num" title={$t('logs.tokens')}>{$t('logs.tokensShort')}</th>
              <th scope="col"><span class="sr-only">{$t('logs.detail')}</span></th>
            </tr>
          </thead>
          <tbody>
            {#each logs as entry (entry.id)}
              <tr class:selected={detail?.id === entry.id}>
                <td class="mono muted">{formatDate(entry.created_at, $locale)}</td>
                <td>{entry.provider || '—'}</td>
                <td class="mono">{entry.model}</td>
                <td><StatusBadge status={entry.status} /></td>
                <td class="num mono">{formatMs(entry.latency_ms, $locale)}</td>
                <td class="num mono">{entry.attempts}</td>
                <td class="num mono muted">
                  {formatCount(entry.input_tokens, $locale)} / {formatCount(entry.output_tokens, $locale)}
                </td>
                <td class="num">
                  <button
                    class="btn-ghost row-action"
                    type="button"
                    onclick={(event) => showDetail(entry, event)}
                  >
                    <PanelRightOpen size={14} />{$t('logs.detail')}
                  </button>
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
      <div class="pager">
        <button class="btn-secondary" type="button" disabled={history.length === 0} onclick={previousPage}>
          <ChevronLeft size={14} />{$t('logs.previous')}
        </button>
        <button class="btn-secondary" type="button" disabled={nextCursor === null} onclick={nextPage}>
          {$t('logs.next')}<ChevronRight size={14} />
        </button>
      </div>
    {/if}
  </div>

  {#if detail !== null}
    <section class="card detail" aria-label={$t('logs.detail')}>
      <header class="card-header">
        <h2 bind:this={detailHeading} tabindex="-1">{$t('logs.detail')}</h2>
        <StatusBadge status={detail.status} />
        <span class="spacer"></span>
        <button class="btn-ghost btn-icon" type="button" aria-label={$t('common.close')} onclick={closeDetail}>
          <X size={16} />
        </button>
      </header>
      <dl class="card-body">
        <div class="wide"><dt>{$t('logs.id')}</dt><dd class="mono break">{detail.id}</dd></div>
        <div class="wide"><dt>{$t('logs.date')}</dt><dd class="mono">{formatDate(detail.created_at, $locale)}</dd></div>
        <div><dt>{$t('logs.status')}</dt><dd class="mono">{detail.status}</dd></div>
        <div><dt>{$t('logs.provider')}</dt><dd>{detail.provider || '—'}</dd></div>
        <div class="wide"><dt>{$t('logs.model')}</dt><dd class="mono">{detail.model}</dd></div>
        <div><dt>{$t('logs.latency')}</dt><dd class="mono">{formatMs(detail.latency_ms, $locale)}</dd></div>
        <div><dt>{$t('logs.attempts')}</dt><dd class="mono">{detail.attempts}</dd></div>
        <div class="wide">
          <dt>{$t('logs.tokens')}</dt>
          <dd class="mono">
            {formatCount(detail.input_tokens, $locale)} / {formatCount(detail.output_tokens, $locale)}
          </dd>
        </div>
      </dl>
    </section>
  {/if}
</div>

<style>
  .filters {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(9rem, 1fr));
    gap: var(--space-3);
    align-items: end;
    margin-bottom: var(--space-4);
    padding: var(--space-3) var(--space-4);
  }

  .layout {
    display: grid;
    gap: var(--space-4);
  }

  .results {
    min-width: 0;
  }

  @media (min-width: 80rem) {
    .layout.with-detail {
      grid-template-columns: minmax(0, 1fr) 20rem;
      align-items: start;
    }
  }

  tr.selected,
  tr.selected:hover {
    background: color-mix(in srgb, var(--color-accent) var(--tint-strength), transparent);
  }

  td.num:last-child {
    padding-right: var(--space-2);
  }

  .row-action {
    min-height: 1.875rem;
    padding: 0 var(--space-2);
    font-size: var(--text-xs);
  }

  .pager {
    display: flex;
    justify-content: flex-end;
    gap: var(--space-2);
    margin-top: var(--space-3);
  }

  .row-skeleton {
    height: 2.5rem;
  }

  .detail {
    position: sticky;
    top: var(--space-4);
    animation: detail-in var(--motion-slow) var(--ease-out);
  }

  .detail h2 {
    font-size: var(--text-sm);
  }

  dl {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: var(--space-4) var(--space-3);
    margin: 0;
  }

  dl .wide {
    grid-column: 1 / -1;
  }

  dt {
    color: var(--color-muted);
    font-size: var(--text-xs);
  }

  dd {
    margin: var(--space-1) 0 0;
  }

  .break {
    word-break: break-all;
  }

  @keyframes detail-in {
    from {
      opacity: 0;
      transform: translateX(8px);
    }
  }
</style>
