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
    notify('success', $t('routes.created'));
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
      notify('error', failure(error));
      return;
    }
    notify('success', $t('routes.saved'));
    await load();
  }

  async function deleteRoute(alias: string): Promise<void> {
    if (!(await confirmAction($t('routes.confirmDelete'), $t('routes.delete')))) {
      return;
    }
    const { error } = await api.DELETE('/admin/v1/routes/{alias}', { params: { path: { alias } } });
    if (error) {
      notify('error', failure(error));
    } else {
      notify('success', $t('routes.deleted'));
    }
    await load();
  }

  onMount(load);
</script>

<h1>{$t('routes.title')}</h1>
<p class="muted lead">{$t('routes.subtitle')}</p>

<form onsubmit={createRoute}>
  <fieldset>
    <legend>{$t('routes.create')}</legend>
    <div class="grid">
      <div class="field">
        <label for="route-alias">{$t('routes.alias')}</label>
        <input id="route-alias" bind:value={form.alias} placeholder="chat" required />
      </div>
      <div class="field">
        <label for="route-fallbacks">{$t('routes.fallbacks')}</label>
        <textarea
          id="route-fallbacks"
          bind:value={form.fallbacks}
          placeholder={'openai/gpt-4o-mini\nanthropic/claude-haiku'}
        ></textarea>
      </div>
    </div>
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
{:else if routes.length === 0}
  <p class="muted">{$t('routes.empty')}</p>
{:else}
  <div class="stack">
    {#each routes as route (route.alias)}
      {@const readOnly = route.source === 'file'}
      {@const chain = drafts[route.alias] ?? []}
      <section class="card" aria-labelledby={`route-${route.alias}`}>
        <header class="card-header">
          <h2 id={`route-${route.alias}`} class="mono">{route.alias}</h2>
          <SourceBadge source={route.source} />
          {#if !readOnly && isDirty(route)}
            <span class="badge badge-warning">{$t('routes.unsaved')}</span>
          {/if}
          <span class="spacer"></span>
          {#if readOnly}
            <span class="muted small read-only"><Lock size={14} />{$t('source.fileHint')}</span>
          {:else}
            <button class="btn-danger" type="button" onclick={() => deleteRoute(route.alias)}>
              <Trash2 size={16} />
              {$t('routes.delete')}
            </button>
          {/if}
        </header>

        <h3 class="label">{$t('routes.chain')}</h3>
        <ol class="chain">
          {#each chain as target, index (`${index}-${target}`)}
            <li>
              <span class="step mono" aria-hidden="true">{index + 1}</span>
              <code class="target">{target}</code>
              {#if !readOnly}
                <button
                  class="btn-ghost btn-icon"
                  type="button"
                  aria-label={`${$t('routes.moveUp')} ${target}`}
                  disabled={index === 0}
                  onclick={() => move(route.alias, index, -1)}
                >
                  <ArrowUp size={16} />
                </button>
                <button
                  class="btn-ghost btn-icon"
                  type="button"
                  aria-label={`${$t('routes.moveDown')} ${target}`}
                  disabled={index === chain.length - 1}
                  onclick={() => move(route.alias, index, 1)}
                >
                  <ArrowDown size={16} />
                </button>
                <button
                  class="btn-ghost btn-icon"
                  type="button"
                  aria-label={`${$t('routes.removeTarget')} ${target}`}
                  onclick={() => removeTarget(route.alias, index)}
                >
                  <X size={16} />
                </button>
              {/if}
            </li>
          {/each}
        </ol>

        {#if !readOnly}
          <form class="inline" onsubmit={(event) => addTarget(event, route.alias)}>
            <label for={`new-target-${route.alias}`} class="sr-only">{$t('routes.newTarget')}</label>
            <input
              id={`new-target-${route.alias}`}
              class="mono"
              bind:value={newTargets[route.alias]}
              placeholder={$t('routes.newTarget')}
            />
            <button class="btn-secondary" type="submit">
              <Plus size={16} />
              {$t('routes.addTarget')}
            </button>
          </form>
          <div class="actions">
            <button
              class="btn-secondary"
              type="button"
              disabled={!isDirty(route)}
              onclick={() => discard(route)}
            >
              <Undo2 size={16} />
              {$t('routes.discard')}
            </button>
            <button type="button" disabled={!isDirty(route)} onclick={() => saveChain(route)}>
              <Save size={16} />
              {$t('routes.save')}
            </button>
          </div>
        {/if}
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
    grid-template-columns: repeat(auto-fit, minmax(16rem, 1fr));
    gap: var(--space-4);
    margin-bottom: var(--space-3);
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

  .label {
    margin: var(--space-4) 0 var(--space-2);
    font-size: var(--text-sm);
    font-weight: 600;
  }

  .chain {
    display: grid;
    gap: var(--space-1);
    margin: 0 0 var(--space-3);
    padding: 0;
    list-style: none;
  }

  .chain li {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-1) var(--space-2);
    border-radius: var(--radius-control);
    background: var(--color-surface-2);
  }

  .step {
    display: inline-grid;
    place-items: center;
    width: 1.5rem;
    height: 1.5rem;
    border-radius: var(--radius-pill);
    background: var(--color-surface);
    color: var(--color-muted);
    font-size: var(--text-xs);
  }

  .target {
    flex: 1;
  }

  .inline {
    display: flex;
    gap: var(--space-2);
  }

  .actions {
    display: flex;
    justify-content: flex-end;
    gap: var(--space-2);
    margin-top: var(--space-3);
  }

  .small {
    margin: 0;
    font-size: var(--text-sm);
  }

  .card-skeleton {
    height: 8rem;
  }
</style>
