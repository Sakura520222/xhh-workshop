<script lang="ts">
  import { onMount } from "svelte";
  import { notifications } from "../lib/api";
 import { setView, setSelectedLinkId, setSelectedUserId } from "../lib/stores.svelte";
 import { renderTextSync, getEmojiVersion, renderEmojiText } from "../lib/render.svelte";
  import { refreshUnread, markSeen } from "../lib/notification.svelte";

  let messages = $state<any[]>([]);
  let loading = $state(true);
  let error = $state("");
  let offset = $state(0);
  let emojiVer = $derived(getEmojiVersion());

  function rt(text: string): string {
    void emojiVer;
    return renderTextSync(text);
  }
  function rtTitle(text: string): string {
    void emojiVer;
    return renderEmojiText(text);
  }
  let hasMore = $state(true);
  let loadingMore = $state(false);

  // 无限滚动
  let sentinel: HTMLElement | undefined = $state();
  let observer: IntersectionObserver | null = null;
  let prevSentinel: HTMLElement | undefined;

  $effect(() => {
    if (prevSentinel) { observer?.unobserve(prevSentinel); prevSentinel = undefined; }
    if (!sentinel) return;
    if (!observer) {
      const scroller = document.querySelector(".content") as HTMLElement | null;
      observer = new IntersectionObserver(
        (entries) => { if (entries[0]?.isIntersecting) loadMore(); },
        { root: scroller, rootMargin: "200px" }
      );
    }
    observer.observe(sentinel);
    prevSentinel = sentinel;
  });

  async function load(refresh = false) {
    if (refresh) {
      offset = 0;
      hasMore = true;
      loading = true;
      messages = [];
    }
    error = "";
    try {
      const v = await notifications(offset, 20);
      const items = v?.result?.messages ?? [];
      if (refresh) {
        messages = items;
      } else {
        messages = [...messages, ...items];
      }
      offset += items.length;
     if (items.length < 20) hasMore = false;
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
      loadingMore = false;
    }
  }

  // 刷新未读计数并标记当前消息为已见（避免重复弹提示）
 async function syncBadge() {
   try {
     await refreshUnread();
     await markSeen();
   } catch { /* ignore */ }
 }

  function handleRefresh() {
    load(true);
    syncBadge();
  }

  function loadMore() {
    if (loadingMore || !hasMore) return;
    loadingMore = true;
    load();
  }

  function openPost(linkId: string | number) {
    if (!linkId) return;
    setSelectedLinkId(String(linkId));
    setView("detail");
  }

  function openUser(userid: string | number) {
    if (!userid) return;
    setSelectedUserId(String(userid));
    setView("user");
  }

  function fmtTime(ts: any): string {
    const n = Number(ts);
    if (!n) return "";
    const d = new Date(n * 1000);
    const now = new Date();
    const diff = Math.floor((now.getTime() - d.getTime()) / 1000);
    if (diff < 60) return "刚刚";
    if (diff < 3600) return `${Math.floor(diff / 60)} 分钟前`;
    if (diff < 86400) return `${Math.floor(diff / 3600)} 小时前`;
    if (diff < 604800) return `${Math.floor(diff / 86400)} 天前`;
    const mm = String(d.getMonth() + 1).padStart(2, "0");
    const dd = String(d.getDate()).padStart(2, "0");
    const hh = String(d.getHours()).padStart(2, "0");
    const mi = String(d.getMinutes()).padStart(2, "0");
    return `${mm}/${dd} ${hh}:${mi}`;
  }

  function msgTypeLabel(msg: any): string {
    switch (msg.message_type) {
      case 1: return "回复了你";
      case 2: return "评论了";
      case 3: return "赞了你";
      case 4: return "关注了你";
      case 16: return "@了你";
      default: return "";
    }
  }

 onMount(() => {
   load(true);
    syncBadge();
   return () => { observer?.disconnect(); observer = null; };
 });
</script>

<div class="notif-page">
  <div class="topbar">
   <span class="topbar-title">通知</span>
    <button class="refresh-btn" onclick={handleRefresh} disabled={loading}>刷新</button>
  </div>

  {#if loading}
    <div class="status">加载中...</div>
  {:else if error}
    <div class="status error">{error}</div>
  {:else if messages.length === 0}
    <div class="status">暂无通知</div>
  {:else}
    <div class="msg-list">
      {#each messages as msg}
        <div
          class="msg-item"
          role="button"
          tabindex="0"
          onclick={() => openPost(msg.linkid)}
          onkeydown={(e) => { if (e.key === "Enter" || e.key === " ") { e.preventDefault(); openPost(msg.linkid); } }}
        >
          <div class="msg-header">
            <button
              class="user-link"
              onclick={(e: MouseEvent) => { e.stopPropagation(); openUser(msg.userid_a); }}
            >
              {msg.user_a?.username ?? ""}
            </button>
            <span class="msg-action">{msgTypeLabel(msg)}</span>
            <span class="time">{fmtTime(msg.timestamp)}</span>
          </div>
          {#if msg.link_title}
            <div class="msg-target">{@html rtTitle(msg.link_title)}</div>
          {/if}
          {#if msg.comment_a_text}
            <div class="msg-text">{@html rt(msg.comment_a_text)}</div>
          {/if}
        </div>
      {/each}
      {#if hasMore}
        <div bind:this={sentinel} class="sentinel"></div>
      {/if}
      {#if loadingMore}
        <div class="status small">加载更多...</div>
      {:else if !hasMore && messages.length > 0}
        <div class="status small">已加载全部</div>
      {/if}
    </div>
  {/if}
</div>

<style>
  .notif-page {
    max-width: 720px;
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
  .refresh-btn {
    margin-left: auto;
    padding: 6px 14px;
    border-radius: 14px;
    background: rgba(255, 255, 255, 0.12);
    border: 0.5px solid rgba(255, 255, 255, 0.2);
    color: var(--text);
    font-size: 13px;
    box-shadow: inset 0 0.5px 0 rgba(255, 255, 255, 0.3);
    transition: all var(--duration-fast) var(--ease-out);
  }
  .refresh-btn:hover:not(:disabled) {
    background: rgba(255, 255, 255, 0.2);
  }
  .refresh-btn:disabled {
    opacity: 0.5;
  }
  .msg-list {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .msg-item {
    padding: 14px 16px;
    border-radius: var(--radius);
    background: var(--glass-bg);
    border: 0.5px solid var(--glass-border);
    box-shadow: var(--elevation-1);
    cursor: pointer;
    transition: all var(--duration-normal) var(--ease-out);
  }
  .msg-item:hover {
    background: var(--glass-hover);
    border-color: rgba(255, 255, 255, 0.12);
  }
  .msg-header {
    display: flex;
    align-items: center;
    gap: 10px;
    margin-bottom: 6px;
  }
  .user-link {
    font-size: 14px;
    font-weight: 500;
    color: var(--accent);
  }
  .user-link:hover {
    text-decoration: underline;
  }
  .msg-action {
    font-size: 12px;
    color: var(--text-secondary);
  }
  .time {
    font-size: 12px;
    color: var(--text-secondary);
    margin-left: auto;
  }
  .msg-target {
    font-size: 13px;
    font-weight: 500;
    color: var(--text);
    margin-bottom: 4px;
    overflow: hidden;
    display: -webkit-box;
    -webkit-line-clamp: 1;
    line-clamp: 1;
    -webkit-box-orient: vertical;
  }
  .msg-text {
    font-size: 14px;
    line-height: 1.5;
    color: var(--text-secondary);
    white-space: pre-wrap;
    word-break: break-word;
    overflow: hidden;
    display: -webkit-box;
    -webkit-line-clamp: 3;
    line-clamp: 3;
    -webkit-box-orient: vertical;
  }
  .status {
    text-align: center;
    padding: 40px 0;
    color: var(--text-secondary);
  }
  .status.small {
    padding: 16px 0;
    font-size: 13px;
  }
  .status.error {
    color: var(--danger);
  }
  .sentinel {
    height: 1px;
  }
  .msg-text :global(.emoji) {
    width: 1em;
    height: 1em;
    vertical-align: middle;
    display: inline-block;
  }
  .msg-target :global(.emoji) {
    width: 1em;
    height: 1em;
    vertical-align: middle;
    display: inline-block;
  }
</style>
