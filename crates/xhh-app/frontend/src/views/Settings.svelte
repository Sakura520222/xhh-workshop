<script lang="ts">
  import { onMount } from "svelte";
  import { agentGetConfig, agentSaveConfig, windowEffectGet, windowEffectSet } from "../lib/api";
  import type { WindowEffect } from "../lib/api";
  import { getTheme, setTheme, THEMES, setWindowEffectAttr } from "../lib/stores.svelte";

  type ProviderKey = "openai" | "anthropic" | "ollama";

  let loading = $state(true);
  let saving = $state(false);
  let error = $state("");
  let saved = $state(false);

  // 表单状态
  let activeProvider = $state<ProviderKey>("openai");
  // OpenAI
  let openaiKey = $state("");
  let openaiModel = $state("");
  let openaiBaseUrl = $state("");
  // Anthropic
  let anthropicKey = $state("");
  let anthropicModel = $state("");
  let anthropicBaseUrl = $state("");
  let anthropicMaxTokens = $state(0);
  // Ollama
  let ollamaModel = $state("");
  let ollamaBaseUrl = $state("");
  // 通用
  let maxLoops = $state(8);
  let temperature = $state("");

  // 窗口效果
  let windowEffect = $state<WindowEffect>("mica");
  let windowEffectSaving = $state(false);

  let currentTheme = $derived(getTheme());

  const windowEffectOptions: { key: WindowEffect; label: string; hint: string }[] = [
    { key: "mica", label: "云母", hint: "Windows 11 推荐效果" },
    { key: "acrylic", label: "亚克力", hint: "Windows 10/11 通用" },
    { key: "none", label: "无", hint: "关闭背景模糊" },
  ];

  let providers: { key: ProviderKey; label: string }[] = [
    { key: "openai", label: "OpenAI" },
    { key: "anthropic", label: "Anthropic" },
    { key: "ollama", label: "Ollama" },
  ];

  async function load() {
    loading = true;
    error = "";
    try {
      windowEffect = await windowEffectGet();
      setWindowEffectAttr(windowEffect);
      const cfg = await agentGetConfig();
      activeProvider = cfg.active_provider || "openai";
      if (cfg.openai) {
        openaiKey = cfg.openai.api_key ?? "";
        openaiModel = cfg.openai.model ?? "";
        openaiBaseUrl = cfg.openai.base_url ?? "";
      }
      if (cfg.anthropic) {
        anthropicKey = cfg.anthropic.api_key ?? "";
        anthropicModel = cfg.anthropic.model ?? "";
        anthropicBaseUrl = cfg.anthropic.base_url ?? "";
        anthropicMaxTokens = cfg.anthropic.max_tokens ?? 0;
      }
      if (cfg.ollama) {
        ollamaModel = cfg.ollama.model ?? "";
        ollamaBaseUrl = cfg.ollama.base_url ?? "";
      }
      maxLoops = cfg.max_loops ?? 8;
      temperature = cfg.temperature != null ? String(cfg.temperature) : "";
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  async function save() {
    saving = true;
    saved = false;
    error = "";
    try {
      const cfg: any = {
        active_provider: activeProvider,
        openai: {
          api_key: openaiKey,
          model: openaiModel,
          base_url: openaiBaseUrl,
        },
        anthropic: {
          api_key: anthropicKey,
          model: anthropicModel,
          base_url: anthropicBaseUrl,
          max_tokens: anthropicMaxTokens,
        },
        ollama: {
          model: ollamaModel,
          base_url: ollamaBaseUrl,
        },
        max_loops: maxLoops,
        temperature: temperature !== "" ? Number(temperature) : null,
      };
      await agentSaveConfig(cfg);
      saved = true;
      setTimeout(() => { saved = false; }, 2000);
    } catch (e) {
      error = String(e);
    } finally {
      saving = false;
    }
  }

  async function changeWindowEffect(effect: WindowEffect) {
    if (effect === windowEffect || windowEffectSaving) return;
    windowEffectSaving = true;
    error = "";
    try {
      await windowEffectSet(effect);
      setWindowEffectAttr(effect);
      windowEffect = effect;
    } catch (e) {
      error = String(e);
    } finally {
      windowEffectSaving = false;
    }
  }

  onMount(load);
</script>

<div class="settings-page">
  <div class="topbar">
    <span class="topbar-title">设置</span>
  </div>

  <section class="section">
    <h3 class="section-title">外观主题</h3>
    <div class="theme-grid">
      {#each THEMES as opt}
        <button
          type="button"
          class="theme-card"
          class:active={currentTheme === opt.key}
          style:--preview={opt.color}
          aria-pressed={currentTheme === opt.key}
          onclick={() => setTheme(opt.key)}
        >
          <span class="theme-swatch" aria-hidden="true"></span>
          <span class="theme-name">{opt.label}</span>
          {#if currentTheme === opt.key}
            <span class="theme-check" aria-hidden="true">
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round" stroke-linejoin="round"><polyline points="20 6 9 17 4 12"/></svg>
            </span>
          {/if}
        </button>
      {/each}
    </div>
  </section>

  <section class="section">
    <h3 class="section-title">窗口效果</h3>
    <div class="effect-grid">
      {#each windowEffectOptions as opt}
        <button
          type="button"
          class="effect-card"
          class:active={windowEffect === opt.key}
          aria-pressed={windowEffect === opt.key}
          disabled={windowEffectSaving}
          onclick={() => changeWindowEffect(opt.key)}
        >
          <span class="effect-name">{opt.label}</span>
          <span class="effect-hint">{opt.hint}</span>
          {#if windowEffect === opt.key}
            <span class="effect-check" aria-hidden="true">
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round" stroke-linejoin="round"><polyline points="20 6 9 17 4 12"/></svg>
            </span>
          {/if}
        </button>
      {/each}
    </div>
  </section>

  {#if loading}
    <div class="status">加载中...</div>
  {:else}
    <form onsubmit={(e) => { e.preventDefault(); save(); }} class="form">
      {#if error}
        <div class="error-msg">{error}</div>
      {/if}

      <!-- Provider 选择 -->
      <section class="section">
        <h3 class="section-title">AI Provider</h3>
        <div class="provider-tabs">
          {#each providers as p}
            <button
              type="button"
              class="tab"
              class:active={activeProvider === p.key}
              onclick={() => { activeProvider = p.key; }}
            >
              {p.label}
            </button>
          {/each}
        </div>

        {#if activeProvider === "openai"}
          <div class="field-group">
            <label class="label">API Key</label>
            <input type="password" bind:value={openaiKey} class="input" placeholder="sk-..." />
          </div>
          <div class="field-group">
            <label class="label">Model</label>
            <input type="text" bind:value={openaiModel} class="input" placeholder="gpt-4o-mini" />
          </div>
          <div class="field-group">
            <label class="label">Base URL</label>
            <input type="text" bind:value={openaiBaseUrl} class="input" placeholder="https://api.openai.com/v1" />
          </div>
        {:else if activeProvider === "anthropic"}
          <div class="field-group">
            <label class="label">API Key</label>
            <input type="password" bind:value={anthropicKey} class="input" placeholder="sk-ant-..." />
          </div>
          <div class="field-group">
            <label class="label">Model</label>
            <input type="text" bind:value={anthropicModel} class="input" placeholder="claude-haiku-4-5-20251001" />
          </div>
          <div class="field-group">
            <label class="label">Base URL</label>
            <input type="text" bind:value={anthropicBaseUrl} class="input" placeholder="https://api.anthropic.com" />
          </div>
          <div class="field-group">
            <label class="label">Max Tokens</label>
            <input type="number" bind:value={anthropicMaxTokens} class="input" placeholder="4096" min="1" />
          </div>
        {:else if activeProvider === "ollama"}
          <div class="field-group">
            <label class="label">Model</label>
            <input type="text" bind:value={ollamaModel} class="input" placeholder="qwen2.5:14b" />
          </div>
          <div class="field-group">
            <label class="label">Base URL</label>
            <input type="text" bind:value={ollamaBaseUrl} class="input" placeholder="http://localhost:11434" />
          </div>
        {/if}
      </section>

      <!-- 通用参数 -->
      <section class="section">
        <h3 class="section-title">通用参数</h3>
        <div class="field-group">
          <label class="label">最大循环轮数</label>
          <input type="number" bind:value={maxLoops} class="input" min="1" max="50" />
        </div>
        <div class="field-group">
          <label class="label">Temperature (0-2，留空用默认)</label>
          <input type="text" bind:value={temperature} class="input" placeholder="0.7" />
        </div>
      </section>

      <div class="actions">
        <button type="submit" class="save-btn" disabled={saving}>
          {saving ? "保存中..." : "保存"}
        </button>
        {#if saved}
          <span class="saved-hint">已保存</span>
        {/if}
      </div>
    </form>
  {/if}
</div>

<style>
  .settings-page {
    max-width: 640px;
    margin: 0 auto;
  }
  .topbar {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 8px;
    margin-left: -10px;
    margin-right: -10px;
    margin-bottom: 20px;
    position: sticky;
    top: 8px;
    z-index: 10;
    border-radius: 22px;
    background: linear-gradient(
      180deg,
      rgba(255, 255, 255, 0.22) 0%,
      rgba(255, 255, 255, 0.10) 100%
    );
    backdrop-filter: blur(40px) saturate(1.8) brightness(1.1);
    -webkit-backdrop-filter: blur(40px) saturate(1.8) brightness(1.1);
    border: 0.5px solid rgba(255, 255, 255, 0.35);
    box-shadow:
      inset 0 1px 0 rgba(255, 255, 255, 0.6),
      inset 0 -0.5px 0 rgba(255, 255, 255, 0.15),
      0 8px 40px rgba(0, 0, 0, 0.10),
      0 2px 12px rgba(0, 0, 0, 0.06);
  }
  .topbar-title {
    font-size: 15px;
    font-weight: 500;
  }
  .form {
    display: flex;
    flex-direction: column;
    gap: 24px;
  }
  .error-msg {
    padding: 10px 14px;
    border-radius: 10px;
    background: rgba(248, 113, 113, 0.15);
    color: #f87171;
    font-size: 13px;
    border: 0.5px solid rgba(248, 113, 113, 0.2);
  }
  .section {
    display: flex;
    flex-direction: column;
    gap: 14px;
    padding: 20px;
    border-radius: var(--radius);
    background: var(--glass-bg);
    backdrop-filter: var(--glass-blur);
    -webkit-backdrop-filter: var(--glass-blur);
    border: 0.5px solid var(--glass-border);
    box-shadow: var(--elevation-1);
  }
  .section + .section {
    margin-top: 24px;
  }
  .section-title {
    font-size: 14px;
    font-weight: 500;
    margin: 0;
    padding-bottom: 8px;
    border-bottom: 0.5px solid var(--glass-border);
  }
  .theme-grid {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 10px;
  }
  .theme-card {
    position: relative;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
    padding: 14px 8px 12px;
    border-radius: var(--radius-sm);
    background: rgba(255, 255, 255, 0.04);
    border: 0.5px solid var(--glass-border);
    transition: all var(--duration-fast) var(--ease-out);
  }
  .theme-card:hover {
    background: rgba(255, 255, 255, 0.07);
    transform: translateY(-1px);
  }
  .theme-card.active {
    border-color: var(--preview);
    box-shadow: 0 0 0 1px var(--preview), 0 4px 18px color-mix(in srgb, var(--preview) 28%, transparent);
  }
  .theme-swatch {
    width: 28px;
    height: 28px;
    border-radius: 50%;
    background: linear-gradient(135deg, var(--preview), color-mix(in srgb, var(--preview) 55%, #000));
    box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.4), 0 2px 10px color-mix(in srgb, var(--preview) 42%, transparent);
  }
  .theme-name {
    font-size: 12px;
    color: var(--text-secondary);
  }
  .theme-card.active .theme-name {
    color: var(--text-strong);
    font-weight: 500;
  }
  .theme-check {
    position: absolute;
    top: 6px;
    right: 6px;
    display: grid;
    place-items: center;
    width: 18px;
    height: 18px;
    border-radius: 50%;
    background: var(--preview);
    color: #fff;
  }
  .effect-grid {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 10px;
  }
  .effect-card {
    position: relative;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
    padding: 14px 8px 12px;
    border-radius: var(--radius-sm);
    background: rgba(255, 255, 255, 0.04);
    border: 0.5px solid var(--glass-border);
    transition: all var(--duration-fast) var(--ease-out);
  }
  .effect-card:hover:not(:disabled) {
    background: rgba(255, 255, 255, 0.07);
    transform: translateY(-1px);
  }
  .effect-card:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
  .effect-card.active {
    border-color: var(--accent);
    box-shadow: 0 0 0 1px var(--accent), 0 4px 18px color-mix(in srgb, var(--accent) 28%, transparent);
  }
  .effect-name {
    font-size: 13px;
    font-weight: 500;
    color: var(--text-strong);
  }
  .effect-hint {
    font-size: 11px;
    color: var(--text-secondary);
  }
  .effect-check {
    position: absolute;
    top: 6px;
    right: 6px;
    display: grid;
    place-items: center;
    width: 18px;
    height: 18px;
    border-radius: 50%;
    background: var(--accent);
    color: #fff;
  }
  .provider-tabs {
    display: flex;
    gap: 8px;
  }
  .tab {
    flex: 1;
    padding: 8px 0;
    border-radius: 10px;
    background: transparent;
    color: var(--text-secondary);
    font-size: 13px;
    font-weight: 500;
    text-align: center;
    border: 0.5px solid var(--glass-border);
    transition: all var(--duration-fast) var(--ease-out);
  }
  .tab:hover {
    background: rgba(255, 255, 255, 0.06);
  }
  .tab.active {
    background: var(--accent);
    color: white;
    border-color: var(--accent);
    box-shadow: 0 2px 8px color-mix(in srgb, var(--accent) 30%, transparent);
  }
  .field-group {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .field-group.half {
    flex: 1;
  }
  .row {
    display: flex;
    gap: 14px;
  }
  .label {
    font-size: 12px;
    color: var(--text-secondary);
  }
  .input {
    padding: 9px 12px;
    border-radius: 10px;
    background: rgba(255, 255, 255, 0.04);
    color: var(--text);
    border: 0.5px solid var(--glass-border);
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
  .actions {
    display: flex;
    align-items: center;
    gap: 14px;
  }
  .save-btn {
    padding: 10px 32px;
    border-radius: 10px;
    background: var(--accent);
    color: white;
    font-size: 14px;
    font-weight: 500;
    box-shadow: 0 2px 8px color-mix(in srgb, var(--accent) 30%, transparent);
    transition: all var(--duration-fast) var(--ease-out);
  }
  .save-btn:hover:not(:disabled) {
    background: var(--accent-hover);
  }
  .save-btn:disabled {
    opacity: 0.5;
  }
  .saved-hint {
    font-size: 13px;
    color: #4ade80;
  }
  .status {
    text-align: center;
    padding: 40px 0;
    color: var(--text-secondary);
  }
</style>
