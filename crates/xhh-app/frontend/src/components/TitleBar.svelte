<script lang="ts">
  let { onMinimize, onMaximize, onClose }: { onMinimize: () => void; onMaximize: () => void; onClose: () => void } = $props();
</script>

<header class="titlebar" data-tauri-drag-region>
  <div class="left" data-tauri-drag-region>
    <img class="brand-mark" src="data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAACAAAAAgCAYAAABzenr0AAABDklEQVR4nO1XUQrDIAw1wVP0c+yvbODuf4YWVvo39tlrbGRMcE6jxm4yukCpqHnPxJdaldq6Qaiz6463TxEuy/mFE75FHFuIljiP/SnYb+ahGAtbRO/yoSTiUKR+XyxLvmEueS5gqQ9wW1BCyllMGyTEaAbWIk9hYWxAomgJlk455pactDSRHWUWVkqUvYDRiSQUlZsVervt0CJSeOBXAScYl4DmHS7XR3va797GcjDYKuDMkhMxPdSWVg2qxqYlTpTGidmC6gyYeWAFZQVnt8AXZi4eGfsp5gBr51aL0DepCME2/ofRTxxGhvnaxcaqDyPzBCjJSIkPSH5K1/orFpfhmgatLyaotn43VK3tDtnCqranOWRYAAAAAElFTkSuQmCC" alt="" />
    <div class="brand-copy" data-tauri-drag-region>
      <span class="logo">黑盒工坊</span>
    </div>
  </div>
  <div class="controls">
    <button class="ctrl-btn" onclick={onMinimize} title="最小化" aria-label="最小化窗口">
      <svg width="12" height="12" viewBox="0 0 12 12" aria-hidden="true"><rect y="5" width="12" height="2" rx="1" fill="currentColor"/></svg>
    </button>
    <button class="ctrl-btn" onclick={onMaximize} title="最大化" aria-label="最大化窗口">
      <svg width="12" height="12" viewBox="0 0 12 12" aria-hidden="true"><rect x="1.5" y="1.5" width="9" height="9" rx="1.5" fill="none" stroke="currentColor" stroke-width="1.5"/></svg>
    </button>
    <button class="ctrl-btn close" onclick={onClose} title="关闭" aria-label="关闭窗口">
      <svg width="12" height="12" viewBox="0 0 12 12" aria-hidden="true"><path d="M1.5 1.5L10.5 10.5M10.5 1.5L1.5 10.5" stroke="currentColor" stroke-width="1.7" stroke-linecap="round"/></svg>
    </button>
  </div>
</header>

<style>
  .titlebar {
    height: var(--titlebar-height);
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0 8px 0 14px;
    -webkit-app-region: drag;
    user-select: none;
    border-bottom: 1px solid rgba(148, 163, 184, 0.14);
    background: color-mix(in srgb, var(--bg) 82%, transparent);
    backdrop-filter: blur(24px) saturate(1.45);
    -webkit-backdrop-filter: blur(24px) saturate(1.45);
  }

  /* 云母/亚克力：降低背景不透明度，让底层效果透过 */
  :global(html[data-window-effect="mica"]) .titlebar,
  :global(html[data-window-effect="acrylic"]) .titlebar {
    background: color-mix(in srgb, var(--bg) 40%, transparent);
  }

  .left {
    display: flex;
    align-items: center;
    gap: 10px;
    min-width: 0;
  }

  .brand-mark {
    width: 24px;
    height: 24px;
    border-radius: 6px;
    object-fit: contain;
  }

  .brand-copy {
    display: flex;
    flex-direction: column;
    line-height: 1.05;
  }

  .logo {
    font-size: 13px;
    font-weight: 800;
    letter-spacing: 0.02em;
    color: var(--text-strong);
  }

  .controls {
    justify-self: end;
    display: flex;
    gap: 4px;
    -webkit-app-region: no-drag;
  }

  .ctrl-btn {
    width: 34px;
    height: 28px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 8px;
    color: var(--text-secondary);
    transition: background var(--duration-fast) var(--ease-out), color var(--duration-fast) var(--ease-out), transform var(--duration-fast) var(--ease-out);
  }

  .ctrl-btn:hover {
    background: rgba(148, 163, 184, 0.14);
    color: var(--text-strong);
  }

  .ctrl-btn:active {
    transform: scale(0.94);
  }

  .ctrl-btn.close:hover {
    background: #e81123;
    color: white;
  }

  @media (max-width: 760px) {
    .brand-copy {
      display: none;
    }
  }
</style>
