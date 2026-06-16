<script lang="ts">
  import { getToasts, dismiss } from "../lib/toast.svelte";
  import { setView } from "../lib/stores.svelte";

  let toasts = $derived(getToasts());
</script>

<div class="toast-host" role="region" aria-label="消息提示" aria-live="polite">
  {#each toasts as t (t.id)}
    <div class={`toast ${t.kind}`} role="status">
      <div class="toast-body">
        <div class="toast-title">{t.title}</div>
        {#if t.desc}
          <div class="toast-desc">{t.desc}</div>
        {/if}
      </div>
      <div class="toast-actions">
        {#if t.kind === "info"}
          <button class="toast-link" onclick={() => { dismiss(t.id); setView("notifications"); }}>查看</button>
        {/if}
        <button class="toast-close" aria-label="关闭" onclick={() => dismiss(t.id)}>x</button>
      </div>
    </div>
  {/each}
</div>

<style>
  .toast-host {
    position: fixed;
    top: calc(var(--titlebar-height, 32px) + 12px);
    right: 16px;
    z-index: 9999;
    display: flex;
    flex-direction: column;
    gap: 10px;
    width: min(360px, calc(100vw - 32px));
    pointer-events: none;
  }
  .toast {
    pointer-events: auto;
    display: flex;
    align-items: flex-start;
    gap: 10px;
    padding: 12px 14px;
    border-radius: var(--radius);
    background: var(--surface-strong);
    border: 1px solid var(--glass-border);
    box-shadow: var(--elevation-2);
    backdrop-filter: var(--glass-blur);
    -webkit-backdrop-filter: var(--glass-blur);
    animation: toast-in var(--duration-normal) var(--ease-out);
  }
  .toast.info { border-left: 3px solid var(--accent); }
  .toast.success { border-left: 3px solid #34d399; }
  .toast.error { border-left: 3px solid var(--danger); }
  .toast-body { flex: 1; min-width: 0; }
  .toast-title { font-size: 13px; font-weight: 650; color: var(--text-strong); }
  .toast-desc {
    margin-top: 3px;
    font-size: 12px;
    line-height: 1.45;
    color: var(--text-secondary);
    white-space: pre-wrap;
    word-break: break-word;
    overflow: hidden;
    display: -webkit-box;
    -webkit-line-clamp: 3;
    line-clamp: 3;
    -webkit-box-orient: vertical;
  }
  .toast-actions { display: flex; align-items: center; gap: 6px; flex-shrink: 0; }
  .toast-link {
    padding: 4px 10px;
    border-radius: 10px;
    background: var(--accent-soft);
    border: 0.5px solid color-mix(in srgb, var(--accent) 40%, transparent);
    color: var(--accent-hover);
    font-size: 12px;
    font-weight: 600;
  }
  .toast-link:hover { background: color-mix(in srgb, var(--accent) 26%, transparent); }
  .toast-close {
    width: 22px;
    height: 22px;
    border-radius: 8px;
    background: transparent;
    color: var(--text-muted);
    font-size: 13px;
    line-height: 1;
  }
  .toast-close:hover { background: rgba(148, 163, 184, 0.16); color: var(--text-secondary); }
  @keyframes toast-in {
    from { opacity: 0; transform: translateX(16px); }
    to { opacity: 1; transform: translateX(0); }
  }
</style>
