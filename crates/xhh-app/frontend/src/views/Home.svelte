<script lang="ts">
  import { onMount } from "svelte";
  import { feedsList, likePost, search, type SearchReq } from "../lib/api";
  import { setView, setSelectedLinkId, getFeedsCache, setFeedsCache, isFeedsCached, getHomeScrollTop, setHomeScrollTop } from "../lib/stores.svelte";
  import PostCard from "../components/PostCard.svelte";

  type Mode = "feeds" | "search";
  type Classified = { kind: "user" | "post" | "topic" | "game"; info: any };

  let feeds = $state<any[]>(getFeedsCache());
  let loadingFeeds = $state(!isFeedsCached());
  let refreshing = $state(false);
  let feedError = $state("");
  let currentPage = $state(1);
  let hasMore = $state(true);
  let loadingMore = $state(false);

  let mode = $state<Mode>("feeds");
  let keyword = $state("");
  let searchType = $state("综合");
  let rawItems = $state<any[]>([]);
  let loadingSearch = $state(false);
  let searchError = $state("");

  let sentinel: HTMLElement | undefined = $state();
  let feedObserver: IntersectionObserver | null = null;
  let prevSentinel: HTMLElement | undefined;

  $effect(() => {
    if (prevSentinel) { feedObserver?.unobserve(prevSentinel); prevSentinel = undefined; }
    if (!sentinel) return;
    if (!feedObserver) {
      const scroller = document.querySelector(".content") as HTMLElement | null;
      feedObserver = new IntersectionObserver(
        (entries) => { if (entries[0]?.isIntersecting && mode === "feeds") loadMore(); },
        { root: scroller, rootMargin: "200px" }
      );
    }
    feedObserver.observe(sentinel);
    prevSentinel = sentinel;
  });

  async function loadFeeds(silent = false) {
    if (silent) {
      refreshing = true;
      feedError = "";
      currentPage = 1;
      hasMore = true;
    } else {
      loadingFeeds = true;
      feedError = "";
      currentPage = 1;
      hasMore = true;
    }
    try {
      const v = await feedsList(1, 20);
      feeds = v?.result?.links ?? [];
      setFeedsCache(feeds);
      currentPage = 1;
      if (feeds.length === 0) hasMore = false;
    } catch (e) {
      if (!silent) feedError = String(e);
    } finally {
      loadingFeeds = false;
      refreshing = false;
    }
  }

  async function loadMore() {
    if (loadingMore || !hasMore) return;
    loadingMore = true;
    const nextPage = currentPage + 1;
    try {
      const v = await feedsList(nextPage, 20);
      const newLinks = v?.result?.links ?? [];
      if (newLinks.length === 0) {
        hasMore = false;
      } else {
        feeds = [...feeds, ...newLinks];
        currentPage = nextPage;
        setFeedsCache(feeds);
      }
    } catch (e) {
      console.error("load more failed:", e);
    } finally {
      loadingMore = false;
    }
  }

  async function doSearch() {
    if (!keyword.trim() || loadingSearch) return;
    mode = "search";
    loadingSearch = true;
    searchError = "";
    try {
      const req: SearchReq = { q: keyword, search_type: searchType, limit: 20 };
      const v = await search(req);
      rawItems = v?.result?.items ?? [];
    } catch (e) {
      searchError = String(e);
      rawItems = [];
    } finally {
      loadingSearch = false;
    }
  }

  function exitSearch() {
    mode = "feeds";
    keyword = "";
    rawItems = [];
  }

  function classify(item: any): Classified | null {
    const info = item?.info ?? {};
    if (item?.type === "space") return null;
    if (item?.type === "user" || (info.userid && !info.linkid)) {
      return { kind: "user", info };
    }
    if (item?.type === "topic" || item?.type === "hashtag" || info.topic_id) {
      return { kind: "topic", info };
    }
    if (item?.type === "game" || info.appid) return { kind: "game", info };
    if (info.linkid || info.title) return { kind: "post", info };
    return null;
  }

  let results = $derived(rawItems.map(classify).filter(Boolean) as Classified[]);

  async function handleLike(linkId: string, post: any) {
    const wasLiked = post?.is_award_link === 1 || post?.is_award_link === true;
    try {
      await likePost(linkId, wasLiked ? 0 : 1);
      post.is_award_link = wasLiked ? 0 : 1;
      post.link_award_num = (post.link_award_num ?? 0) + (wasLiked ? -1 : 1);
      setFeedsCache(feeds);
    } catch (e) {
      console.error(e);
    }
  }

  function openDetail(linkId: string) {
    const scroller = document.querySelector(".content") as HTMLElement | null;
    if (scroller) setHomeScrollTop(scroller.scrollTop);
    setSelectedLinkId(linkId);
    setView("detail");
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Enter") doSearch();
  }

  onMount(() => {
    if (!isFeedsCached()) {
      loadFeeds();
    } else {
      const saved = getHomeScrollTop();
      requestAnimationFrame(() => {
        const scroller = document.querySelector(".content") as HTMLElement | null;
        if (scroller) scroller.scrollTop = saved;
      });
    }

    return () => { feedObserver?.disconnect(); feedObserver = null; };
  });
</script>

<div class="home">
  <div class="search-bar" role="search">
    {#if mode === "search"}
      <button class="back-btn" onclick={exitSearch} aria-label="返回信息流">
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M19 12H5"/><path d="m12 19-7-7 7-7"/></svg>
        返回
      </button>
    {/if}
    <div class="search-input-wrap">
      <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><circle cx="11" cy="11" r="8"/><path d="m21 21-4.35-4.35"/></svg>
      <input
        type="text"
        bind:value={keyword}
        onkeydown={handleKeydown}
        placeholder="搜索帖子、用户、游戏..."
        class="input"
        aria-label="搜索关键词"
      />
    </div>
    <select bind:value={searchType} class="type-select" aria-label="搜索类型">
      <option value="综合">综合</option>
      <option value="内容">内容</option>
      <option value="用户">用户</option>
      <option value="游戏">游戏</option>
      <option value="话题">话题</option>
    </select>
    <button class="btn" onclick={doSearch} disabled={loadingSearch}>
      {loadingSearch ? "搜索中" : "搜索"}
    </button>
    {#if mode === "feeds"}
      <button class="refresh-btn" onclick={() => loadFeeds(true)} disabled={refreshing}>{refreshing ? "刷新中" : "刷新"}</button>
    {/if}
    <div class="inline-metrics" aria-label="当前数据概览">
      <div class="inline-metric">
        <span>{feeds.length}</span>
        <em>已加载</em>
      </div>
      <div class="inline-metric">
        <span>{currentPage}</span>
        <em>页</em>
      </div>
      <div class="inline-metric warm">
        <span>{results.length}</span>
        <em>结果</em>
      </div>
    </div>
  </div>

  {#if mode === "feeds"}
    {#if loadingFeeds}
      <div class="state-card loading" role="status" aria-live="polite">
        <span class="loading-ring"></span>
        <div>
          <strong>正在加载信息流</strong>
          <p>同步最新社区内容与互动数据</p>
        </div>
      </div>
    {:else if feedError}
      <div class="state-card error" role="alert">{feedError}</div>
    {:else if feeds.length === 0}
      <div class="state-card">暂无帖子</div>
    {:else}
      <div class="list">
        {#each feeds as post}
          <PostCard {post} onLike={() => handleLike(String(post.linkid), post)} onOpen={() => openDetail(String(post.linkid))} />
        {/each}
        <div bind:this={sentinel} class="sentinel"></div>
        {#if loadingMore}
          <div class="status small">加载更多...</div>
        {/if}
      </div>
    {/if}
  {:else if searchError}
    <div class="state-card error" role="alert">{searchError}</div>
  {:else if loadingSearch}
    <div class="state-card loading" role="status" aria-live="polite">
      <span class="loading-ring"></span>
      <div>
        <strong>正在搜索</strong>
        <p>匹配帖子、用户、游戏与话题</p>
      </div>
    </div>
  {:else if results.length === 0}
    <div class="state-card">无搜索结果</div>
  {:else}
    <div class="list">
      {#each results as r}
        {#if r.kind === "user"}
          <div class="result-item user-item">
            {#if r.info.avatar}
              <img src={r.info.avatar} alt="" class="user-avatar" />
            {:else}
              <div class="user-avatar placeholder" aria-hidden="true">{r.info.username?.charAt(0) ?? "?"}</div>
            {/if}
            <div class="user-meta">
              <div class="user-name">{r.info.username ?? "未知用户"}</div>
              <div class="user-sub">ID: {r.info.userid}{#if r.info.rec_tag} · {r.info.rec_tag}{/if}</div>
            </div>
          </div>
        {:else if r.kind === "post"}
          <PostCard post={r.info} onOpen={() => openDetail(String(r.info.linkid))} />
        {:else if r.kind === "topic"}
          <div class="result-item topic-item">
            <span class="badge">话题</span>
            <span class="topic-name">{r.info.name ?? r.info.hashtag ?? "(未命名)"}</span>
            {#if r.info.description}<span class="topic-desc">{r.info.description}</span>{/if}
            {#if r.info.topic_id}<span class="muted">topic_id: {r.info.topic_id}</span>{/if}
          </div>
        {:else if r.kind === "game"}
          <div class="result-item game-item">
            <span class="badge warm">游戏</span>
            <span>{r.info.name ?? r.info.title}</span>
          </div>
        {/if}
      {/each}
    </div>
  {/if}
</div>

<style>
  .home {
    width: min(980px, 100%);
    margin: 0 auto;
  }

  .search-bar {
    display: flex;
    gap: 10px;
    margin-bottom: 18px;
    position: sticky;
    top: 0;
    z-index: 10;
    padding: 10px;
    border-radius: 24px;
    background: color-mix(in srgb, var(--bg) 76%, transparent);
    backdrop-filter: blur(28px) saturate(1.4);
    -webkit-backdrop-filter: blur(28px) saturate(1.4);
    border: 1px solid rgba(148, 163, 184, 0.16);
    box-shadow: var(--elevation-1);
  }

  .search-input-wrap {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: center;
    gap: 10px;
    min-height: 44px;
    padding: 0 14px;
    border-radius: 16px;
    background: color-mix(in srgb, var(--bg-soft) 72%, transparent);
    border: 1px solid rgba(148, 163, 184, 0.16);
    color: var(--text-muted);
    transition: border-color var(--duration-fast) var(--ease-out), box-shadow var(--duration-fast) var(--ease-out), background var(--duration-fast) var(--ease-out);
  }

  .search-input-wrap:focus-within {
    background: color-mix(in srgb, var(--bg-soft) 88%, transparent);
    border-color: color-mix(in srgb, var(--accent-hover) 42%, transparent);
    box-shadow: 0 0 0 4px color-mix(in srgb, var(--accent) 14%, transparent);
  }

  .input {
    width: 100%;
    min-width: 0;
    border: 0;
    outline: none;
    background: transparent;
    color: var(--text-strong);
    font-size: 14px;
  }

  .type-select,
  .back-btn,
  .btn,
  .refresh-btn {
    min-height: 44px;
    border-radius: 16px;
    font-size: 13px;
    font-weight: 750;
    transition: transform var(--duration-fast) var(--ease-out), background var(--duration-fast) var(--ease-out), border-color var(--duration-fast) var(--ease-out), filter var(--duration-fast) var(--ease-out);
  }

  .back-btn {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    padding: 0 14px;
    background: rgba(148, 163, 184, 0.1);
    color: var(--text-secondary);
    border: 1px solid rgba(148, 163, 184, 0.16);
  }

  .type-select {
    padding: 0 12px;
    background: color-mix(in srgb, var(--bg-soft) 72%, transparent);
    color: var(--text);
    border: 1px solid rgba(148, 163, 184, 0.16);
    outline: none;
  }

  .btn {
    padding: 0 22px;
    background: linear-gradient(135deg, var(--accent), var(--accent-strong));
    color: white;
    box-shadow: 0 14px 30px color-mix(in srgb, var(--accent-strong) 26%, transparent);
  }

  .refresh-btn {
    padding: 0 16px;
    background: rgba(148, 163, 184, 0.1);
    color: var(--text-secondary);
    border: 1px solid rgba(148, 163, 184, 0.16);
  }

  .inline-metrics {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    padding-left: 2px;
  }

  .inline-metric {
    min-width: 70px;
    min-height: 44px;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 5px 10px;
    border-radius: 15px;
    background: color-mix(in srgb, var(--bg-soft) 64%, transparent);
    border: 1px solid rgba(148, 163, 184, 0.14);
  }

  .inline-metric.warm {
    background: color-mix(in srgb, var(--accent-warm) 10%, transparent);
    border-color: color-mix(in srgb, var(--accent-warm) 18%, transparent);
  }

  .inline-metric span {
    color: var(--text-strong);
    font-size: 16px;
    font-weight: 850;
    line-height: 1;
    font-variant-numeric: tabular-nums;
  }

  .inline-metric em {
    margin-top: 4px;
    color: var(--text-muted);
    font-size: 11px;
    font-style: normal;
    white-space: nowrap;
  }

  .back-btn:hover:not(:disabled),
  .refresh-btn:hover:not(:disabled),
  .type-select:hover {
    background: rgba(148, 163, 184, 0.16);
    color: var(--text-strong);
  }

  .btn:hover:not(:disabled) {
    filter: brightness(1.08);
    transform: translateY(-1px);
  }

  .back-btn:active,
  .btn:active,
  .refresh-btn:active {
    transform: scale(0.97);
  }

  .btn:disabled,
  .refresh-btn:disabled {
    opacity: 0.52;
  }

  .list {
    display: flex;
    flex-direction: column;
    gap: 14px;
  }

  .result-item,
  .state-card {
    padding: 18px;
    border-radius: var(--radius-lg);
    background: linear-gradient(180deg, color-mix(in srgb, var(--bg-soft) 76%, transparent), color-mix(in srgb, var(--bg-soft) 57%, transparent));
    border: 1px solid var(--glass-border);
    box-shadow: var(--elevation-1);
    font-size: 14px;
    transition: all var(--duration-normal) var(--ease-out);
  }

  .result-item:hover {
    transform: translateY(-1px);
    border-color: color-mix(in srgb, var(--accent-hover) 28%, transparent);
    box-shadow: var(--elevation-2);
  }

  .state-card {
    min-height: 120px;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 14px;
    color: var(--text-secondary);
    text-align: center;
  }

  .state-card strong {
    display: block;
    color: var(--text-strong);
    font-size: 15px;
  }

  .state-card p {
    margin-top: 4px;
    font-size: 13px;
  }

  .state-card.error {
    justify-content: flex-start;
    color: var(--danger-fg);
    background: var(--danger-soft);
    border-color: rgba(248, 113, 113, 0.24);
    text-align: left;
  }

  .loading-ring {
    width: 32px;
    height: 32px;
    border-radius: 50%;
    border: 3px solid rgba(148, 163, 184, 0.2);
    border-top-color: var(--accent);
    animation: spin 900ms linear infinite;
    flex-shrink: 0;
  }

  .user-item {
    display: flex;
    align-items: center;
    gap: 14px;
  }

  .user-avatar {
    width: 48px;
    height: 48px;
    border-radius: 16px;
    object-fit: cover;
    flex-shrink: 0;
  }

  .user-avatar.placeholder {
    display: flex;
    align-items: center;
    justify-content: center;
    background: linear-gradient(135deg, var(--accent), var(--accent-warm));
    color: white;
    font-weight: 800;
  }

  .user-meta {
    min-width: 0;
    overflow: hidden;
  }

  .user-name {
    font-size: 15px;
    font-weight: 800;
    color: var(--text-strong);
  }

  .user-sub {
    font-size: 12px;
    color: var(--text-muted);
    margin-top: 4px;
  }

  .badge {
    display: inline-flex;
    align-items: center;
    min-height: 26px;
    padding: 4px 10px;
    border-radius: 999px;
    background: var(--accent-soft);
    color: var(--on-accent-soft);
    border: 1px solid color-mix(in srgb, var(--accent-hover) 18%, transparent);
    font-size: 12px;
    font-weight: 800;
    margin-right: 8px;
  }

  .badge.warm {
    background: var(--accent-warm-soft);
    color: var(--warning-fg);
    border-color: color-mix(in srgb, var(--accent-warm) 24%, transparent);
  }

  .topic-item,
  .game-item {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 8px;
  }

  .topic-name {
    font-weight: 800;
    color: var(--text-strong);
  }

  .topic-desc,
  .muted {
    font-size: 12px;
    color: var(--text-muted);
  }

  .status {
    text-align: center;
    padding: 30px 0;
    color: var(--text-secondary);
  }

  .status.small {
    padding: 16px 0;
    font-size: 13px;
  }

  .sentinel {
    height: 1px;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }

  @media (max-width: 960px) {
    .search-bar {
      flex-wrap: wrap;
    }

    .inline-metrics {
      width: 100%;
      justify-content: flex-end;
    }
  }

  @media (max-width: 720px) {
    .search-bar {
      display: grid;
      grid-template-columns: 1fr;
    }

    .type-select,
    .btn,
    .refresh-btn,
    .back-btn,
    .inline-metrics {
      width: 100%;
    }

    .inline-metrics {
      justify-content: stretch;
    }

    .inline-metric {
      flex: 1;
    }
  }
</style>
