<script lang="ts">
  import { onMount } from 'svelte';
  import Activity from '@lucide/svelte/icons/activity';
  import Server from '@lucide/svelte/icons/server';
  import Route from '@lucide/svelte/icons/route';
  import Gauge from '@lucide/svelte/icons/gauge';
  import CircleCheck from '@lucide/svelte/icons/circle-check';
  import CircleAlert from '@lucide/svelte/icons/circle-alert';
  import ChevronRight from '@lucide/svelte/icons/chevron-right';
  import { api } from '$lib/api';
  import type { components } from '$lib/api/schema';
  import { locale, t } from '$lib/i18n';
  import { formatCount, formatDate, formatMs, formatPercent, percentile } from '$lib/format';
  import StatusBadge from '$lib/components/StatusBadge.svelte';

  type LogView = components['schemas']['RequestLogView'];

  /** Taille de l'échantillon de trafic affiché (dernières requêtes). */
  const SAMPLE = 100;

  let loading = $state(true);
  let ready = $state<boolean | null>(null);
  let providers = $state({ total: 0, file: 0 });
  let routes = $state({ total: 0, file: 0 });
  let logs = $state<LogView[]>([]);

  const errors = $derived(logs.filter((log) => log.status >= 400));
  const latencies = $derived(logs.map((log) => log.latency_ms));
  const p50 = $derived(percentile(latencies, 0.5));
  const p95 = $derived(percentile(latencies, 0.95));

  onMount(async () => {
    const [health, providerPage, routePage, logPage] = await Promise.all([
      fetch('/ready').then((response) => response.ok).catch(() => false),
      api.GET('/admin/v1/providers', { params: { query: { limit: 100 } } }),
      api.GET('/admin/v1/routes', { params: { query: { limit: 100 } } }),
      api.GET('/admin/v1/requests', { params: { query: { limit: SAMPLE } } })
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
    loading = false;
  });
</script>

<h1>{$t('overview.title')}</h1>
<p class="muted lead">{$t('overview.subtitle')}</p>

<div class="bento overview">
  <section class="tile" aria-labelledby="tile-health">
    <h2 id="tile-health" class="tile-title"><Activity size={16} />{$t('overview.health')}</h2>
    {#if ready === null}
      <div class="skeleton metric-skeleton"></div>
    {:else if ready}
      <p class="metric metric-ok"><CircleCheck size={28} />{$t('overview.ready')}</p>
    {:else}
      <p class="metric metric-ko"><CircleAlert size={28} />{$t('overview.notReady')}</p>
    {/if}
  </section>

  <a class="tile" href="/providers">
    <h2 id="tile-providers" class="tile-title"><Server size={16} />{$t('overview.providers')}</h2>
    {#if loading}
      <div class="skeleton metric-skeleton"></div>
    {:else}
      <p class="metric">{formatCount(providers.total, $locale)}</p>
      <p class="muted small">
        {providers.file}
        {$t('overview.fromFile')} · {providers.total - providers.file}
        {$t('overview.fromConsole')}
      </p>
    {/if}
  </a>

  <a class="tile" href="/routes">
    <h2 id="tile-routes" class="tile-title"><Route size={16} />{$t('overview.routes')}</h2>
    {#if loading}
      <div class="skeleton metric-skeleton"></div>
    {:else}
      <p class="metric">{formatCount(routes.total, $locale)}</p>
      <p class="muted small">
        {routes.file}
        {$t('overview.fromFile')} · {routes.total - routes.file}
        {$t('overview.fromConsole')}
      </p>
    {/if}
  </a>

  <section class="tile tile-wide" aria-labelledby="tile-traffic">
    <h2 id="tile-traffic" class="tile-title"><Gauge size={16} />{$t('overview.traffic')}</h2>
    {#if loading}
      <div class="skeleton metric-skeleton"></div>
    {:else if logs.length === 0}
      <p class="muted">{$t('overview.noTraffic')}</p>
    {:else}
      <dl class="stats">
        <div>
          <dt>{$t('overview.requests')}</dt>
          <dd class="mono">{formatCount(logs.length, $locale)}</dd>
        </div>
        <div>
          <dt>{$t('overview.errorRate')}</dt>
          <dd class="mono">{formatPercent(errors.length / logs.length, $locale)}</dd>
        </div>
        <div>
          <dt>{$t('overview.latencyP50')}</dt>
          <dd class="mono">{p50 === null ? '—' : formatMs(p50, $locale)}</dd>
        </div>
        <div>
          <dt>{$t('overview.latencyP95')}</dt>
          <dd class="mono">{p95 === null ? '—' : formatMs(p95, $locale)}</dd>
        </div>
      </dl>
      <p class="muted small">{$t('overview.sample', { n: logs.length })}</p>
    {/if}
  </section>

  <section class="tile tile-wide" aria-labelledby="tile-errors">
    <h2 id="tile-errors" class="tile-title">
      <CircleAlert size={16} />{$t('overview.recentErrors')}
    </h2>
    {#if loading}
      <div class="skeleton metric-skeleton"></div>
    {:else if errors.length === 0}
      <p class="muted">{$t('overview.noErrors')}</p>
    {:else}
      <ul class="errors">
        {#each errors.slice(0, 5) as error (error.id)}
          <li>
            <StatusBadge status={error.status} />
            <span class="mono error-model">{error.model}</span>
            <span class="muted small">{formatDate(error.created_at, $locale)}</span>
          </li>
        {/each}
      </ul>
    {/if}
    <a class="see-logs" href="/logs">{$t('overview.seeLogs')}<ChevronRight size={16} /></a>
  </section>
</div>

<style>
  .lead {
    margin: 0 0 var(--space-5);
  }

  /* Ordinateur : 3 tuiles compactes puis 2 tuiles larges, sans colonne vide. */
  @media (min-width: 64rem) {
    .overview {
      grid-template-columns: repeat(6, minmax(0, 1fr));
    }

    .overview > :global(.tile) {
      grid-column: span 2;
    }

    .overview > :global(.tile-wide) {
      grid-column: span 3;
    }
  }

  .tile-title {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    margin: 0 0 var(--space-3);
    color: var(--color-muted);
    font-size: var(--text-sm);
    font-weight: 600;
  }

  .metric {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    margin: 0;
    font-size: var(--text-2xl);
    font-weight: 600;
    letter-spacing: -0.02em;
  }

  .metric-ok :global(svg) {
    color: var(--color-success);
  }

  .metric-ko :global(svg) {
    color: var(--color-danger);
  }

  .metric-skeleton {
    height: 2.5rem;
    max-width: 10rem;
  }

  .small {
    margin: var(--space-2) 0 0;
    font-size: var(--text-sm);
  }

  .stats {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(6.5rem, 1fr));
    gap: var(--space-4);
    margin: 0;
  }

  .stats dt {
    color: var(--color-muted);
    font-size: var(--text-xs);
    font-weight: 600;
    letter-spacing: 0.04em;
    text-transform: uppercase;
  }

  .stats dd {
    margin: var(--space-1) 0 0;
    font-size: var(--text-xl);
    font-weight: 600;
  }

  .errors {
    display: grid;
    gap: var(--space-2);
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .errors li {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-2);
  }

  .error-model {
    flex: 1;
  }

  .see-logs {
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
    margin-top: var(--space-3);
    font-size: var(--text-sm);
    font-weight: 500;
  }
</style>
