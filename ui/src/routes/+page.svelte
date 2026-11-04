<script lang="ts">
  import { t } from '$lib/i18n';
</script>

<h1>buNtu</h1>
<p style="color: var(--muted); max-width: 46rem;">
  {$t('providers.subtitle')}
</p>
<div style="display: grid; grid-template-columns: repeat(auto-fit, minmax(16rem, 1fr)); gap: 1rem;">
  <a
    href="/providers"
    style="display: block; padding: 1.25rem; border: 1px solid var(--border); border-radius: 8px; background: var(--panel); text-decoration: none; color: var(--text);"
  >
    <strong>{$t('nav.providers')}</strong>
  </a>
  <a
    href="/routes"
    style="display: block; padding: 1.25rem; border: 1px solid var(--border); border-radius: 8px; background: var(--panel); text-decoration: none; color: var(--text);"
  >
    <strong>{$t('nav.routes')}</strong>
  </a>
</div>
