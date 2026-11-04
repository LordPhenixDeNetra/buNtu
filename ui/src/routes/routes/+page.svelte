<script lang="ts">
  import { onMount } from 'svelte';
  import { api, errorMessage } from '$lib/api';
  import type { components } from '$lib/api/schema';
  import { t } from '$lib/i18n';

  type RouteView = components['schemas']['RouteView'];

  let routes: RouteView[] = [];
  let drafts: Record<string, string> = {};
  let loading = true;
  let error = '';
  let notice = '';
  let saving = false;
  let form = { alias: '', fallbacks: '' };

  async function load(): Promise<void> {
    loading = true;
    error = '';
    const { data, error: requestError } = await api.GET('/admin/v1/routes');
    if (requestError) {
      error = errorMessage(requestError);
    } else {
      routes = data?.data ?? [];
      drafts = Object.fromEntries(
        routes.map((route) => [route.alias, route.fallbacks.join('\n')])
      );
    }
    loading = false;
  }

  function splitFallbacks(value: string): string[] {
    return value
      .split('\n')
      .map((line) => line.trim())
      .filter((line) => line !== '');
  }

  async function createRoute(): Promise<void> {
    saving = true;
    error = '';
    notice = '';
    const fallbacks = splitFallbacks(form.fallbacks);
    const { error: requestError } = await api.POST('/admin/v1/routes', {
      body: { alias: form.alias.trim(), fallbacks }
    });
    saving = false;
    if (requestError) {
      error = errorMessage(requestError);
      return;
    }
    notice = 'Route créée.';
    form = { alias: '', fallbacks: '' };
    await load();
  }

  async function saveFallbacks(route: RouteView): Promise<void> {
    const fallbacks = splitFallbacks(drafts[route.alias] ?? '');
    const { error: requestError } = await api.PATCH('/admin/v1/routes/{alias}', {
      params: { path: { alias: route.alias } },
      body: { fallbacks }
    });
    if (requestError) {
      error = errorMessage(requestError);
    } else {
      notice = 'Chaîne de repli enregistrée.';
    }
    await load();
  }

  async function deleteRoute(alias: string): Promise<void> {
    if (!confirm($t('routes.confirmDelete'))) {
      return;
    }
    const { error: requestError } = await api.DELETE('/admin/v1/routes/{alias}', {
      params: { path: { alias } }
    });
    if (requestError) {
      error = errorMessage(requestError);
    }
    await load();
  }

  function move(route: RouteView, index: number, delta: number): void {
    const target = index + delta;
    if (target < 0 || target >= route.fallbacks.length) {
      return;
    }
    const fallbacks = [...route.fallbacks];
    const current = fallbacks[index];
    const other = fallbacks[target];
    if (current === undefined || other === undefined) {
      return;
    }
    fallbacks[index] = other;
    fallbacks[target] = current;
    route.fallbacks = fallbacks;
    drafts[route.alias] = fallbacks.join('\n');
    drafts = { ...drafts };
    routes = [...routes];
  }

  onMount(load);
</script>

<h1>{$t('routes.title')}</h1>
<p style="color: var(--muted);">{$t('routes.subtitle')}</p>

{#if notice}
  <p role="status" style="color: var(--accent);">{notice}</p>
{/if}
{#if error}
  <p role="alert" style="color: var(--danger);">{$t('error.title')} : {error}</p>
{/if}

<fieldset>
  <legend>{$t('routes.create')}</legend>
  <div style="display: grid; grid-template-columns: repeat(auto-fit, minmax(16rem, 1fr)); gap: 0.9rem;">
    <div>
      <label for="route-alias">{$t('routes.alias')}</label>
      <input id="route-alias" bind:value={form.alias} placeholder="chat" />
    </div>
    <div>
      <label for="route-fallbacks">{$t('routes.fallbacks')}</label>
      <textarea id="route-fallbacks" bind:value={form.fallbacks} placeholder="mock/model-a
mock/model-b"></textarea>
    </div>
  </div>
  <button type="button" on:click={createRoute} disabled={saving}>{$t('providers.save')}</button>
</fieldset>

{#if loading}
  <p>{$t('loading')}</p>
{:else if routes.length === 0}
  <p>{$t('routes.empty')}</p>
{:else}
  <div style="display: grid; gap: 1rem;">
    {#each routes as route (route.alias)}
      <section style="border: 1px solid var(--border); border-radius: 8px; padding: 1rem; background: var(--panel);">
        <header style="display: flex; align-items: baseline; gap: 0.75rem;">
          <h2 style="margin: 0; font-size: 1.1rem;">{route.alias}</h2>
          <span style="margin-left: auto;">
            <button class="danger" type="button" on:click={() => deleteRoute(route.alias)}>
              {$t('routes.delete')}
            </button>
          </span>
        </header>
        <ol style="padding-left: 1.25rem; margin: 0.75rem 0;">
          {#each route.fallbacks as fallback, index (index)}
            <li style="display: flex; gap: 0.5rem; align-items: center; padding: 0.2rem 0;">
              <code style="flex: 1;">{fallback}</code>
              <button class="secondary" type="button" on:click={() => move(route, index, -1)} disabled={index === 0}>
                ↑ <span class="sr-only" style="position: absolute; left: -999px;">{$t('routes.moveUp')}</span>
              </button>
              <button
                class="secondary"
                type="button"
                on:click={() => move(route, index, 1)}
                disabled={index === route.fallbacks.length - 1}
              >
                ↓ <span class="sr-only" style="position: absolute; left: -999px;">{$t('routes.moveDown')}</span>
              </button>
            </li>
          {/each}
        </ol>
        <div>
          <label for={`fallbacks-${route.alias}`}>{$t('routes.fallbacks')}</label>
          <textarea id={`fallbacks-${route.alias}`} bind:value={drafts[route.alias]}></textarea>
        </div>
        <button type="button" on:click={() => saveFallbacks(route)}>{$t('routes.save')}</button>
      </section>
    {/each}
  </div>
{/if}
