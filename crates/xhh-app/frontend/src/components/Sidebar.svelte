<script lang="ts">
  import { getView, setView, getAuth, getColorMode, toggleColorMode } from "../lib/stores.svelte";
  import { authLogout } from "../lib/api";
  import { getUnread } from "../lib/notification.svelte";

  let view = $derived(getView());
  let auth = $derived(getAuth());
  let unread = $derived(getUnread());
  let colorMode = $derived(getColorMode());
  let avatarBroken = $state(false);
  let showAvatar = $derived(!!auth.avatar && !avatarBroken);
  $effect(() => {
    auth.avatar;
    avatarBroken = false;
  });

  const icons: Record<string, string> = {
    home: '<path d="M3 9l9-7 9 7v11a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z"/><polyline points="9 22 9 12 15 12 15 22"/>',
    editor: '<path d="M12 5v14"/><path d="M5 12h14"/>',
    bell: '<path d="M18 8A6 6 0 0 0 6 8c0 7-3 9-3 9h18s-3-2-3-9"/><path d="M13.73 21a2 2 0 0 1-3.46 0"/>',
    bookmark: '<path d="M19 21l-7-5-7 5V5a2 2 0 0 1 2-2h10a2 2 0 0 1 2 2z"/>',
    user: '<path d="M20 21v-2a4 4 0 0 0-4-4H8a4 4 0 0 0-4 4v2"/><circle cx="12" cy="7" r="4"/>',
    zap: '<polygon points="13 2 3 14 12 14 11 22 21 10 12 10 13 2"/>',
    settings: '<line x1="4" x2="4" y1="21" y2="14"/><line x1="4" x2="4" y1="10" y2="3"/><line x1="12" x2="12" y1="21" y2="12"/><line x1="12" x2="12" y1="8" y2="3"/><line x1="20" x2="20" y1="21" y2="16"/><line x1="20" x2="20" y1="12" y2="3"/><line x1="2" x2="6" y1="14" y2="14"/><line x1="10" x2="14" y1="8" y2="8"/><line x1="18" x2="22" y1="16" y2="16"/>',
    logout: '<path d="M9 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h4"/><polyline points="16 17 21 12 16 7"/><line x1="21" y1="12" x2="9" y2="12"/>',
    sun: '<circle cx="12" cy="12" r="4"/><path d="M12 2v2"/><path d="M12 20v2"/><path d="m4.93 4.93 1.41 1.41"/><path d="m17.66 17.66 1.41 1.41"/><path d="M2 12h2"/><path d="M20 12h2"/><path d="m6.34 17.66-1.41 1.41"/><path d="m19.07 4.93-1.41 1.41"/>',
    moon: '<path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79Z"/>',
  };

  let menus = [
    { key: "home", label: "主页", icon: "home" },
    { key: "editor", label: "发帖", icon: "editor" },
    { key: "notifications", label: "通知", icon: "bell" },
    { key: "favourites", label: "收藏", icon: "bookmark" },
    { key: "profile", label: "我的", icon: "user" },
    { key: "agent", label: "Agent", icon: "zap" },
    { key: "settings", label: "设置", icon: "settings" },
  ] as const;

  async function handleLogout() {
    await authLogout();
    location.reload();
  }
</script>

<nav class="sidebar" aria-label="主导航">
  <div class="user-info">
    <div class="avatar" aria-hidden="true">
      {#if showAvatar}
        <img src={auth.avatar} alt={auth.nickname} class="avatar-img" onerror={() => (avatarBroken = true)} />
      {:else}
        {auth.nickname.charAt(0) || "?"}
      {/if}
    </div>
    <div class="user-text">
      <div class="name">{auth.nickname || "未登录"}</div>
      <div class="id">ID: {auth.heybox_id}</div>
    </div>
  </div>

  <div class="menu-head">导航</div>
  <div class="menu">
    {#each menus as m}
      <button
        class="menu-item"
        class:active={view === m.key}
        aria-current={view === m.key ? "page" : undefined}
        aria-label={`打开${m.label}`}
        onclick={() => setView(m.key)}
      >
       <span class="icon-shell">
         <svg class="mi" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">{@html icons[m.icon] ?? ""}</svg>
       </span>
       <span class="label">{m.label}</span>
        {#if m.key === "notifications" && unread > 0}
          <span class="badge" aria-label={`${unread} 条未读`}>{unread > 99 ? "99+" : unread}</span>
        {/if}
     </button>
    {/each}
  </div>

  <div class="bottom">
    <button
      class="mode-toggle"
      onclick={toggleColorMode}
      aria-label="切换明暗模式"
      aria-pressed={colorMode === "light"}
      title={colorMode === "light" ? "切换到深色" : "切换到浅色"}
    >
      <span class="icon-shell">
        <svg class="mi" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">{@html colorMode === "light" ? icons.sun : icons.moon}</svg>
      </span>
      <span class="label">{colorMode === "light" ? "浅色" : "深色"}</span>
      <span class="switch" aria-hidden="true"><span class="switch-thumb"></span></span>
    </button>
    <button class="menu-item logout" onclick={handleLogout} aria-label="退出登录">
      <span class="icon-shell">
        <svg class="mi" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">{@html icons.logout}</svg>
      </span>
      <span class="label">退出登录</span>
    </button>
  </div>
</nav>

<style>
  .sidebar {
    width: var(--sidebar-width);
    display: flex;
    flex-direction: column;
    gap: 14px;
    padding: 18px 14px;
    border-right: 1px solid rgba(148, 163, 184, 0.14);
    background: linear-gradient(180deg, color-mix(in srgb, var(--bg) 90%, transparent) 0%, color-mix(in srgb, var(--bg-soft) 76%, transparent) 100%);
    backdrop-filter: blur(28px) saturate(1.35);
    -webkit-backdrop-filter: blur(28px) saturate(1.35);
  }

  /* 云母/亚克力：降低背景不透明度，让底层效果透过 */
  :global(html[data-window-effect="mica"]) .sidebar,
  :global(html[data-window-effect="acrylic"]) .sidebar {
    background: linear-gradient(180deg, color-mix(in srgb, var(--bg) 45%, transparent) 0%, color-mix(in srgb, var(--bg-soft) 35%, transparent) 100%);
  }

  .user-info {
    display: flex;
    align-items: center;
    gap: 12px;
    min-height: 66px;
    padding: 10px;
    border-radius: 18px;
    background: color-mix(in srgb, var(--bg-soft) 50%, transparent);
    border: 1px solid rgba(148, 163, 184, 0.12);
  }

  .avatar {
    width: 44px;
    height: 44px;
    border-radius: 16px;
    background: linear-gradient(135deg, var(--accent), var(--accent-warm));
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    font-size: 16px;
    font-weight: 800;
    overflow: hidden;
    box-shadow: 0 0 0 3px color-mix(in srgb, var(--accent) 14%, transparent), 0 12px 24px rgba(2, 6, 23, 0.26);
  }

  .avatar-img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .user-text {
    min-width: 0;
    overflow: hidden;
  }

  .name {
    font-size: 14px;
    font-weight: 700;
    color: var(--text-strong);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .id {
    margin-top: 4px;
    font-size: 12px;
    color: var(--text-muted);
  }

  .menu-head {
    padding: 0 10px;
    font-size: 11px;
    color: var(--text-muted);
    letter-spacing: 0.08em;
    text-transform: uppercase;
  }

  .menu {
    display: flex;
    flex-direction: column;
    gap: 6px;
    flex: 1;
  }

  .menu-item {
    position: relative;
    display: flex;
    align-items: center;
    gap: 11px;
    min-height: 44px;
    padding: 9px 12px;
    border-radius: 14px;
    font-size: 14px;
    font-weight: 650;
    color: var(--text-secondary);
    transition: background var(--duration-normal) var(--ease-out), color var(--duration-normal) var(--ease-out), transform var(--duration-fast) var(--ease-out), box-shadow var(--duration-normal) var(--ease-out);
    width: 100%;
    text-align: left;
  }

  .menu-item::before {
    content: "";
    position: absolute;
    left: -7px;
    width: 3px;
    height: 18px;
    border-radius: 999px;
    background: linear-gradient(180deg, var(--accent), var(--accent-warm));
    opacity: 0;
    transform: scaleY(0.45);
    transition: opacity var(--duration-normal) var(--ease-out), transform var(--duration-normal) var(--ease-out);
  }

  .menu-item:hover {
    background: rgba(148, 163, 184, 0.1);
    color: var(--text-strong);
  }

  .menu-item:active {
    transform: scale(0.985);
  }

  .menu-item.active {
    background: linear-gradient(135deg, color-mix(in srgb, var(--accent) 22%, transparent), color-mix(in srgb, var(--accent-warm) 12%, transparent));
    color: var(--text-strong);
    box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--accent-hover) 18%, transparent), 0 12px 28px rgba(2, 6, 23, 0.2);
  }

  .menu-item.active::before {
    opacity: 1;
    transform: scaleY(1);
  }

  .icon-shell {
    width: 30px;
    height: 30px;
    display: grid;
    place-items: center;
    flex-shrink: 0;
    border-radius: 10px;
    background: rgba(148, 163, 184, 0.08);
    color: currentColor;
  }

 .menu-item.active .icon-shell {
   background: color-mix(in srgb, var(--accent) 22%, transparent);
   color: var(--on-accent-soft);
 }

  .badge {
    margin-left: auto;
    min-width: 18px;
    height: 18px;
    padding: 0 5px;
    border-radius: 999px;
    background: linear-gradient(135deg, var(--accent-warm), #ef4444);
    color: #fff;
    font-size: 11px;
    font-weight: 700;
    line-height: 18px;
    text-align: center;
    box-shadow: 0 2px 8px rgba(239, 68, 68, 0.4);
    flex-shrink: 0;
  }

 .mi {
    flex-shrink: 0;
  }

  .bottom {
    margin-top: auto;
    padding-top: 12px;
    border-top: 1px solid rgba(148, 163, 184, 0.12);
  }

  .mode-toggle {
    position: relative;
    display: flex;
    align-items: center;
    gap: 11px;
    min-height: 44px;
    margin-bottom: 6px;
    padding: 9px 12px;
    border-radius: 14px;
    width: 100%;
    text-align: left;
    font-size: 14px;
    font-weight: 650;
    color: var(--text-secondary);
    transition: background var(--duration-normal) var(--ease-out), color var(--duration-normal) var(--ease-out);
  }

  .mode-toggle:hover {
    background: rgba(148, 163, 184, 0.1);
    color: var(--text-strong);
  }

  .switch {
    margin-left: auto;
    position: relative;
    width: 36px;
    height: 20px;
    border-radius: 999px;
    background: rgba(148, 163, 184, 0.28);
    flex-shrink: 0;
    transition: background var(--duration-normal) var(--ease-out);
  }

  .switch-thumb {
    position: absolute;
    top: 2px;
    left: 2px;
    width: 16px;
    height: 16px;
    border-radius: 50%;
    background: #fff;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.32);
    transition: transform var(--duration-normal) var(--ease-spring);
  }

  .mode-toggle[aria-pressed="true"] .switch {
    background: var(--accent);
  }

  .mode-toggle[aria-pressed="true"] .switch-thumb {
    transform: translateX(16px);
  }

  .logout {
    color: rgba(248, 113, 113, 0.86);
  }

  .logout:hover {
    background: var(--danger-soft);
    color: var(--danger-fg);
  }
</style>
