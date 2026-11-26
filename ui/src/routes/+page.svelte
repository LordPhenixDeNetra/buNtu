<script lang="ts">
  import { onMount } from 'svelte';
  import Activity from '@lucide/svelte/icons/activity';
  import Server from '@lucide/svelte/icons/server';
  import Route from '@lucide/svelte/icons/route';
  import Gauge from '@lucide/svelte/icons/gauge';
  import Timer from '@lucide/svelte/icons/timer';
  import Hash from '@lucide/svelte/icons/hash';
  import CircleAlert from '@lucide/svelte/icons/circle-alert';
  import CircleCheck from '@lucide/svelte/icons/circle-check';
  import ArrowRight from '@lucide/svelte/icons/arrow-right';
  import { api } from '$lib/api';
  import type { components } from '$lib/api/schema';
  import { locale, t } from '$lib/i18n';
  import { formatCount, formatDate, formatMs, formatPercent } from '$lib/format';
  import StatusBadge from '$lib/components/StatusBadge.svelte';

  type LogView = components['schemas']['RequestLogView'];
  type StatsView = components['schemas']['StatsView'];

  /** Dernières requêtes lues pour la liste des erreurs récentes. */
  const RECENT = 100;
  /** Fenêtre des métriques de trafic, calculées par le serveur sur tout le journal. */
  const WINDOW_HOURS = 24;

  let loading = $state(true);
  let ready = $state<boolean | null>(null);
  let providers = $state({ total: 0, file: 0 });
  let routes = $state({ total: 0, file: 0 });
  let logs = $state<LogView[]>([]);
  let stats = $state<StatsView | null>(null);

  const errors = $derived(logs.filter((log) => log.status >= 400));
  const errorRate = $derived(stats && stats.requests > 0 ? stats.errors / stats.requests : 0);

  onMount(async () => {
    const [health, providerPage, routePage, logPage, statsPage] = await Promise.all([
      fetch('/ready').then((response) => response.ok).catch(() => false),
      api.GET('/admin/v1/providers', { params: { query: { limit: 100 } } }),
      api.GET('/admin/v1/routes', { params: { query: { limit: 100 } } }),
      api.GET('/admin/v1/requests', { params: { query: { limit: RECENT } } }),
      api.GET('/admin/v1/stats', {
        params: { query: { window_seconds: WINDOW_HOURS * 3600 } }
      })
    ]);
    ready = health;
    const providerList = providerPage.data?.data ?? [];
    providers = {
      total: providerList.length,
      file: providerList.filter((provider) => provider.source === 'file').length
    };
    const routeList = routePage.data?.data ?? [];
    routes = {
      total: routeList.length,
      file: routeList.filter((route) => route.source === 'file').length
    };
    logs = logPage.data?.data ?? [];
    stats = statsPage.data ?? null;
    loading = false;
  });
</script>

<header class="page-header">
  <div>
    <h1>{$t('overview.title')}</h1>
    <p>{$t('overview.subtitle')}</p>
  </div>
  <a class="btn btn-secondary" href="/logs">{$t('overview.seeLogs')}<ArrowRight size={16} /></a>
</header>

<div class="bento summary">
  <section class="tile" aria-labelledby="tile-health">
    <h2 id="tile-health" class="kpi-label"><Activity size={14} />{$t('overview.health')}</h2>
    {#if ready === null}
      <div class="skeleton value-skeleton"></div>
    {:else}
      <p class="kpi-value health">
        <span class="status-dot" class:ok={ready} class:ko={!ready}></span>
        {ready ? $t('overview.ready') : $t('overview.notReady')}
      </p>
      <p class="kpi-foot mono">GET /ready</p>
    {/if}
  </section>

  <a class="tile" href="/providers">
    <h2 id="tile-providers" class="kpi-label"><Server size={14} />{$t('overview.providers')}</h2>
    {#if loading}
      <div class="skeleton value-skeleton"></div>
    {:else}
      <p class="kpi-value">{formatCount(providers.total, $locale)}</p>
      <p class="kpi-foot">
        {providers.file}
        {$t('overview.fromFile')} · {providers.total - providers.file}
        {$t('overview.fromConsole')}
      </p>
    {/if}
  </a>

  <a class="tile" href="/routes">
    <h2 id="tile-routes" class="kpi-label"><Route size={14} />{$t('overview.routes')}</h2>
    {#if loading}
      <div class="skeleton value-skeleton"></div>
    {:else}
      <p class="kpi-value">{formatCount(routes.total, $locale)}</p>
      <p class="kpi-foot">
        {routes.file}
        {$t('overview.fromFile')} · {routes.total - routes.file}
        {$t('overview.fromConsole')}
      </p>
    {/if}
  </a>
</div>

<h2 class="section-title" id="tile-traffic">
  <Gauge size={16} />{$t('overview.traffic')}
  <span class="muted">· {$t('overview.window', { n: WINDOW_HOURS })}</span>
</h2>

{#if loading}
  <div class="bento metrics" aria-busy="true">
    {#each [0, 1, 2, 3] as index (index)}
      <div class="tile"><div class="skeleton value-skeleton"></div></div>
    {/each}
  </div>
{:else if !stats || stats.requests === 0}
  <div class="empty">
    <span class="icon-tile"><Gauge size={18} /></span>
    <strong>{$t('overview.noTraffic')}</strong>
  </div>
{:else}
  <dl class="bento metrics" aria-labelledby="tile-traffic">
    <div class="tile">
      <dt class="kpi-label"><Hash size={14} />{$t('overview.requests')}</dt>
      <dd class="kpi-value">{formatCount(stats.requests, $locale)}</dd>
    </div>
    <div class="tile">
      <dt class="kpi-label"><CircleAlert size={14} />{$t('overview.errorRate')}</dt>
      <dd class="kpi-value" class:bad={errorRate >= 0.05}>{formatPercent(errorRate, $locale)}</dd>
    </div>
    <div class="tile">
      <dt class="kpi-label"><Timer size={14} />{$t('overview.latencyP50')}</dt>
      <dd class="kpi-value">
        {stats.latency_p50_ms == null ? '—' : formatMs(stats.latency_p50_ms, $locale)}
      </dd>
    </div>
    <div class="tile">
      <dt class="kpi-label"><Timer size={14} />{$t('overview.latencyP95')}</dt>
      <dd class="kpi-value">
        {stats.latency_p95_ms == null ? '—' : formatMs(stats.latency_p95_ms, $locale)}
      </dd>
    </div>
  </dl>
{/if}

<section class="card recent" aria-labelledby="tile-errors">
  <header class="card-header">
    <h2 id="tile-errors" class="cluster"><CircleAlert size={16} />{$t('overview.recentErrors')}</h2>
    <span class="spacer"></span>
    <a class="small" href="/logs">{$t('overview.seeLogs')}</a>
  </header>
  {#if loading}
    <div class="card-body stack" aria-busy="true">
      <div class="skeleton row-skeleton"></div>
      <div class="skeleton row-skeleton"></div>
    </div>
  {:else if errors.length === 0}
    <p class="card-body no-errors">
      <CircleCheck size={16} />{$t('overview.noErrors')}
    </p>
  {:else}
    <ul class="errors">
      {#each errors.slice(0, 5) as error (error.id)}
        <li>
          <StatusBadge status={error.status} />
          <code class="error-model">{error.model}</code>
          <span class="muted">{error.provider || '—'}</span>
          <span class="muted mono small">{formatDate(error.created_at, $locale)}</span>
        </li>
      {/each}
    </ul>
  {/if}
</section>

<style>
  @media (min-width: 64rem) {
    .summary {
      grid-template-columns: repeat(3, minmax(0, 1fr));
    }

    .metrics {
      grid-template-columns: repeat(4, minmax(0, 1fr));
    }
  }

  .metrics {
    margin: 0;
  }

  .metrics dd {
    margin: 0;
  }

  .health {
    display: flex;
    align-items: center;
    gap: var(--space-3);
  }

  .health .status-dot {
    width: 0.625rem;
    height: 0.625rem;
  }

  .bad {
    color: color-mix(in srgb, var(--color-danger) 85%, var(--color-text));
  }

  .value-skeleton {
    height: 2.25rem;
    max-width: 8rem;
    margin-top: var(--space-2);
  }

  .row-skeleton {
    height: 1.5rem;
  }

  .recent {
    margin-top: var(--space-6);
  }

  .no-errors {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    color: var(--color-muted);
  }

  .no-errors :global(svg) {
    color: var(--color-success);
  }

  .errors {
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .errors li {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-2) var(--space-3);
    padding: var(--space-3) var(--space-4);
    border-bottom: 1px solid var(--color-border);
  }

  .errors li:last-child {
    border-bottom: 0;
  }

  .error-model {
    flex: 1;
    min-width: 10rem;
  }
</style>
