<script lang="ts">
  import { onMount } from "svelte";
  import { userProfile, userEvents, followUser, unfollowUser } from "../lib/api";
  import { getSelectedUserId, setView } from "../lib/stores.svelte";

  let userid = $derived(getSelectedUserId());
  let profile = $state<any>(null);
  let posts = $state<any[]>([]);
  let loading = $state(true);
  let error = $state("");
  let lastval = $state<string | undefined>(undefined);
  let hasMore = $state(true);
  let loadingMore = $state(false);
  let isFollowed = $state(false);

  async function load() {
    if (!userid) return;
    loading = true;
    error = "";
    posts = [];
    lastval = undefined;
    hasMore = true;
    try {
      const [p, e] = await Promise.all([
        userProfile(userid),
        userEvents(userid),
      ]);
      profile = p?.result?.account_detail ?? null;
      isFollowed = p?.result?.is_followed === 1;
      const items = e?.result?.list ?? [];
      posts = items;
      lastval = e?.result?.lastval;
      if (items.length < 20) hasMore = false;
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  async function loadMore() {
    if (loadingMore || !hasMore || !userid) return;
    loadingMore = true;
    try {
      const e = await userEvents(userid, lastval);
      const items = e?.result?.list ?? [];
      posts = [...posts, ...items];
      lastval = e?.result?.lastval;
      if (items.length < 20) hasMore = false;
    } catch (e) {
      console.warn("[user] load more failed:", e);
      hasMore = false;
    } finally {
      loadingMore = false;
    }
  }

  async function toggleFollow() {
    if (!userid) return;
    try {
      const r = await (isFollowed ? unfollowUser(userid) : followUser(userid));
      isFollowed = r?.result?.is_followed === 1;
    } catch (e) {
      console.error(e);
    }
  }

  function openPost(linkId: string) {
    if (!linkId) return;
    window.history.pushState({}, "", `#detail/${linkId}`);
    setView("detail");
  }

  onMount(() => load());
</script>

<div class="user-page">
  {#if loading}
    <div class="status">加载中...</div>
  {:else if error}
    <div class="status error">{error}</div>
  {:else if profile}
    <div class="profile-card">
      <img class="avatar" src={profile.avatar ?? ""} alt="" />
      <div class="info">
        <div class="username">{profile.username ?? ""}</div>
        <div class="sub">
          {#if profile.level_info}
            <span class="level">Lv.{profile.level_info.level ?? 0}</span>
          {/if}
          <span class="userid">ID: {userid}</span>
        </div>
      </div>
      {#if userid}
        <button class="follow-btn" class:followed={isFollowed} onclick={toggleFollow}>
          {isFollowed ? "已关注" : "关注"}
        </button>
      {/if}
    </div>

    <div class="section-title">动态</div>

    {#if posts.length === 0}
      <div class="status">暂无动态</div>
    {:else}
      <div class="post-list">
        {#each posts as p}
          <div
            class="post-item"
            role="button"
            tabindex="0"
            onclick={() => openPost(String(p.linkid))}
            onkeydown={(e) => { if (e.key === "Enter" || e.key === " ") { e.preventDefault(); openPost(String(p.linkid)); } }}
          >
            <div class="post-title">{p.title ?? ""}</div>
            <div class="post-meta">
              <span>{p.comment_num ?? 0} 评论</span>
              <span>{p.up ?? 0} 赞</span>
            </div>
          </div>
        {/each}
      </div>
      {#if hasMore}
        <button class="load-more" onclick={loadMore} disabled={loadingMore}>
          {loadingMore ? "加载中..." : "加载更多"}
        </button>
      {/if}
    {/if}
  {:else}
    <div class="status">未选择用户</div>
  {/if}
</div>

<style>
  .user-page {
    max-width: 720px;
    margin: 0 auto;
  }
  .profile-card {
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 16px;
    border-radius: var(--radius);
    background: var(--glass-bg);
    backdrop-filter: var(--glass-blur);
    -webkit-backdrop-filter: var(--glass-blur);
    border: 0.5px solid var(--glass-border);
    box-shadow: var(--elevation-1);
    margin-bottom: 20px;
  }
  .avatar {
    width: 56px;
    height: 56px;
    border-radius: 50%;
    object-fit: cover;
    flex-shrink: 0;
    box-shadow: 0 0 0 2px rgba(255, 107, 53, 0.2);
  }
  .info {
    flex: 1;
    min-width: 0;
  }
  .username {
    font-size: 15px;
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .sub {
    display: flex;
    gap: 8px;
    font-size: 12px;
    color: var(--text-secondary);
    margin-top: 4px;
  }
  .follow-btn {
    padding: 6px 18px;
    border-radius: 10px;
    background: var(--accent);
    border: 0.5px solid transparent;
    font-size: 13px;
    flex-shrink: 0;
    cursor: pointer;
    box-shadow: 0 2px 8px rgba(255, 107, 53, 0.3);
    transition: all var(--duration-fast) var(--ease-out);
  }
  .follow-btn.followed {
    opacity: 0.6;
    background: var(--glass-bg);
    border-color: var(--glass-border);
    box-shadow: none;
  }
  .section-title {
    font-size: 14px;
    font-weight: 600;
    padding: 8px 0 12px;
  }
  .post-list {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .post-item {
    padding: 14px 16px;
    border-radius: var(--radius);
    background: var(--glass-bg);
    border: 0.5px solid var(--glass-border);
    box-shadow: var(--elevation-1);
    cursor: pointer;
    transition: all var(--duration-normal) var(--ease-out);
  }
  .post-item:hover {
    background: var(--glass-hover);
    border-color: rgba(255, 255, 255, 0.12);
  }
  .post-title {
    font-size: 14px;
    font-weight: 500;
    margin-bottom: 6px;
  }
  .post-meta {
    display: flex;
    gap: 12px;
    font-size: 12px;
    color: var(--text-secondary);
  }
  .status {
    text-align: center;
    padding: 40px 0;
    color: var(--text-secondary);
  }
  .status.error {
    color: var(--danger);
  }
  .load-more {
    display: block;
    margin: 16px auto;
    padding: 8px 24px;
    border-radius: 10px;
    background: var(--glass-bg);
    border: 0.5px solid var(--glass-border);
    color: var(--text);
    font-size: 13px;
    transition: all var(--duration-fast) var(--ease-out);
  }
  .load-more:hover:not(:disabled) {
    background: var(--glass-hover);
    border-color: rgba(255, 255, 255, 0.12);
  }
  .load-more:disabled {
    opacity: 0.5;
  }
</style>
