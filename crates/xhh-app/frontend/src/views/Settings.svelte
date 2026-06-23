<script lang="ts">
  import { onMount } from "svelte";
  import {
    agentGetConfig,
    agentSaveConfig,
    windowEffectGet,
    windowEffectSet,
    cacheGetConfig,
    cacheSaveConfig,
    cacheStats as loadCacheStats,
    cacheClear,
  } from "../lib/api";
  import type { WindowEffect, CacheStats } from "../lib/api";
  import { getTheme, setTheme, THEMES, setWindowEffectAttr, getColorMode, setColorMode } from "../lib/stores.svelte";

  type ProviderKey = "openai" | "anthropic" | "ollama";
  type Tab = "appearance" | "model" | "storage";

  let loading = $state(true);
  let saving = $state(false);
  let error = $state("");
  let saved = $state(false);

  // 顶层类目
  let activeTab = $state<Tab>("appearance");

  // 表单状态
  let activeProvider = $state<ProviderKey>("openai");
  // OpenAI
  let openaiKey = $state("");
  let openaiModel = $state("");
  let openaiBaseUrl = $state("");
  let openaiTimeout = $state(0);
  // Anthropic
  let anthropicKey = $state("");
  let anthropicModel = $state("");
  let anthropicBaseUrl = $state("");
  let anthropicMaxTokens = $state(0);
  let anthropicTimeout = $state(0);
  // Ollama
  let ollamaModel = $state("");
  let ollamaBaseUrl = $state("");
  let ollamaTimeout = $state(0);
  // 通用
  let maxLoops = $state(8);
  let temperature = $state("");

  // 窗口效果
  let windowEffect = $state<WindowEffect>("mica");
  let windowEffectSaving = $state(false);

  // 内容缓存
  let cacheEnabled = $state(true);
  let cacheMaxMb = $state(200);
  let cacheStats = $state<CacheStats | null>(null);
  let cacheSaving = $state(false);
  let cacheClearing = $state(false);
  let cacheSaved = $state(false);

  let currentTheme = $derived(getTheme());
  let currentMode = $derived(getColorMode());

  const tabs: { key: Tab; label: string }[] = [
    { key: "appearance", label: "外观" },
    { key: "model", label: "模型" },
    { key: "storage", label: "存储" },
  ];

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
        openaiTimeout = cfg.openai.timeout_secs ?? 0;
      }
      if (cfg.anthropic) {
        anthropicKey = cfg.anthropic.api_key ?? "";
        anthropicModel = cfg.anthropic.model ?? "";
        anthropicBaseUrl = cfg.anthropic.base_url ?? "";
        anthropicMaxTokens = cfg.anthropic.max_tokens ?? 0;
        anthropicTimeout = cfg.anthropic.timeout_secs ?? 0;
      }
      if (cfg.ollama) {
        ollamaModel = cfg.ollama.model ?? "";
        ollamaBaseUrl = cfg.ollama.base_url ?? "";
        ollamaTimeout = cfg.ollama.timeout_secs ?? 0;
      }
      maxLoops = cfg.max_loops ?? 8;
      temperature = cfg.temperature != null ? String(cfg.temperature) : "";
      try {
        const cc = await cacheGetConfig();
        cacheEnabled = cc.enabled;
        cacheMaxMb = cc.max_bytes ? Math.max(1, Math.round(cc.max_bytes / 1048576)) : 200;
      } catch {
        // 缓存配置可选，读取失败不阻塞页面
      }
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
    await refreshCacheStats();
  }

  function fmtBytes(n: number): string {
    if (n < 1024) return `${n} B`;
    if (n < 1048576) return `${(n / 1024).toFixed(1)} KB`;
    return `${(n / 1048576).toFixed(1)} MB`;
  }

  async function refreshCacheStats() {
    try {
      cacheStats = await loadCacheStats();
    } catch {
      cacheStats = null;
    }
  }

  async function saveCache() {
    cacheSaving = true;
    cacheSaved = false;
    error = "";
    try {
      await cacheSaveConfig(cacheEnabled, Math.round(cacheMaxMb * 1048576));
      cacheSaved = true;
      setTimeout(() => { cacheSaved = false; }, 2000);
      await refreshCacheStats();
    } catch (e) {
      error = String(e);
    } finally {
      cacheSaving = false;
    }
  }

  async function clearCache() {
    cacheClearing = true;
    error = "";
    try {
      await cacheClear();
      await refreshCacheStats();
    } catch (e) {
      error = String(e);
    } finally {
      cacheClearing = false;
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
          timeout_secs: openaiTimeout,
        },
        anthropic: {
          api_key: anthropicKey,
          model: anthropicModel,
          base_url: anthropicBaseUrl,
          max_tokens: anthropicMaxTokens,
          timeout_secs: anthropicTimeout,
        },
        ollama: {
          model: ollamaModel,
          base_url: ollamaBaseUrl,
          timeout_secs: ollamaTimeout,
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

  <div class="nav-tabs" role="tablist">
    {#each tabs as t}
      <button
        type="button"
        role="tab"
        class="nav-tab"
        class:active={activeTab === t.key}
        aria-selected={activeTab === t.key}
        onclick={() => { activeTab = t.key; }}
      >
        {t.label}
      </button>
    {/each}
  </div>

  {#if activeTab === "appearance"}
    <!-- 外观：纯本地操作，即时生效 -->
    <section class="section">
      <h3 class="section-title">主题色</h3>
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

      <div class="mode-row">
        <span class="mode-label">显示模式</span>
        <div class="mode-segment" role="group" aria-label="明暗模式">
          <button
            type="button"
            class="seg"
            class:active={currentMode === "dark"}
            aria-pressed={currentMode === "dark"}
            onclick={() => setColorMode("dark")}
          >深色</button>
          <button
            type="button"
            class="seg"
            class:active={currentMode === "light"}
            aria-pressed={currentMode === "light"}
            onclick={() => setColorMode("light")}
          >浅色</button>
        </div>
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
  {:else if loading}
    <div class="status">加载中...</div>
  {:else if activeTab === "storage"}
    {#if error}<div class="error-msg">{error}</div>{/if}
    <section class="section">
      <h3 class="section-title">内容缓存</h3>
      <p class="cache-desc">
        缓存帖子正文与图片到本地，加速详情浏览与 AI 图片分析，并支持离线查看正文。仅首屏请求会被缓存。
      </p>
      <label class="toggle-row" for="cache-enabled">
        <input id="cache-enabled" type="checkbox" bind:checked={cacheEnabled} />
        <span>启用内容缓存</span>
      </label>
      <div class="field-group">
        <label class="label" for="cache-max-mb">磁盘配额（MB）</label>
        <input
          id="cache-max-mb"
          type="number"
          bind:value={cacheMaxMb}
          class="input"
          min="50"
          max="4096"
        />
      </div>
      <div class="cache-stats" aria-live="polite">
        {#if cacheStats}
          <span>已用 {fmtBytes(cacheStats.used_bytes)} / {cacheMaxMb} MB</span>
          <span class="cache-break">帖子 {cacheStats.posts.count} 条 · 图片 {cacheStats.images.count} 张</span>
        {:else}
          <span>统计加载中...</span>
        {/if}
      </div>
      <div class="cache-actions">
        <button type="button" class="cache-save-btn" onclick={saveCache} disabled={cacheSaving}>
          {cacheSaving ? "保存中..." : "保存缓存设置"}
        </button>
        <button type="button" class="cache-clear-btn" onclick={clearCache} disabled={cacheClearing}>
          {cacheClearing ? "清理中..." : "清空缓存"}
        </button>
        {#if cacheSaved}<span class="saved-hint">已保存</span>{/if}
      </div>
    </section>
  {:else if activeTab === "model"}
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
            <label class="label" for="openai-key">API Key</label>
            <input id="openai-key" type="password" bind:value={openaiKey} class="input" placeholder="sk-..." />
          </div>
          <div class="field-group">
            <label class="label" for="openai-model">Model</label>
            <input id="openai-model" type="text" bind:value={openaiModel} class="input" placeholder="gpt-4o-mini" />
          </div>
          <div class="field-group">
            <label class="label" for="openai-base-url">Base URL</label>
            <input id="openai-base-url" type="text" bind:value={openaiBaseUrl} class="input" placeholder="https://api.openai.com/v1" />
          </div>
          <div class="field-group">
            <label class="label" for="openai-timeout">超时（秒，0 用默认 120）</label>
            <input id="openai-timeout" type="number" bind:value={openaiTimeout} class="input" placeholder="120" min="0" />
          </div>
        {:else if activeProvider === "anthropic"}
          <div class="field-group">
            <label class="label" for="anthropic-key">API Key</label>
            <input id="anthropic-key" type="password" bind:value={anthropicKey} class="input" placeholder="sk-ant-..." />
          </div>
          <div class="field-group">
            <label class="label" for="anthropic-model">Model</label>
            <input id="anthropic-model" type="text" bind:value={anthropicModel} class="input" placeholder="claude-haiku-4-5-20251001" />
          </div>
          <div class="field-group">
            <label class="label" for="anthropic-base-url">Base URL</label>
            <input id="anthropic-base-url" type="text" bind:value={anthropicBaseUrl} class="input" placeholder="https://api.anthropic.com" />
          </div>
          <div class="field-group">
            <label class="label" for="anthropic-max-tokens">Max Tokens</label>
            <input id="anthropic-max-tokens" type="number" bind:value={anthropicMaxTokens} class="input" placeholder="4096" min="1" />
          </div>
          <div class="field-group">
            <label class="label" for="anthropic-timeout">超时（秒，0 用默认 120）</label>
            <input id="anthropic-timeout" type="number" bind:value={anthropicTimeout} class="input" placeholder="120" min="0" />
          </div>
        {:else if activeProvider === "ollama"}
          <div class="field-group">
            <label class="label" for="ollama-model">Model</label>
            <input id="ollama-model" type="text" bind:value={ollamaModel} class="input" placeholder="qwen2.5:14b" />
          </div>
          <div class="field-group">
            <label class="label" for="ollama-base-url">Base URL</label>
            <input id="ollama-base-url" type="text" bind:value={ollamaBaseUrl} class="input" placeholder="http://localhost:11434" />
          </div>
          <div class="field-group">
            <label class="label" for="ollama-timeout">超时（秒，0 用默认 600）</label>
            <input id="ollama-timeout" type="number" bind:value={ollamaTimeout} class="input" placeholder="600" min="0" />
          </div>
        {/if}
      </section>

      <!-- 通用参数 -->
      <section class="section">
        <h3 class="section-title">通用参数</h3>
        <div class="field-group">
          <label class="label" for="max-loops">最大循环轮数</label>
          <input id="max-loops" type="number" bind:value={maxLoops} class="input" min="1" max="500" />
        </div>
        <div class="field-group">
          <label class="label" for="temperature">Temperature (0-2，留空用默认)</label>
          <input id="temperature" type="text" bind:value={temperature} class="input" placeholder="0.7" />
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
  /* 顶层类目导航 */
  .nav-tabs {
    display: flex;
    gap: 6px;
    margin-bottom: 24px;
    padding: 4px;
    border-radius: var(--radius);
    background: var(--glass-bg);
    backdrop-filter: var(--glass-blur);
    -webkit-backdrop-filter: var(--glass-blur);
    border: 0.5px solid var(--glass-border);
    box-shadow: var(--elevation-1);
  }
  .nav-tab {
    flex: 1;
    padding: 10px 0;
    border-radius: var(--radius-sm);
    color: var(--text-secondary);
    font-size: 14px;
    font-weight: 500;
    transition: all var(--duration-fast) var(--ease-out);
  }
  .nav-tab:hover {
    color: var(--text-strong);
    background: var(--fill);
  }
  .nav-tab.active {
    background: var(--accent);
    color: #fff;
    box-shadow: 0 2px 12px color-mix(in srgb, var(--accent) 32%, transparent);
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
    color: var(--danger);
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
    background: var(--fill);
    border: 0.5px solid var(--glass-border);
    transition: all var(--duration-fast) var(--ease-out);
  }
  .theme-card:hover {
    background: var(--fill-strong);
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
  .mode-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding-top: 4px;
  }
  .mode-label {
    font-size: 13px;
    color: var(--text-secondary);
  }
  .mode-segment {
    display: flex;
    gap: 4px;
    padding: 3px;
    border-radius: 12px;
    background: var(--fill);
    border: 0.5px solid var(--glass-border);
  }
  .seg {
    padding: 6px 18px;
    border-radius: 9px;
    font-size: 13px;
    font-weight: 500;
    color: var(--text-secondary);
    transition: all var(--duration-fast) var(--ease-out);
  }
  .seg:hover {
    color: var(--text-strong);
  }
  .seg.active {
    background: var(--accent);
    color: #fff;
    box-shadow: 0 2px 8px color-mix(in srgb, var(--accent) 30%, transparent);
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
    background: var(--fill);
    border: 0.5px solid var(--glass-border);
    transition: all var(--duration-fast) var(--ease-out);
  }
  .effect-card:hover:not(:disabled) {
    background: var(--fill-strong);
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
    background: var(--fill-hover);
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
  .label {
    font-size: 12px;
    color: var(--text-secondary);
  }
  .input {
    padding: 9px 12px;
    border-radius: 10px;
    background: var(--fill);
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
    color: var(--success-fg);
  }
  .status {
    text-align: center;
    padding: 40px 0;
    color: var(--text-secondary);
  }
  .cache-desc {
    margin: 0;
    font-size: 12px;
    line-height: 1.6;
    color: var(--text-secondary);
  }
  .toggle-row {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 13px;
    color: var(--text);
    cursor: pointer;
  }
  .toggle-row input {
    width: 16px;
    height: 16px;
    accent-color: var(--accent);
    cursor: pointer;
  }
  .cache-stats {
    display: flex;
    flex-wrap: wrap;
    gap: 6px 14px;
    font-size: 12px;
    color: var(--text-secondary);
    padding: 10px 12px;
    border-radius: 10px;
    background: var(--fill);
    border: 0.5px solid var(--glass-border);
  }
  .cache-break {
    color: var(--text-secondary);
    opacity: 0.85;
  }
  .cache-actions {
    display: flex;
    align-items: center;
    gap: 10px;
    flex-wrap: wrap;
  }
  .cache-save-btn {
    padding: 8px 18px;
    border-radius: 10px;
    background: var(--accent);
    color: white;
    font-size: 13px;
    font-weight: 500;
    box-shadow: 0 2px 8px color-mix(in srgb, var(--accent) 30%, transparent);
    transition: all var(--duration-fast) var(--ease-out);
  }
  .cache-save-btn:hover:not(:disabled) {
    background: var(--accent-hover);
  }
  .cache-save-btn:disabled {
    opacity: 0.5;
  }
  .cache-clear-btn {
    padding: 8px 18px;
    border-radius: 10px;
    background: var(--fill-hover);
    color: var(--text);
    font-size: 13px;
    border: 0.5px solid var(--glass-border);
    transition: all var(--duration-fast) var(--ease-out);
  }
  .cache-clear-btn:hover:not(:disabled) {
    background: rgba(248, 113, 113, 0.15);
    border-color: rgba(248, 113, 113, 0.4);
    color: var(--danger);
  }
  .cache-clear-btn:disabled {
    opacity: 0.5;
  }
</style>
