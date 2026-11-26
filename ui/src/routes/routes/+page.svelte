<script lang="ts">
  import { onMount } from 'svelte';
  import Plus from '@lucide/svelte/icons/plus';
  import Trash2 from '@lucide/svelte/icons/trash-2';
  import Lock from '@lucide/svelte/icons/lock';
  import ArrowUp from '@lucide/svelte/icons/arrow-up';
  import ArrowDown from '@lucide/svelte/icons/arrow-down';
  import X from '@lucide/svelte/icons/x';
  import Save from '@lucide/svelte/icons/save';
  import Undo2 from '@lucide/svelte/icons/undo-2';
  import RouteIcon from '@lucide/svelte/icons/route';
  import CircleDot from '@lucide/svelte/icons/circle-dot';
  import { api, errorMessage } from '$lib/api';
  import type { components } from '$lib/api/schema';
  import { t } from '$lib/i18n';
  import { confirmAction, notify } from '$lib/feedback';
  import SourceBadge from '$lib/components/SourceBadge.svelte';

  type RouteView = components['schemas']['RouteView'];

  let routes = $state<RouteView[]>([]);
  /** Chaîne en cours d'édition par alias : seul éditeur d'une chaîne existante. */
  let drafts = $state<Record<string, string[]>>({});
  let newTargets = $state<Record<string, string>>({});
  let loading = $state(true);
  let loadError = $state('');
  let createError = $state('');
  let saving = $state(false);
  let form = $state({ alias: '', fallbacks: '' });

  function failure(error: unknown): string {
    return errorMessage(error, $t('error.unexpected'));
  }

  function isDirty(route: RouteView): boolean {
    const draft = drafts[route.alias] ?? [];
    return draft.join('\n') !== route.fallbacks.join('\n');
  }

  async function load(): Promise<void> {
    const { data, error } = await api.GET('/admin/v1/routes', { params: { query: { limit: 100 } } });
    if (error) {
      loadError = failure(error);
    } else {
      loadError = '';
      routes = data?.data ?? [];
      drafts = Object.fromEntries(routes.map((route) => [route.alias, [...route.fallbacks]]));
    }
    loading = false;
  }

  function splitFallbacks(value: string): string[] {
    return value
      .split('\n')
      .map((line) => line.trim())
      .filter((line) => line !== '');
  }

  async function createRoute(event: SubmitEvent): Promise<void> {
    event.preventDefault();
    saving = true;
    createError = '';
    const { error } = await api.POST('/admin/v1/routes', {
      body: { alias: form.alias.trim(), fallbacks: splitFallbacks(form.fallbacks) }
    });
    saving = false;
    if (error) {
      createError = failure(error);
      return;
    }
    notify('success', { key: 'routes.created' });
    form = { alias: '', fallbacks: '' };
    await load();
  }

  function move(alias: string, index: number, delta: number): void {
    const chain = drafts[alias];
    const target = index + delta;
    if (!chain || target < 0 || target >= chain.length) {
      return;
    }
    const next = [...chain];
    [next[index], next[target]] = [next[target] as string, next[index] as string];
    drafts[alias] = next;
  }

  function removeTarget(alias: string, index: number): void {
    drafts[alias] = (drafts[alias] ?? []).filter((_, position) => position !== index);
  }

  function addTarget(event: SubmitEvent, alias: string): void {
    event.preventDefault();
    const value = (newTargets[alias] ?? '').trim();
    if (value === '') {
      return;
    }
    drafts[alias] = [...(drafts[alias] ?? []), value];
    newTargets[alias] = '';
  }

  function discard(route: RouteView): void {
    drafts[route.alias] = [...route.fallbacks];
  }

  async function saveChain(route: RouteView): Promise<void> {
    const { error } = await api.PATCH('/admin/v1/routes/{alias}', {
      params: { path: { alias: route.alias } },
      body: { fallbacks: drafts[route.alias] ?? [] }
    });
    if (error) {
      notify('error', { error });
      return;
    }
    notify('success', { key: 'routes.saved' });
    await load();
  }

  async function deleteRoute(alias: string): Promise<void> {
    if (!(await confirmAction($t('routes.confirmDelete'), $t('routes.delete')))) {
      return;
    }
    const { error } = await api.DELETE('/admin/v1/routes/{alias}', { params: { path: { alias } } });
    if (error) {
      notify('error', { error });
    } else {
      notify('success', { key: 'routes.deleted' });
    }
    await load();
  }

  onMount(load);
</script>

<header class="page-header">
  <div>
    <h1>{$t('routes.title')}</h1>
    <p>{$t('routes.subtitle')}</p>
  </div>
</header>

<form class="card" onsubmit={createRoute}>
  <fieldset>
    <legend class="card-header create-title">
      <span class="icon-tile"><Plus size={16} /></span>{$t('routes.create')}
    </legend>
    <div class="card-body create-grid">
      <div class="field">
        <label for="route-alias">{$t('routes.alias')}</label>
        <input id="route-alias" class="mono" bind:value={form.alias} placeholder="chat" required />
      </div>
      <div class="field">
        <label for="route-fallbacks">{$t('routes.fallbacks')}</label>
        <textarea
          id="route-fallbacks"
          bind:value={form.fallbacks}
          placeholder={'openai/gpt-4o-mini\nanthropic/claude-haiku'}
        ></textarea>
      </div>
      {#if createError}
        <p class="field-error full" role="alert">{$t('error.title')} : {createError}</p>
      {/if}
    </div>
    <div class="card-footer">
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
  <RouteIcon size={16} />{$t('routes.title')}
  {#if !loading}<span class="badge">{routes.length}</span>{/if}
</h2>

{#if loading}
  <div class="stack" aria-busy="true">
    <div class="skeleton card-skeleton"></div>
    <div class="skeleton card-skeleton"></div>
  </div>
{:else if routes.length === 0}
  <div class="empty">
    <span class="icon-tile"><RouteIcon size={18} /></span>
    <strong>{$t('routes.empty')}</strong>
  </div>
{:else}
  <div class="stack">
    {#each routes as route (route.alias)}
      {@const readOnly = route.source === 'file'}
      {@const chain = drafts[route.alias] ?? []}
      <section class="card" aria-labelledby={`route-${route.alias}`}>
        <header class="card-header">
          <span class="icon-tile"><RouteIcon size={16} /></span>
          <h2 id={`route-${route.alias}`} class="mono">{route.alias}</h2>
          <SourceBadge source={route.source} />
          {#if !readOnly && isDirty(route)}
            <span class="badge badge-warning"><CircleDot size={12} />{$t('routes.unsaved')}</span>
          {/if}
          <span class="spacer"></span>
          {#if readOnly}
            <span class="muted small read-only"><Lock size={12} />{$t('source.fileHint')}</span>
          {:else}
            <button class="btn-danger" type="button" onclick={() => deleteRoute(route.alias)}>
              <Trash2 size={14} />
              {$t('routes.delete')}
            </button>
          {/if}
        </header>

        <div class="card-body">
          <h3 class="eyebrow chain-title">{$t('routes.chain')}</h3>
          <ol class="chain">
            {#each chain as target, index (`${index}-${target}`)}
              {@const slash = target.indexOf('/')}
              <li>
                <span class="step mono" aria-hidden="true">{index + 1}</span>
                <span class="target">
                  <code>
                    {#if slash > 0}<span class="muted">{target.slice(0, slash)}/</span
                      >{target.slice(slash + 1)}{:else}{target}{/if}
                  </code>
                  <span class="badge" class:badge-brand={index === 0}>
                    {index === 0 ? $t('routes.primary') : $t('routes.fallback')}
                  </span>
                </span>
                {#if !readOnly}
                  <span class="step-actions">
                    <button
                      class="btn-ghost btn-icon"
                      type="button"
                      aria-label={`${$t('routes.moveUp')} ${target}`}
                      disabled={index === 0}
                      onclick={() => move(route.alias, index, -1)}
                    >
                      <ArrowUp size={14} />
                    </button>
                    <button
                      class="btn-ghost btn-icon"
                      type="button"
                      aria-label={`${$t('routes.moveDown')} ${target}`}
                      disabled={index === chain.length - 1}
                      onclick={() => move(route.alias, index, 1)}
                    >
                      <ArrowDown size={14} />
                    </button>
                    <button
                      class="btn-ghost btn-icon"
                      type="button"
                      aria-label={`${$t('routes.removeTarget')} ${target}`}
                      onclick={() => removeTarget(route.alias, index)}
                    >
                      <X size={14} />
                    </button>
                  </span>
                {/if}
              </li>
            {/each}
          </ol>
        </div>

        {#if !readOnly}
          <div class="card-footer">
            <form class="inline spacer" onsubmit={(event) => addTarget(event, route.alias)}>
              <label for={`new-target-${route.alias}`} class="sr-only">{$t('routes.newTarget')}</label>
              <input
                id={`new-target-${route.alias}`}
                class="mono"
                bind:value={newTargets[route.alias]}
                placeholder={$t('routes.newTarget')}
              />
              <button class="btn-secondary" type="submit">
                <Plus size={14} />
                {$t('routes.addTarget')}
              </button>
            </form>
            <button
              class="btn-ghost"
              type="button"
              disabled={!isDirty(route)}
              onclick={() => discard(route)}
            >
              <Undo2 size={14} />
              {$t('routes.discard')}
            </button>
            <button type="button" disabled={!isDirty(route)} onclick={() => saveChain(route)}>
              <Save size={14} />
              {$t('routes.save')}
            </button>
          </div>
        {/if}
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

  .create-grid {
    display: grid;
    gap: var(--space-4);
  }

  @media (min-width: 48rem) {
    .create-grid {
      grid-template-columns: minmax(12rem, 1fr) 2fr;
    }

    .full {
      grid-column: 1 / -1;
    }
  }

  .card-header h2 {
    font-size: var(--text-base);
  }

  .read-only {
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
  }

  .chain-title {
    margin-bottom: var(--space-3);
  }

  .chain {
    position: relative;
    display: grid;
    gap: var(--space-2);
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .chain li {
    position: relative;
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-3);
    min-height: 2.75rem;
    padding: var(--space-1) var(--space-2) var(--space-1) var(--space-1);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-control);
    background: var(--color-surface-2);
  }

  /* Trait qui relie les étapes : on lit l'ordre de repli d'un coup d'œil. */
  .chain li + li::before {
    content: '';
    position: absolute;
    left: calc(var(--space-1) + 0.875rem);
    top: calc(-1 * var(--space-2) - 1px);
    width: 1px;
    height: var(--space-2);
    background: var(--color-border-strong);
  }

  .step {
    display: inline-grid;
    flex: none;
    place-items: center;
    width: 1.75rem;
    height: 1.75rem;
    border: 1px solid var(--color-border-strong);
    border-radius: var(--radius-pill);
    background: var(--color-surface);
    color: var(--color-muted);
    font-size: var(--text-xs);
  }

  .chain li:first-child .step {
    border-color: color-mix(in srgb, var(--color-accent) 60%, transparent);
    color: var(--color-accent-text);
  }

  .target {
    display: flex;
    flex: 1;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-2);
    min-width: 10rem;
  }

  .step-actions {
    display: flex;
    gap: 2px;
  }

  .card-footer form {
    min-width: min(100%, 18rem);
  }

  .card-skeleton {
    height: 10rem;
  }
</style>
