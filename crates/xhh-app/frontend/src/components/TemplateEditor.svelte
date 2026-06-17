<script lang="ts">
  import type { AgentTemplate } from "../lib/api";

  type Props = {
    template: AgentTemplate | null;
    onCommit: (title: string, content: string, id: string | null) => Promise<void> | void;
    onDelete: (id: string) => Promise<void> | void;
    onClose: () => void;
  };

  let { template, onCommit, onDelete, onClose }: Props = $props();

  let title = $state("");
  let content = $state("");
  let saving = $state(false);
  let deleting = $state(false);
  let error = $state("");
  let titleInput: HTMLInputElement | null = $state(null);

  $effect(() => {
    title = template?.title ?? "";
    content = template?.content ?? "";
    error = "";
    if (template) {
      queueMicrotask(() => titleInput?.focus());
    } else {
      queueMicrotask(() => titleInput?.focus());
    }
  });

  const isExisting = $derived(!!template);

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Escape") {
      e.preventDefault();
      onClose();
    }
  }

  async function submit() {
    if (saving) return;
    if (!content.trim()) {
      error = "内容不能为空";
      return;
    }
    saving = true;
    error = "";
    try {
      await onCommit(title.trim(), content.trim(), template?.id ?? null);
    } catch (e) {
      error = String(e);
      return;
    } finally {
      saving = false;
    }
  }

  async function remove() {
    if (!template) return;
    if (deleting) return;
    deleting = true;
    try {
      await onDelete(template.id);
    } catch (e) {
      error = String(e);
      return;
    } finally {
      deleting = false;
    }
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<div class="backdrop" role="presentation">
  <div class="dialog" role="dialog" aria-modal="true" aria-labelledby="tpl-editor-title">
    <form onsubmit={(e) => { e.preventDefault(); submit(); }}>
    <h2 id="tpl-editor-title">{isExisting ? "编辑模板" : "新建模板"}</h2>

    <label class="field">
      <span class="label">标题（留空将用内容前 20 字）</span>
      <input
        bind:this={titleInput}
        bind:value={title}
        type="text"
        class="input"
        placeholder="例：分析原神板块热帖"
        maxlength="50"
      />
    </label>

    <label class="field">
      <span class="label">内容</span>
      <textarea
        bind:value={content}
        class="input textarea"
        placeholder="例：请用 search_community 找到「原神」板块，然后用 community_feeds 拉取最近热帖…"
        rows="6"
      ></textarea>
    </label>

    {#if error}
      <div class="error-msg" role="alert">{error}</div>
    {/if}

    <div class="actions">
      {#if isExisting}
        <button
          type="button"
          class="btn btn-danger"
          onclick={remove}
          disabled={deleting || saving}
        >
          {deleting ? "删除中…" : "删除"}
        </button>
      {/if}
      <span class="spacer"></span>
      <button type="button" class="btn btn-secondary" onclick={onClose} disabled={saving || deleting}>
        取消
      </button>
      <button type="submit" class="btn btn-primary" disabled={saving || deleting}>
        {saving ? "保存中…" : "保存"}
      </button>
    </div>
    </form>
  </div>
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 1100;
    display: grid;
    place-items: center;
    padding: 24px;
    background: var(--scrim);
    backdrop-filter: blur(8px);
    -webkit-backdrop-filter: blur(8px);
    animation: backdrop-in 160ms var(--ease-out);
  }

  .dialog {
    width: min(520px, 100%);
    border-radius: 22px;
    background: var(--glass-bg);
    border: 1px solid var(--glass-border);
    box-shadow: 0 24px 60px rgba(0, 0, 0, 0.42);
    overflow: hidden;
    animation: dialog-in 220ms var(--ease-out);
  }

  .dialog > form {
    padding: 26px;
    display: flex;
    flex-direction: column;
    gap: 14px;
  }

  h2 {
    margin: 0;
    font-size: 18px;
    color: var(--text-strong);
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .label {
    font-size: 12px;
    color: var(--text-secondary);
  }

  .input {
    padding: 10px 12px;
    border-radius: 12px;
    background: var(--fill-hover);
    color: var(--text);
    border: 1px solid var(--glass-border);
    font-size: 13px;
    outline: none;
    transition: border-color var(--duration-fast) var(--ease-out), box-shadow var(--duration-fast) var(--ease-out);
  }

  .input:focus {
    border-color: color-mix(in srgb, var(--accent-hover) 42%, transparent);
    box-shadow: 0 0 0 3px color-mix(in srgb, var(--accent) 14%, transparent);
  }

  .textarea {
    min-height: 132px;
    resize: vertical;
    line-height: 1.6;
    font-family: inherit;
  }

  .error-msg {
    padding: 8px 12px;
    border-radius: 10px;
    background: rgba(248, 113, 113, 0.12);
    color: var(--danger-fg);
    font-size: 12px;
    border: 1px solid rgba(248, 113, 113, 0.22);
  }

  .actions {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-top: 4px;
  }

  .spacer {
    flex: 1;
  }

  .btn {
    min-height: 38px;
    padding: 0 18px;
    border-radius: 11px;
    font-size: 13px;
    font-weight: 700;
    transition: filter var(--duration-fast) var(--ease-out), background var(--duration-fast) var(--ease-out);
  }

  .btn-primary {
    background: linear-gradient(135deg, var(--accent), var(--accent-strong));
    color: white;
    box-shadow: 0 10px 22px color-mix(in srgb, var(--accent-strong) 24%, transparent);
  }

  .btn-primary:hover:not(:disabled) {
    filter: brightness(1.08);
  }

  .btn-secondary {
    background: rgba(148, 163, 184, 0.14);
    color: var(--text);
    border: 1px solid rgba(148, 163, 184, 0.18);
  }

  .btn-secondary:hover:not(:disabled) {
    background: rgba(148, 163, 184, 0.22);
  }

  .btn-danger {
    background: rgba(248, 113, 113, 0.16);
    color: var(--danger-fg);
    border: 1px solid rgba(248, 113, 113, 0.28);
  }

  .btn-danger:hover:not(:disabled) {
    background: rgba(248, 113, 113, 0.24);
  }

  .btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  @keyframes backdrop-in {
    from { opacity: 0; }
  }

  @keyframes dialog-in {
    from { opacity: 0; transform: translateY(8px) scale(0.98); }
  }
</style>
