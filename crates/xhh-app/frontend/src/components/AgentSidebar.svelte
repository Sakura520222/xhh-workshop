<script lang="ts">
  import type { SessionMeta, AgentTemplate } from "../lib/api";

  type Props = {
    sessions: SessionMeta[];
    activeId: string;
    busy: boolean;
    templates: AgentTemplate[];
    templatesCollapsed: boolean;
    onToggleTemplates: () => void;
    onCreateSession: () => void;
    onSelectSession: (id: string) => void;
    onRenameSession: (id: string, title: string) => void;
    onDeleteSession: (id: string) => void;
    onInsertTemplate: (template: AgentTemplate) => void;
    onEditTemplate: (template: AgentTemplate) => void;
    onCreateTemplate: () => void;
    onDeleteTemplate: (id: string) => void;
  };

  let {
    sessions,
    activeId,
    busy,
    templates,
    templatesCollapsed,
    onToggleTemplates,
    onCreateSession,
    onSelectSession,
    onRenameSession,
    onDeleteSession,
    onInsertTemplate,
    onEditTemplate,
    onCreateTemplate,
    onDeleteTemplate,
  }: Props = $props();

  type Editing = { kind: "session"; id: string; value: string } | null;
  let editing: Editing = $state(null);
  let confirmDelete: { kind: "session" | "template"; id: string } | null = $state(null);

  function startRenameSession(s: SessionMeta) {
    if (busy && s.id !== activeId) return;
    editing = { kind: "session", id: s.id, value: s.title };
  }

  function commitRename() {
    if (!editing || editing.kind !== "session") return;
    const value = editing.value.trim();
    const id = editing.id;
    editing = null;
    if (!value) return;
    const current = sessions.find((s) => s.id === id);
    if (current && current.title !== value) onRenameSession(id, value);
  }

  function cancelRename() {
    editing = null;
  }

  function handleRenameKeydown(e: KeyboardEvent) {
    if (e.key === "Enter") {
      e.preventDefault();
      commitRename();
    } else if (e.key === "Escape") {
      e.preventDefault();
      cancelRename();
    }
  }

  function selectSession(id: string) {
    if (busy) return;
    if (id === activeId) return;
    onSelectSession(id);
  }

  function fmtRelative(ts: number): string {
    if (!ts) return "";
    const diff = Math.floor((Date.now() / 1000 - ts));
    if (diff < 60) return "刚刚";
    if (diff < 3600) return `${Math.floor(diff / 60)} 分钟前`;
    if (diff < 86400) return `${Math.floor(diff / 3600)} 小时前`;
    if (diff < 604800) return `${Math.floor(diff / 86400)} 天前`;
    const d = new Date(ts * 1000);
    return `${d.getMonth() + 1}/${d.getDate()}`;
  }

  function requestDelete(kind: "session" | "template", id: string) {
    confirmDelete = { kind, id };
  }

  function confirmDeleteYes() {
    if (!confirmDelete) return;
    const { kind, id } = confirmDelete;
    confirmDelete = null;
    if (kind === "session") onDeleteSession(id);
    else onDeleteTemplate(id);
  }

  function confirmDeleteNo() {
    confirmDelete = null;
  }
</script>

<aside class="agent-sidebar" aria-label="Agent 会话与模板">
  <div class="sidebar-top">
    <button class="new-session" onclick={onCreateSession} disabled={busy}>
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M12 5v14"/><path d="M5 12h14"/></svg>
      新建会话
    </button>
  </div>

  <div class="sessions" role="list">
    {#each sessions as s (s.id)}
      {@const isActive = s.id === activeId}
      {@const isDisabled = busy && !isActive}
      {@const isEditingThis = editing?.kind === "session" && editing.id === s.id}
      <div
        class="session-item"
        class:active={isActive}
        class:disabled={isDisabled}
        role="listitem"
      >
        {#if isEditingThis}
          <input
            class="rename-input"
            value={editing?.value ?? ""}
            oninput={(e: Event) => {
              if (editing?.kind === "session") editing.value = (e.target as HTMLInputElement).value;
            }}
            onkeydown={handleRenameKeydown}
            onblur={commitRename}
          />
        {:else}
          <button
            type="button"
            class="session-main"
            disabled={isDisabled}
            onclick={() => selectSession(s.id)}
            ondblclick={() => startRenameSession(s)}
            aria-current={isActive ? "true" : undefined}
            title={s.title}
          >
            <span class="session-title">{s.title || "(未命名)"}</span>
            <span class="session-meta">
              <span>{fmtRelative(s.updated_at)}</span>
              <span class="dot">·</span>
              <span>{s.message_count} 条</span>
            </span>
          </button>
          <div class="session-actions">
            <button
              type="button"
              class="icon-action"
              title="重命名"
              aria-label="重命名会话"
              disabled={isDisabled}
              onclick={() => startRenameSession(s)}
            >
              <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M12 20h9"/><path d="M16.5 3.5a2.121 2.121 0 1 1 3 3L7 19l-4 1 1-4 12.5-12.5z"/></svg>
            </button>
            <button
              type="button"
              class="icon-action danger"
              title="删除"
              aria-label="删除会话"
              disabled={isDisabled}
              onclick={() => requestDelete("session", s.id)}
            >
              <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M3 6h18"/><path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6"/><path d="M8 6V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"/></svg>
            </button>
          </div>
        {/if}
      </div>
    {/each}
    {#if sessions.length === 0}
      <div class="empty-hint">暂无会话</div>
    {/if}
  </div>

  <div class="templates-section" class:collapsed={templatesCollapsed}>
    <button
      type="button"
      class="templates-head"
      onclick={onToggleTemplates}
      aria-expanded={!templatesCollapsed}
    >
      <svg class="caret" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><polyline points="6 9 12 15 18 9"/></svg>
      <span>模板</span>
      <span class="count">{templates.length}</span>
      <span class="spacer"></span>
      <span
        class="add-template"
        role="button"
        tabindex="0"
        aria-label="新建模板"
        onclick={(e: MouseEvent) => { e.stopPropagation(); onCreateTemplate(); }}
        onkeydown={(e: KeyboardEvent) => { if (e.key === "Enter" || e.key === " ") { e.preventDefault(); e.stopPropagation(); onCreateTemplate(); } }}
      >
        <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M12 5v14"/><path d="M5 12h14"/></svg>
      </span>
    </button>

    {#if !templatesCollapsed}
      <div class="templates-list" role="list">
        {#each templates as t (t.id)}
          <div class="template-item" role="listitem">
            <button
              type="button"
              class="template-main"
              onclick={() => onInsertTemplate(t)}
              ondblclick={() => onEditTemplate(t)}
              title={t.content}
            >
              <span class="template-title">{t.title || "(未命名)"}</span>
              {#if t.is_builtin}<span class="builtin-tag">内置</span>{/if}
            </button>
            <div class="template-actions">
              <button
                type="button"
                class="icon-action"
                title="编辑"
                aria-label="编辑模板"
                onclick={() => onEditTemplate(t)}
              >
                <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M12 20h9"/><path d="M16.5 3.5a2.121 2.121 0 1 1 3 3L7 19l-4 1 1-4 12.5-12.5z"/></svg>
              </button>
              <button
                type="button"
                class="icon-action danger"
                title="删除"
                aria-label="删除模板"
                onclick={() => requestDelete("template", t.id)}
              >
                <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M3 6h18"/><path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6"/><path d="M8 6V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"/></svg>
              </button>
            </div>
          </div>
        {/each}
        {#if templates.length === 0}
          <div class="empty-hint">点击「+」创建第一个模板</div>
        {/if}
      </div>
    {/if}
  </div>
</aside>

{#if confirmDelete}
  <div class="confirm-backdrop" role="presentation">
    <div class="confirm-dialog" role="alertdialog" aria-modal="true" aria-labelledby="confirm-del-title">
      <h3 id="confirm-del-title">确认删除？</h3>
      <p>该操作不可撤销。</p>
      <div class="confirm-actions">
        <button type="button" class="btn-secondary" onclick={confirmDeleteNo}>取消</button>
        <button type="button" class="btn-danger" onclick={confirmDeleteYes}>删除</button>
      </div>
    </div>
  </div>
{/if}

<style>
  .agent-sidebar {
    display: flex;
    flex-direction: column;
    width: 220px;
    flex-shrink: 0;
    height: 100%;
    min-height: 0;
    padding: 12px 10px 10px;
    border-right: 1px solid rgba(148, 163, 184, 0.16);
    background: color-mix(in srgb, var(--bg-soft) 70%, transparent);
    backdrop-filter: blur(18px) saturate(1.3);
    -webkit-backdrop-filter: blur(18px) saturate(1.3);
  }

  .sidebar-top {
    margin-bottom: 10px;
  }

  .new-session {
    width: 100%;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    min-height: 36px;
    padding: 0 12px;
    border-radius: 12px;
    background: linear-gradient(135deg, var(--accent), var(--accent-strong));
    color: white;
    font-size: 13px;
    font-weight: 700;
    box-shadow: 0 8px 20px color-mix(in srgb, var(--accent-strong) 24%, transparent);
    transition: filter var(--duration-fast) var(--ease-out), transform var(--duration-fast) var(--ease-out);
  }
  .new-session:hover:not(:disabled) {
    filter: brightness(1.08);
    transform: translateY(-1px);
  }
  .new-session:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .sessions {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding-right: 2px;
  }

  .session-item {
    position: relative;
    display: flex;
    align-items: center;
    border-radius: 12px;
    transition: background var(--duration-fast) var(--ease-out);
  }

  .session-item:hover {
    background: rgba(148, 163, 184, 0.08);
  }

  .session-item.active {
    background: linear-gradient(135deg, color-mix(in srgb, var(--accent) 22%, transparent), color-mix(in srgb, var(--accent-warm) 10%, transparent));
    box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--accent-hover) 20%, transparent);
  }

  .session-item.active::before {
    content: "";
    position: absolute;
    left: -3px;
    top: 50%;
    transform: translateY(-50%);
    width: 3px;
    height: 60%;
    border-radius: 999px;
    background: linear-gradient(180deg, var(--accent), var(--accent-warm));
  }

  .session-main {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
    align-items: flex-start;
    padding: 8px 10px;
    background: transparent;
    border: 0;
    text-align: left;
    color: var(--text-secondary);
    transition: color var(--duration-fast) var(--ease-out);
  }

  .session-item.active .session-main {
    color: var(--text-strong);
  }

  .session-main:hover:not(:disabled) {
    color: var(--text-strong);
  }

  .session-main:disabled {
    cursor: not-allowed;
    opacity: 0.45;
  }

  .session-title {
    width: 100%;
    font-size: 13px;
    font-weight: 650;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .session-meta {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    font-size: 11px;
    color: var(--text-muted);
  }

  .session-meta .dot {
    opacity: 0.6;
  }

  .session-actions,
  .template-actions {
    display: flex;
    align-items: center;
    gap: 2px;
    padding-right: 6px;
    opacity: 0;
    transition: opacity var(--duration-fast) var(--ease-out);
  }

  .session-item:hover .session-actions,
  .template-item:hover .template-actions {
    opacity: 1;
  }

  .icon-action {
    display: inline-grid;
    place-items: center;
    width: 26px;
    height: 26px;
    border-radius: 8px;
    background: transparent;
    color: var(--text-muted);
    transition: background var(--duration-fast) var(--ease-out), color var(--duration-fast) var(--ease-out);
  }

  .icon-action:hover:not(:disabled) {
    background: rgba(148, 163, 184, 0.18);
    color: var(--text-strong);
  }

  .icon-action.danger:hover:not(:disabled) {
    background: rgba(248, 113, 113, 0.16);
    color: var(--danger-fg);
  }

  .icon-action:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .rename-input {
    flex: 1;
    min-width: 0;
    margin: 4px 8px;
    padding: 6px 8px;
    border-radius: 8px;
    background: var(--fill-strong);
    color: var(--text-strong);
    border: 1px solid color-mix(in srgb, var(--accent-hover) 40%, transparent);
    font-size: 13px;
    outline: none;
  }

  .templates-section {
    border-top: 1px solid rgba(148, 163, 184, 0.14);
    padding-top: 8px;
    margin-top: 8px;
    display: flex;
    flex-direction: column;
    min-height: 0;
  }

  .templates-section.collapsed .templates-list {
    display: none;
  }

  .templates-head {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 10px;
    border-radius: 10px;
    background: transparent;
    color: var(--text-muted);
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 0.04em;
    text-transform: uppercase;
  }

  .templates-head:hover {
    background: rgba(148, 163, 184, 0.1);
    color: var(--text-secondary);
  }

  .caret {
    transition: transform var(--duration-fast) var(--ease-out);
  }

  .templates-section.collapsed .caret {
    transform: rotate(-90deg);
  }

  .count {
    padding: 0 6px;
    border-radius: 999px;
    background: rgba(148, 163, 184, 0.18);
    color: var(--text-secondary);
    font-size: 10px;
    font-weight: 700;
    letter-spacing: 0;
  }

  .spacer {
    flex: 1;
  }

  .add-template {
    display: inline-grid;
    place-items: center;
    width: 22px;
    height: 22px;
    border-radius: 6px;
    color: var(--text-muted);
    transition: background var(--duration-fast) var(--ease-out), color var(--duration-fast) var(--ease-out);
  }

  .add-template:hover {
    background: rgba(148, 163, 184, 0.18);
    color: var(--text-strong);
  }

  .templates-list {
    margin-top: 6px;
    display: flex;
    flex-direction: column;
    gap: 3px;
    max-height: 240px;
    overflow-y: auto;
    padding-right: 2px;
  }

  .template-item {
    display: flex;
    align-items: center;
    border-radius: 10px;
    transition: background var(--duration-fast) var(--ease-out);
  }

  .template-item:hover {
    background: rgba(148, 163, 184, 0.08);
  }

  .template-main {
    flex: 1;
    min-width: 0;
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 6px 10px;
    background: transparent;
    border: 0;
    text-align: left;
    color: var(--text-secondary);
    font-size: 12.5px;
    transition: color var(--duration-fast) var(--ease-out);
  }

  .template-main:hover {
    color: var(--text-strong);
  }

  .template-title {
    flex: 1;
    min-width: 0;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .builtin-tag {
    flex-shrink: 0;
    padding: 1px 6px;
    border-radius: 999px;
    background: var(--accent-soft);
    color: var(--on-accent-soft);
    border: 1px solid color-mix(in srgb, var(--accent-hover) 20%, transparent);
    font-size: 9px;
    font-weight: 700;
    letter-spacing: 0.04em;
  }

  .empty-hint {
    padding: 10px 12px;
    color: var(--text-muted);
    font-size: 11px;
    text-align: center;
  }

  .confirm-backdrop {
    position: fixed;
    inset: 0;
    z-index: 1100;
    display: grid;
    place-items: center;
    background: var(--scrim);
    backdrop-filter: blur(8px);
    -webkit-backdrop-filter: blur(8px);
  }

  .confirm-dialog {
    width: min(360px, 92vw);
    padding: 22px;
    border-radius: 18px;
    background: var(--glass-bg);
    border: 1px solid var(--glass-border);
    box-shadow: 0 24px 60px rgba(0, 0, 0, 0.4);
  }

  .confirm-dialog h3 {
    margin: 0 0 8px;
    font-size: 16px;
    color: var(--text-strong);
  }

  .confirm-dialog p {
    margin: 0 0 18px;
    color: var(--text-secondary);
    font-size: 13px;
  }

  .confirm-actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }

  .btn-secondary,
  .btn-danger {
    min-height: 36px;
    padding: 0 16px;
    border-radius: 10px;
    font-size: 13px;
    font-weight: 700;
    transition: filter var(--duration-fast) var(--ease-out), background var(--duration-fast) var(--ease-out);
  }

  .btn-secondary {
    background: rgba(148, 163, 184, 0.14);
    color: var(--text);
    border: 1px solid rgba(148, 163, 184, 0.18);
  }

  .btn-secondary:hover {
    background: rgba(148, 163, 184, 0.22);
  }

  .btn-danger {
    background: linear-gradient(135deg, #ef4444, #f97316);
    color: white;
    box-shadow: 0 8px 18px rgba(239, 68, 68, 0.28);
  }

  .btn-danger:hover {
    filter: brightness(1.08);
  }
</style>
