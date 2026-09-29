<script lang="ts">
  let {
    id,
    label,
    placeholder = "",
    options = [],
    fetching = false,
    error = "",
    value = $bindable(""),
    onfetch,
  }: {
    id: string;
    label: string;
    placeholder?: string;
    options?: string[];
    fetching?: boolean;
    error?: string;
    value?: string;
    onfetch: () => void;
  } = $props();

  let root: HTMLDivElement | undefined = $state();
  let listEl: HTMLDivElement | undefined = $state();
  let open = $state(false);
  let activeIdx = $state(-1);
  // null 表示未输入过滤词，下拉展示全部
  let filterQ = $state<string | null>(null);

  let filtered = $derived.by(() => {
    const q = (filterQ ?? "").trim().toLowerCase();
    if (!q) return options;
    return options.filter((m) => m.toLowerCase().includes(q));
  });

  $effect(() => {
    if (!options.length) open = false;
  });

  $effect(() => {
    if (activeIdx < 0 || !open || !listEl) return;
    listEl
      .querySelector(`[data-idx="${activeIdx}"]`)
      ?.scrollIntoView({ block: "nearest" });
  });

  function openList() {
    if (!options.length) return;
    filterQ = null;
    activeIdx = -1;
    open = true;
  }

  function pick(m: string) {
    value = m;
    open = false;
    activeIdx = -1;
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.key === "ArrowDown" || e.key === "ArrowUp") {
      if (!open) {
        openList();
      } else {
        e.preventDefault();
        const next = activeIdx + (e.key === "ArrowDown" ? 1 : -1);
        activeIdx = Math.max(0, Math.min(next, filtered.length - 1));
      }
    } else if (e.key === "Enter" && open && activeIdx >= 0 && filtered[activeIdx]) {
      e.preventDefault();
      pick(filtered[activeIdx]);
    } else if (e.key === "Escape") {
      open = false;
    }
  }

  function onWindowClick(e: MouseEvent) {
    if (open && root && !root.contains(e.target as Node)) open = false;
  }
</script>

<svelte:window onclick={onWindowClick} />

<div
  class="field-group"
  bind:this={root}
  onfocusout={(e) => {
    if (open && root && !root.contains(e.relatedTarget as Node)) open = false;
  }}
>
  <label class="label" for={id}>{label}</label>
  <div class="model-row">
    <div class="model-input-wrap">
      <input
        {id}
        type="text"
        class="input"
        bind:value
        {placeholder}
        autocomplete="off"
        role="combobox"
        aria-expanded={open}
        aria-controls="{id}-list"
        onfocus={openList}
        oninput={() => {
          filterQ = value;
          activeIdx = -1;
          open = options.length > 0;
        }}
        onkeydown={onKeydown}
      />
      {#if open}
        <div class="model-list" role="listbox" id="{id}-list" bind:this={listEl}>
          {#each filtered as m, i (m)}
            <button
              type="button"
              role="option"
              aria-selected={i === activeIdx}
              data-idx={i}
              class="model-option"
              class:active={i === activeIdx}
              onmousedown={(e) => e.preventDefault()}
              onclick={() => pick(m)}
            >
              {m}
            </button>
          {:else}
            <span class="model-empty">无匹配模型</span>
          {/each}
        </div>
      {/if}
    </div>
    <button type="button" class="fetch-btn" onclick={onfetch} disabled={fetching}>
      {fetching ? "获取中..." : "获取列表"}
    </button>
  </div>
  {#if error}
    <span class="model-error">{error}</span>
  {:else if options.length}
    <span class="field-hint">已获取 {options.length} 个模型，点击输入框选择</span>
  {/if}
</div>

<style>
  .field-group {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .label {
    font-size: 12px;
    color: var(--text-secondary);
  }
  .input {
    width: 100%;
    padding: 9px 12px;
    border-radius: 10px;
    background: var(--fill);
    color: var(--text);
    border: 0.5px solid var(--border);
    font-size: 13px;
    outline: none;
    transition: all var(--duration-fast) var(--ease-out);
  }
  .input:focus {
    border-color: var(--accent);
    box-shadow: 0 0 0 3px color-mix(in srgb, var(--accent) 12%, transparent);
  }
  .input::placeholder {
    color: var(--text-secondary);
    opacity: 0.5;
  }
  .model-row {
    display: flex;
    gap: 8px;
  }
  .model-input-wrap {
    position: relative;
    flex: 1;
  }
  .model-list {
    position: absolute;
    top: calc(100% + 4px);
    left: 0;
    right: 0;
    z-index: 30;
    max-height: 240px;
    overflow-y: auto;
    padding: 4px;
    border-radius: 10px;
    background: var(--panel-bg);
    border: 0.5px solid var(--border);
    box-shadow: var(--elevation-2);
  }
  .model-option {
    display: block;
    width: 100%;
    padding: 8px 10px;
    border-radius: 8px;
    font-size: 13px;
    color: var(--text);
    text-align: left;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .model-option:hover,
  .model-option.active {
    background: var(--fill-hover);
  }
  .model-empty {
    display: block;
    padding: 8px 10px;
    font-size: 12px;
    color: var(--text-secondary);
  }
  .fetch-btn {
    padding: 0 16px;
    border-radius: 10px;
    background: var(--fill-hover);
    color: var(--text);
    font-size: 13px;
    font-weight: 500;
    border: 0.5px solid var(--border);
    white-space: nowrap;
    transition: all var(--duration-fast) var(--ease-out);
  }
  .fetch-btn:hover:not(:disabled) {
    background: var(--accent);
    border-color: var(--accent);
    color: #fff;
  }
  .fetch-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
  .field-hint {
    font-size: 12px;
    color: var(--text-secondary);
    opacity: 0.85;
  }
  .model-error {
    font-size: 12px;
    color: var(--danger);
  }
</style>
