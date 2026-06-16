<script lang="ts">
  import { onMount } from "svelte";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import Login from "./views/Login.svelte";
  import Home from "./views/Home.svelte";
  import PostDetail from "./views/PostDetail.svelte";
  import Editor from "./views/Editor.svelte";
  import Profile from "./views/Profile.svelte";
  import Agent from "./views/Agent.svelte";
  import Settings from "./views/Settings.svelte";
  import Search from "./views/Search.svelte";
  import Notifications from "./views/Notifications.svelte";
  import Favourites from "./views/Favourites.svelte";
  import UserView from "./views/UserView.svelte";
 import TitleBar from "./components/TitleBar.svelte";
 import Sidebar from "./components/Sidebar.svelte";
  import ToastHost from "./components/ToastHost.svelte";
 import { getAuth, getView, refreshAuth, isAuthChecking } from "./lib/stores.svelte";
  import { startPolling, stopPolling } from "./lib/notification.svelte";

 let auth = $derived(getAuth());
 let checking = $derived(isAuthChecking());

 onMount(async () => {
   await refreshAuth();
 });

  // 已登录时启动通知轮询，登出/未登录时停止
  $effect(() => {
    if (auth.ok) {
      startPolling();
    } else {
      stopPolling();
    }
  });

  async function handleMinimize() {
    await getCurrentWindow().minimize();
  }
  async function handleMaximize() {
    const win = getCurrentWindow();
    if (await win.isMaximized()) {
      await win.unmaximize();
    } else {
      await win.maximize();
    }
  }
  async function handleClose() {
    await getCurrentWindow().close();
  }
</script>

<TitleBar onMinimize={handleMinimize} onMaximize={handleMaximize} onClose={handleClose} />

<ToastHost />

{#if checking}
  <div class="splash">
    <div class="splash-card" role="status" aria-live="polite">
      <div class="loader"></div>
      <div>
        <div class="splash-title">正在启动</div>
        <div class="splash-subtitle">开启你的全新黑盒之旅</div>
      </div>
    </div>
  </div>
{:else if !auth.ok}
  <Login />
{:else}
  <div class="main-layout">
    <Sidebar />
    <main class="content" aria-label="主内容区">
      <!-- Agent 常驻挂载（keep-alive）：切换页面时仅隐藏，保留流式状态/消息/确认弹窗 -->
      <div
        class="agent-mount"
        class:hidden={getView() !== "agent"}
        aria-hidden={getView() !== "agent"}
      >
        <Agent />
      </div>
      {#if getView() !== "agent"}
        {#if getView() === "home"}
          <Home />
        {:else if getView() === "detail"}
          <PostDetail />
        {:else if getView() === "editor"}
          <Editor />
        {:else if getView() === "profile"}
          <Profile />
        {:else if getView() === "settings"}
          <Settings />
        {:else if getView() === "search"}
          <Search />
        {:else if getView() === "notifications"}
          <Notifications />
        {:else if getView() === "favourites"}
          <Favourites />
        {:else if getView() === "user"}
          <UserView />
        {/if}
      {/if}
    </main>
  </div>
{/if}

<style>
  .main-layout {
    position: relative;
    display: flex;
    height: calc(100vh - var(--titlebar-height));
    min-height: 0;
  }

  .main-layout::before {
    content: "";
    position: absolute;
    inset: 16px 18px 18px calc(var(--sidebar-width) + 18px);
    pointer-events: none;
    border-radius: 32px;
    background:
      radial-gradient(circle at 24% 12%, color-mix(in srgb, var(--glow-1) 12%, transparent), transparent 34%),
      radial-gradient(circle at 82% 26%, color-mix(in srgb, var(--glow-2) 9%, transparent), transparent 30%);
    filter: blur(4px);
    opacity: 0.95;
  }

  .content {
    position: relative;
    flex: 1;
    min-width: 0;
    overflow-y: auto;
    padding: 28px 32px 40px;
    scroll-padding-top: 24px;
  }

  /* Agent 常驻挂载点：与 .content 同高，hidden 时不占位 */
  .agent-mount {
    height: 100%;
    min-height: 0;
  }

  .agent-mount.hidden {
    display: none;
  }

  .splash {
    height: calc(100vh - var(--titlebar-height));
    display: grid;
    place-items: center;
    padding: 24px;
  }

  .splash-card {
    display: flex;
    align-items: center;
    gap: 16px;
    min-width: 280px;
    padding: 18px 20px;
    border-radius: var(--radius-lg);
    background: var(--surface);
    border: 1px solid var(--glass-border);
    box-shadow: var(--elevation-2);
    backdrop-filter: var(--glass-blur);
    -webkit-backdrop-filter: var(--glass-blur);
  }

  .loader {
    width: 34px;
    height: 34px;
    border-radius: 50%;
    border: 3px solid rgba(148, 163, 184, 0.24);
    border-top-color: var(--accent);
    animation: spin 900ms linear infinite;
  }

  .splash-title {
    font-size: 14px;
    font-weight: 700;
    color: var(--text-strong);
  }

  .splash-subtitle {
    margin-top: 3px;
    font-size: 12px;
    color: var(--text-secondary);
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }

  @media (max-width: 960px) {
    .content {
      padding: 22px 20px 32px;
    }
  }
</style>
