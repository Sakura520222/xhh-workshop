<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { authGetQrCode, authLogin, authCancelLogin, onAuthScanned, type QrCodeResp } from "../lib/api";
  import { setAuth } from "../lib/stores.svelte";

  let qr = $state<QrCodeResp | null>(null);
  let status = $state<"loading" | "waiting" | "success" | "error">("loading");
  let errorMsg = $state("");
  let scanned = $state(false);
  let unlistenScanned: (() => void) | null = null;

  function renderQrSvg(url: string) {
    qrSvg = url;
  }
  let qrSvg = $state("");

  // 本地轮次序号：每开一轮自增，过期轮次的返回不再写状态
  let loginSeq = 0;

  async function startLogin() {
    const seq = ++loginSeq;
    status = "loading";
    errorMsg = "";
    scanned = false;
    try {
      qr = await authGetQrCode();
      if (seq !== loginSeq) return;
      renderQrSvg(qr.qr_url);
      status = "waiting";
      const result = await authLogin(qr.raw_query, "web-client");
      if (seq !== loginSeq) return;
      if (result.ok) {
        status = "success";
        setAuth(result);
      } else if (result.message === "已取消") {
        // restartLogin 主动取消，新一轮已接管状态
      } else {
        status = "error";
        errorMsg = result.message;
      }
    } catch (e) {
      if (seq !== loginSeq) return;
      status = "error";
      errorMsg = String(e);
    }
  }

  // 刷新二维码 / 扫码后取消：先让后端停止当前轮询，再开新一轮
  async function restartLogin() {
    await authCancelLogin();
    startLogin();
  }

  onMount(() => {
    unlistenScanned = onAuthScanned(() => {
      scanned = true;
    });
    startLogin();
  });

  onDestroy(() => {
    unlistenScanned?.();
  });
</script>

<div class="login-page">
  <section class="login-card" aria-label="扫码登录">
    <div class="card-head">
      <span class="card-kicker">扫码登录</span>
      <h1>使用小黑盒 APP 扫码</h1>
      <p class="subtitle">打开小黑盒 APP，扫描下方二维码完成登录</p>
    </div>

    <div class="qr-frame">
      {#if status === "loading"}
        <div class="qr-area" role="status" aria-live="polite">
          <div class="placeholder">
            <div class="loader"></div>
            <span>正在获取二维码</span>
          </div>
        </div>
      {:else if status === "waiting" && qr}
        <div class="qr-area">
          <img src={`https://api.qrserver.com/v1/create-qr-code/?size=220x220&data=${encodeURIComponent(qr.qr_url)}`} alt="小黑盒扫码登录二维码" class="qr-img" />
        </div>
      {:else if status === "success"}
        <div class="qr-area" role="status" aria-live="polite">
          <div class="placeholder success">
            <span class="success-mark">✓</span>
            <span>登录成功</span>
          </div>
        </div>
      {:else}
        <div class="qr-area">
          <div class="placeholder error" role="alert">{errorMsg}</div>
        </div>
      {/if}

      {#if scanned && status === "waiting"}
        <div class="qr-scan-overlay" role="status" aria-live="polite">
          <div class="scan-pulse"></div>
          <span class="scan-text">已扫码，请在手机端确认</span>
          <button class="cancel-btn" onclick={restartLogin}>取消登录</button>
        </div>
      {/if}
    </div>

    {#if status === "waiting" && !scanned}
      <button class="refresh-btn" onclick={restartLogin}>刷新二维码</button>
    {:else if status === "waiting" && scanned}
      <p class="hint">请在手机端确认，或取消后重新扫码</p>
    {:else if status === "error"}
      <button class="retry-btn" onclick={startLogin}>重新获取二维码</button>
    {:else}
      <p class="hint-placeholder">&nbsp;</p>
    {/if}
  </section>
</div>

<style>
  .login-page {
    min-height: calc(100vh - var(--titlebar-height));
    display: grid;
    place-items: center;
    padding: 42px;
  }

  .login-card {
    position: relative;
    overflow: hidden;
    width: min(400px, 100%);
    border-radius: 30px;
    border: 1px solid var(--glass-border);
    background: linear-gradient(180deg, color-mix(in srgb, var(--bg-soft) 78%, transparent), color-mix(in srgb, var(--bg-soft) 57%, transparent));
    box-shadow: var(--elevation-3);
    backdrop-filter: var(--glass-blur);
    -webkit-backdrop-filter: var(--glass-blur);
    padding: 44px 36px;
    display: flex;
    flex-direction: column;
    align-items: center;
  }

  .card-head {
    text-align: center;
    margin-bottom: 28px;
  }

  .card-kicker {
    color: var(--accent-hover);
    font-size: 12px;
    font-weight: 800;
    letter-spacing: 0.1em;
    text-transform: uppercase;
  }

  h1 {
    margin-top: 10px;
    font-size: 24px;
    font-weight: 850;
    letter-spacing: -0.03em;
    color: var(--text-strong);
  }

  .subtitle {
    margin-top: 8px;
    color: var(--text-secondary);
    font-size: 14px;
    line-height: 1.6;
  }

  .qr-frame {
    position: relative;
    display: flex;
    justify-content: center;
    align-items: center;
  }

  .qr-area {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 16px;
  }

  .qr-img {
    width: 220px;
    height: 220px;
    border-radius: 24px;
    background: white;
    padding: 12px;
    box-shadow: 0 24px 70px rgba(2, 6, 23, 0.42);
  }

  .placeholder {
    width: 220px;
    height: 220px;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 14px;
    border-radius: 24px;
    background: rgba(148, 163, 184, 0.08);
    border: 1px solid rgba(148, 163, 184, 0.18);
    color: var(--text-secondary);
    font-size: 14px;
    text-align: center;
  }

  .loader {
    width: 34px;
    height: 34px;
    border-radius: 50%;
    border: 3px solid rgba(148, 163, 184, 0.24);
    border-top-color: var(--accent);
    animation: spin 900ms linear infinite;
  }

  .placeholder.success {
    color: #bbf7d0;
    background: var(--success-soft);
    border-color: rgba(34, 197, 94, 0.24);
    font-size: 16px;
    font-weight: 800;
  }

  .success-mark {
    width: 46px;
    height: 46px;
    display: grid;
    place-items: center;
    border-radius: 50%;
    background: var(--success);
    color: #052e16;
    font-size: 28px;
  }

  .placeholder.error {
    color: #fecaca;
    background: var(--danger-soft);
    border-color: rgba(248, 113, 113, 0.24);
    padding: 18px;
    line-height: 1.6;
  }

  .qr-scan-overlay {
    position: absolute;
    inset: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 16px;
    border-radius: 24px;
    background: color-mix(in srgb, var(--bg-soft) 72%, transparent);
    backdrop-filter: blur(6px);
    -webkit-backdrop-filter: blur(6px);
    animation: overlay-in 280ms var(--ease-out);
  }

  .scan-pulse {
    width: 52px;
    height: 52px;
    border-radius: 50%;
    border: 3px solid var(--accent);
    box-shadow: 0 0 0 0 color-mix(in srgb, var(--accent) 50%, transparent);
    animation: pulse 1.6s ease-out infinite;
  }

  .scan-text {
    color: var(--text-strong);
    font-size: 15px;
    font-weight: 700;
    letter-spacing: 0.01em;
  }

  .hint,
  .hint-placeholder {
    margin-top: 20px;
    font-size: 13px;
    color: var(--text-muted);
    text-align: center;
  }

  .retry-btn,
  .refresh-btn {
    margin-top: 20px;
    min-height: 44px;
    padding: 0 22px;
    border-radius: 999px;
    font-size: 14px;
    font-weight: 750;
    transition: transform var(--duration-fast) var(--ease-out), filter var(--duration-fast) var(--ease-out), border-color var(--duration-fast) var(--ease-out);
  }

  .retry-btn {
    background: linear-gradient(135deg, var(--accent), var(--accent-strong));
    color: white;
    box-shadow: 0 16px 36px color-mix(in srgb, var(--accent-strong) 30%, transparent);
  }

  .refresh-btn {
    border: 1px solid var(--glass-border);
    background: color-mix(in srgb, var(--bg-soft) 40%, transparent);
    color: var(--text-strong);
  }

  .retry-btn:hover,
  .refresh-btn:hover {
    filter: brightness(1.08);
    transform: translateY(-1px);
  }

  .refresh-btn:hover {
    border-color: color-mix(in srgb, var(--accent) 50%, transparent);
  }

  .cancel-btn {
    min-height: 38px;
    padding: 0 20px;
    border-radius: 999px;
    border: 1px solid rgba(248, 113, 113, 0.4);
    background: color-mix(in srgb, var(--danger-soft) 60%, transparent);
    color: #fecaca;
    font-size: 13px;
    font-weight: 700;
    transition: transform var(--duration-fast) var(--ease-out), filter var(--duration-fast) var(--ease-out);
  }

  .cancel-btn:hover {
    filter: brightness(1.08);
    transform: translateY(-1px);
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }

  @keyframes overlay-in {
    from { opacity: 0; }
    to { opacity: 1; }
  }

  @keyframes pulse {
    0% { box-shadow: 0 0 0 0 color-mix(in srgb, var(--accent) 50%, transparent); }
    70% { box-shadow: 0 0 0 18px color-mix(in srgb, var(--accent) 0%, transparent); }
    100% { box-shadow: 0 0 0 0 color-mix(in srgb, var(--accent) 0%, transparent); }
  }

  @media (max-width: 900px) {
    .login-page {
      padding: 24px;
    }
  }
</style>
