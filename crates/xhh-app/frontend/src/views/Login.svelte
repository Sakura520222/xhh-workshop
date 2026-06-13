<script lang="ts">
  import { onMount } from "svelte";
  import { authGetQrCode, authLogin, type QrCodeResp } from "../lib/api";
  import { setAuth } from "../lib/stores.svelte";

  let qr = $state<QrCodeResp | null>(null);
  let status = $state<"loading" | "waiting" | "success" | "error">("loading");
  let errorMsg = $state("");
  let qrSvg = $state("");

  function renderQrSvg(url: string) {
    qrSvg = url;
  }

  async function startLogin() {
    status = "loading";
    errorMsg = "";
    try {
      qr = await authGetQrCode();
      renderQrSvg(qr.qr_url);
      status = "waiting";
      const result = await authLogin(qr.raw_query, "web-client");
      if (result.ok) {
        status = "success";
        setAuth(result);
      } else {
        status = "error";
        errorMsg = result.message;
      }
    } catch (e) {
      status = "error";
      errorMsg = String(e);
    }
  }

  onMount(startLogin);
</script>

<div class="login-page">
  <div class="login-shell">
    <section class="login-copy" aria-labelledby="login-title">
      <span class="eyebrow">Secure Community Console</span>
      <h1 id="login-title">小黑盒社区工作台</h1>
      <p class="subtitle">扫码后即可进入内容运营、帖子管理与 Agent 自动化能力集合。</p>
      <div class="feature-list" aria-label="能力概览">
        <div class="feature-item">
          <span class="feature-dot"></span>
          <div>
            <strong>内容中枢</strong>
            <span>热帖、搜索、收藏与通知统一管理</span>
          </div>
        </div>
        <div class="feature-item">
          <span class="feature-dot"></span>
          <div>
            <strong>AI 助手</strong>
            <span>支持多轮上下文与工具调用</span>
          </div>
        </div>
        <div class="feature-item">
          <span class="feature-dot"></span>
          <div>
            <strong>安全登录</strong>
            <span>使用小黑盒 APP 官方扫码链路</span>
          </div>
        </div>
      </div>
    </section>

    <section class="login-card" aria-label="扫码登录">
      <div class="card-head">
        <span class="card-kicker">登录验证</span>
        <h2>使用小黑盒 APP 扫码</h2>
      </div>

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
          <p class="hint">二维码有效期内请完成确认</p>
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
          <button class="retry-btn" onclick={startLogin}>重新获取二维码</button>
        </div>
      {/if}
    </section>
  </div>
</div>

<style>
  .login-page {
    min-height: calc(100vh - var(--titlebar-height));
    display: grid;
    place-items: center;
    padding: 42px;
  }

  .login-shell {
    width: min(1040px, 100%);
    display: grid;
    grid-template-columns: minmax(0, 1.2fr) 390px;
    gap: 28px;
    align-items: stretch;
  }

  .login-copy,
  .login-card {
    position: relative;
    overflow: hidden;
    border-radius: 30px;
    border: 1px solid var(--glass-border);
    background: linear-gradient(180deg, color-mix(in srgb, var(--bg-soft) 78%, transparent), color-mix(in srgb, var(--bg-soft) 57%, transparent));
    box-shadow: var(--elevation-3);
    backdrop-filter: var(--glass-blur);
    -webkit-backdrop-filter: var(--glass-blur);
  }

  .login-copy {
    min-height: 520px;
    padding: 48px;
    display: flex;
    flex-direction: column;
    justify-content: flex-end;
  }

  .login-copy::before {
    content: "";
    position: absolute;
    inset: -25% -15% auto auto;
    width: 420px;
    height: 420px;
    border-radius: 50%;
    background: radial-gradient(circle, color-mix(in srgb, var(--accent) 32%, transparent), transparent 64%);
  }

  .login-copy::after {
    content: "";
    position: absolute;
    left: 44px;
    top: 44px;
    width: 88px;
    height: 88px;
    border-radius: 28px;
    background: linear-gradient(135deg, var(--accent), var(--accent-warm));
    box-shadow: 0 26px 70px color-mix(in srgb, var(--accent) 32%, transparent);
  }

  .eyebrow,
  h1,
  .subtitle,
  .feature-list {
    position: relative;
  }

  .eyebrow {
    display: inline-flex;
    width: fit-content;
    padding: 7px 11px;
    border-radius: 999px;
    background: var(--accent-soft);
    border: 1px solid color-mix(in srgb, var(--accent-hover) 20%, transparent);
    color: #bfdbfe;
    font-size: 12px;
    font-weight: 800;
    letter-spacing: 0.08em;
    text-transform: uppercase;
  }

  h1 {
    margin-top: 18px;
    max-width: 620px;
    font-size: clamp(40px, 5vw, 64px);
    font-weight: 900;
    line-height: 1.03;
    letter-spacing: -0.05em;
    color: var(--text-strong);
  }

  .subtitle {
    max-width: 520px;
    margin-top: 18px;
    color: var(--text-secondary);
    font-size: 16px;
    line-height: 1.8;
  }

  .feature-list {
    display: grid;
    gap: 12px;
    margin-top: 34px;
  }

  .feature-item {
    display: flex;
    align-items: flex-start;
    gap: 12px;
    padding: 14px;
    border-radius: 18px;
    background: color-mix(in srgb, var(--bg-soft) 54%, transparent);
    border: 1px solid rgba(148, 163, 184, 0.12);
  }

  .feature-dot {
    width: 10px;
    height: 10px;
    margin-top: 6px;
    border-radius: 50%;
    background: linear-gradient(135deg, var(--accent), var(--accent-warm));
    box-shadow: 0 0 0 5px color-mix(in srgb, var(--accent) 12%, transparent);
    flex-shrink: 0;
  }

  .feature-item strong {
    display: block;
    font-size: 14px;
    color: var(--text-strong);
  }

  .feature-item span:last-child {
    display: block;
    margin-top: 4px;
    color: var(--text-muted);
    font-size: 13px;
  }

  .login-card {
    padding: 32px;
    display: flex;
    flex-direction: column;
    justify-content: center;
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

  h2 {
    margin-top: 8px;
    font-size: 24px;
    font-weight: 850;
    letter-spacing: -0.03em;
    color: var(--text-strong);
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

  .hint {
    font-size: 13px;
    color: var(--text-muted);
  }

  .retry-btn {
    min-height: 44px;
    padding: 0 22px;
    border-radius: 999px;
    background: linear-gradient(135deg, var(--accent), var(--accent-strong));
    color: white;
    font-size: 14px;
    font-weight: 750;
    box-shadow: 0 16px 36px color-mix(in srgb, var(--accent-strong) 30%, transparent);
    transition: transform var(--duration-fast) var(--ease-out), filter var(--duration-fast) var(--ease-out);
  }

  .retry-btn:hover {
    filter: brightness(1.08);
    transform: translateY(-1px);
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }

  @media (max-width: 900px) {
    .login-page {
      padding: 24px;
    }

    .login-shell {
      grid-template-columns: 1fr;
    }

    .login-copy {
      min-height: 420px;
      padding: 34px;
    }
  }
</style>
