<script lang="ts">
  import { onMount } from "svelte";
  import { favourFolders, favourFolder } from "../lib/api";
  import { setSelectedLinkId, setView } from "../lib/stores.svelte";

  let folders = $state<any[]>([]);
  let loading = $state(true);
  let error = $state("");
  // 收藏夹内帖子视图
  let selectedFolder = $state<any>(null);
  let posts = $state<any[]>([]);
  let postsLoading = $state(false);
  let postsOffset = $state(0);
  let postsHasMore = $state(true);
  let showAllFav = $state(false);
  let allFavPosts = $state<any[]>([]);
  let allFavOffset = $state(0);
  let allFavHasMore = $state(true);
  let allFavLoading = $state(false);

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
        (entries) => {
          if (entries[0]?.isIntersecting) {
            if (selectedFolder) loadMorePosts();
            else if (showAllFav) loadMoreAllFav();
          }
        },
        { root: scroller, rootMargin: "200px" }
      );
    }
    observer.observe(sentinel);
    prevSentinel = sentinel;
  });

  async function loadFolders() {
    loading = true;
    error = "";
    try {
      const v = await favourFolders();
      folders = v?.result?.folders ?? [];
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  async function openFolder(folder: any) {
    selectedFolder = folder;
    postsOffset = 0;
    postsHasMore = true;
    postsLoading = true;
    posts = [];
    error = "";
    try {
      const v = await favourFolder(String(folder.id), 0, 30);
      const raw = v?.result?.links ?? [];
      posts = raw.map((item: any) => item.link);
      postsOffset = raw.length;
      if (posts.length < 30) postsHasMore = false;
    } catch (e) {
      error = String(e);
    } finally {
      postsLoading = false;
    }
  }

  async function loadMorePosts() {
    if (postsLoading || !postsHasMore || !selectedFolder) return;
    postsLoading = true;
    try {
      const v = await favourFolder(String(selectedFolder.id), postsOffset, 30);
      const raw = v?.result?.links ?? [];
      const newPosts = raw.map((item: any) => item.link);
      if (newPosts.length === 0) {
        postsHasMore = false;
      } else {
        posts = [...posts, ...newPosts];
        postsOffset += newPosts.length;
        if (newPosts.length < 30) postsHasMore = false;
      }
    } catch (e) {
      console.warn("[favourites] load more failed:", e);
      postsHasMore = false;
    } finally {
      postsLoading = false;
    }
  }

  async function openAllFav() {
    showAllFav = true;
    allFavOffset = 0;
    allFavHasMore = true;
    allFavLoading = true;
    allFavPosts = [];
    error = "";
    try {
      const v = await favourFolder(undefined, 0, 30);
      const raw = v?.result?.links ?? [];
      allFavPosts = raw.map((item: any) => item.link);
      allFavOffset = raw.length;
      if (allFavPosts.length < 30) allFavHasMore = false;
    } catch (e) {
      error = String(e);
    } finally {
      allFavLoading = false;
    }
  }

  async function loadMoreAllFav() {
    if (allFavLoading || !allFavHasMore) return;
    allFavLoading = true;
    try {
      const v = await favourFolder(undefined, allFavOffset, 30);
      const raw = v?.result?.links ?? [];
      const newPosts = raw.map((item: any) => item.link);
      if (newPosts.length === 0) {
        allFavHasMore = false;
      } else {
        allFavPosts = [...allFavPosts, ...newPosts];
        allFavOffset += newPosts.length;
        if (newPosts.length < 30) allFavHasMore = false;
      }
    } catch (e) {
      console.warn("[favourites] load more all failed:", e);
      allFavHasMore = false;
    } finally {
      allFavLoading = false;
    }
  }

  function openPost(linkId: string | number) {
    if (!linkId) return;
    setSelectedLinkId(String(linkId));
    setView("detail");
  }

  function back() {
    selectedFolder = null;
    showAllFav = false;
    posts = [];
    postsOffset = 0;
    postsHasMore = true;
  }

  function fmtTime(ts: any): string {
    const n = Number(ts);
    if (!n) return "";
    const d = new Date(n * 1000);
    const mm = String(d.getMonth() + 1).padStart(2, "0");
    const dd = String(d.getDate()).padStart(2, "0");
    const hh = String(d.getHours()).padStart(2, "0");
    const mi = String(d.getMinutes()).padStart(2, "0");
    return `${mm}/${dd} ${hh}:${mi}`;
  }

  onMount(() => {
    loadFolders();
    return () => { observer?.disconnect(); observer = null; };
  });
</script>

<div class="fav-page">
  <div class="topbar">
    {#if selectedFolder || showAllFav}
      <button class="back-btn" onclick={back}>返回</button>
      <span class="topbar-title">{selectedFolder ? selectedFolder.name : '全部收藏'}</span>
    {:else}
      <span class="topbar-title">收藏</span>
      <button class="refresh-btn" onclick={loadFolders} disabled={loading}>刷新</button>
    {/if}
  </div>

  {#if loading}
    <div class="status">加载中...</div>
  {:else if error}
    <div class="status error">{error}</div>
  {:else if !selectedFolder && !showAllFav}
    <button class="all-fav-btn" onclick={openAllFav}>查看全部收藏内容</button>
    {#if folders.length === 0}
      <div class="status">暂无收藏夹</div>
    {:else}
      <div class="folder-list">
        {#each folders as f}
          <button class="folder-item" onclick={() => openFolder(f)}>
            <div class="folder-info">
              <div class="folder-name">{f.name}</div>
              <div class="folder-count">{f.count ?? 0} 条</div>
            </div>
            <span class="arrow">&rarr;</span>
          </button>
        {/each}
      </div>
    {/if}
  {:else if selectedFolder}
    {#if postsLoading && posts.length === 0}
      <div class="status">加载中...</div>
    {:else if posts.length === 0}
      <div class="status">暂无帖子</div>
    {:else}
      <div class="post-list">
        {#each posts as p}
          <div
            class="post-item"
            role="button"
            tabindex="0"
            onclick={() => openPost(p.linkid)}
            onkeydown={(e) => { if (e.key === "Enter" || e.key === " ") { e.preventDefault(); openPost(p.linkid); } }}
          >
            <div class="post-title">{p.title || "(无标题)"}</div>
            <div class="post-footer">
              <div class="post-meta">
                {#if p.user?.username}
                  <span>{p.user.username}</span>
                {/if}
                <span>{p.comment_num ?? 0} 评论</span>
                <span>{p.link_award_num ?? 0} 赞</span>
              </div>
              {#if p.create_at}
                <span class="post-time">{fmtTime(p.create_at)}</span>
              {/if}
            </div>
          </div>
        {/each}
        {#if postsHasMore}
          <div bind:this={sentinel} class="sentinel"></div>
        {/if}
        {#if postsLoading}
          <div class="status small">加载更多...</div>
        {:else if !postsHasMore && posts.length > 0}
          <div class="status small">已加载全部</div>
        {/if}
      </div>
    {/if}
  {:else if showAllFav}
    {#if allFavLoading && allFavPosts.length === 0}
      <div class="status">加载中...</div>
    {:else if allFavPosts.length === 0}
      <div class="status">暂无收藏</div>
    {:else}
      <div class="post-list">
        {#each allFavPosts as p}
          <div
            class="post-item"
            role="button"
            tabindex="0"
            onclick={() => openPost(p.linkid)}
            onkeydown={(e) => { if (e.key === "Enter" || e.key === " ") { e.preventDefault(); openPost(p.linkid); } }}
          >
            <div class="post-title">{p.title || "(无标题)"}</div>
            <div class="post-footer">
              <div class="post-meta">
                {#if p.user?.username}
                  <span>{p.user.username}</span>
                {/if}
                <span>{p.comment_num ?? 0} 评论</span>
                <span>{p.link_award_num ?? 0} 赞</span>
              </div>
              {#if p.create_at}
                <span class="post-time">{fmtTime(p.create_at)}</span>
              {/if}
            </div>
          </div>
        {/each}
        {#if allFavHasMore}
          <div bind:this={sentinel} class="sentinel"></div>
        {/if}
        {#if allFavLoading}
          <div class="status small">加载更多...</div>
        {:else if !allFavHasMore && allFavPosts.length > 0}
          <div class="status small">已加载全部</div>
        {/if}
      </div>
    {/if}
  {/if}
</div>

<style>
  .fav-page {
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
  .back-btn {
    padding: 6px 14px;
    border-radius: 14px;
    background: rgba(255, 255, 255, 0.12);
    border: 0.5px solid rgba(255, 255, 255, 0.2);
    color: var(--text);
    font-size: 13px;
    box-shadow: inset 0 0.5px 0 rgba(255, 255, 255, 0.3);
    transition: all var(--duration-fast) var(--ease-out);
  }
  .back-btn:hover {
    background: rgba(255, 255, 255, 0.2);
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
  .all-fav-btn {
    display: block;
    width: 100%;
    padding: 14px 18px;
    margin-bottom: 16px;
    border-radius: var(--radius);
    background: var(--accent);
    color: white;
    font-size: 14px;
    font-weight: 500;
    box-shadow: 0 2px 8px rgba(255, 107, 53, 0.3);
    transition: all var(--duration-fast) var(--ease-out);
  }
  .all-fav-btn:hover {
    opacity: 0.9;
  }
  .folder-list {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .folder-item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 16px 18px;
    border-radius: var(--radius);
    background: var(--glass-bg);
    border: 0.5px solid var(--glass-border);
    box-shadow: var(--elevation-1);
    transition: all var(--duration-normal) var(--ease-out);
  }
  .folder-item:hover {
    background: var(--glass-hover);
    border-color: rgba(255, 255, 255, 0.12);
  }
  .folder-info {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .folder-name {
    font-size: 15px;
    font-weight: 500;
  }
  .folder-count {
    font-size: 13px;
    color: var(--text-secondary);
  }
  .arrow {
    font-size: 16px;
    color: var(--text-secondary);
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
    line-height: 1.4;
    margin-bottom: 8px;
    overflow: hidden;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
  }
  .post-footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }
  .post-meta {
    display: flex;
    gap: 12px;
    font-size: 12px;
    color: var(--text-secondary);
  }
  .post-time {
    font-size: 12px;
    color: var(--text-secondary);
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
</style>
