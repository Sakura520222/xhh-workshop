<script lang="ts">
  import { search, type SearchReq } from "../lib/api";
  import PostCard from "../components/PostCard.svelte";

  type Classified = { kind: "user" | "post" | "topic" | "hashtag" | "game"; info: any };

  let keyword = $state("");
  let rawItems = $state<any[]>([]);
  let loading = $state(false);
  let searched = $state(false);
  let errorMsg = $state("");
  let searchType = $state("综合");

  async function doSearch() {
    if (!keyword.trim() || loading) return;
    loading = true;
    errorMsg = "";
    searched = true;
    try {
      const req: SearchReq = { q: keyword, search_type: searchType, limit: 20 };
      console.log("[search] sending:", JSON.stringify(req));
      const v = await search(req);
      console.log("[search] response:", JSON.stringify(v).slice(0, 500));
      rawItems = v?.result?.items ?? [];
      console.log("[search] rawItems count:", rawItems.length, "types:", rawItems.map((i: any) => i?.type));
    } catch (e) {
      errorMsg = String(e);
      console.error("[search] error:", e);
      rawItems = [];
    } finally {
      loading = false;
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Enter") doSearch();
  }

  // 按 info 内容分类，过滤占位符（space）和无法识别的项
  function classify(item: any): Classified | null {
    const info = item?.info ?? {};
    if (item?.type === "space") return null;
    if (item?.type === "user" || (info.userid && !info.linkid)) {
      return { kind: "user", info };
    }
    if (item?.type === "topic" || item?.type === "hashtag" || info.topic_id) return { kind: "topic", info };
    if (item?.type === "game" || info.appid) return { kind: "game", info };
    if (info.linkid || info.title) return { kind: "post", info };
    return null;
  }

  let results = $derived(rawItems.map(classify).filter(Boolean) as Classified[]);
</script>

<div class="search-page">
  <div class="search-bar">
    <input
      type="text"
      bind:value={keyword}
      onkeydown={handleKeydown}
      placeholder="搜索帖子、用户、游戏..."
      class="input"
    />
    <select bind:value={searchType} class="type-select">
      <option value="综合">综合</option>
      <option value="内容">内容</option>
      <option value="用户">用户</option>
      <option value="游戏">游戏</option>
      <option value="话题">话题</option>
    </select>
    <button class="btn" onclick={doSearch} disabled={loading}>搜索</button>
  </div>

  {#if errorMsg}
    <div class="status error">{errorMsg}</div>
  {/if}

  {#if loading}
    <div class="status">搜索中...</div>
  {:else if searched && results.length === 0}
    <div class="status">无搜索结果</div>
  {:else}
    <div class="results">
      {#each results as r}
        {#if r.kind === "user"}
          <div class="result-item user-item">
            {#if r.info.avatar}
              <img src={r.info.avatar} alt="" class="user-avatar" />
            {:else}
              <div class="user-avatar placeholder">{r.info.username?.charAt(0) ?? "?"}</div>
            {/if}
            <div class="user-meta">
              <div class="user-name">{r.info.username ?? "未知用户"}</div>
              <div class="user-sub">ID: {r.info.userid}{#if r.info.rec_tag} · {r.info.rec_tag}{/if}</div>
            </div>
          </div>
        {:else if r.kind === "post"}
          <PostCard post={r.info} />
        {:else if r.kind === "topic" || r.kind === "hashtag"}
          <div class="result-item">
            <span class="badge">{r.kind === "hashtag" ? "话题" : "社区"}</span>
            {r.info.name} {#if r.info.description}· {r.info.description}{/if} {#if r.info.sub_title}<span class="badge">{r.info.sub_title}</span>{/if}
          </div>
        {:else if r.kind === "game"}
          <div class="result-item">
            <span class="badge">游戏</span>
            {r.info.name ?? r.info.title}
          </div>
        {/if}
      {/each}
    </div>
  {/if}
</div>

<style>
  .search-page {
    max-width: 720px;
    margin: 0 auto;
  }
  .search-bar {
    display: flex;
    gap: 8px;
    margin-bottom: 20px;
  }
  .input {
    flex: 1;
    padding: 10px 14px;
    border-radius: 14px;
    background: var(--fill);
    color: var(--text);
    border: 0.5px solid var(--glass-border);
    font-size: 14px;
    outline: none;
    transition: all var(--duration-fast) var(--ease-out);
  }
  .input:focus {
    border-color: var(--accent);
    box-shadow: 0 0 0 3px rgba(255, 107, 53, 0.12);
  }
  .type-select {
    padding: 10px 12px;
    border-radius: 14px;
    background: var(--fill);
    color: var(--text);
    border: 0.5px solid var(--glass-border);
    font-size: 13px;
    outline: none;
    transition: all var(--duration-fast) var(--ease-out);
  }
  .type-select:focus {
    border-color: var(--accent);
  }
  .type-select option {
    background: #fff;
    color: #222;
  }
  .btn {
    padding: 10px 20px;
    border-radius: 14px;
    background: var(--accent);
    color: white;
    font-size: 14px;
    box-shadow: 0 2px 8px rgba(255, 107, 53, 0.3);
    transition: all var(--duration-fast) var(--ease-out);
  }
  .btn:hover:not(:disabled) {
    background: var(--accent-hover);
  }
  .btn:disabled {
    opacity: 0.5;
  }
  .results {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .result-item {
    padding: 14px 16px;
    border-radius: var(--radius);
    background: var(--glass-bg);
    border: 0.5px solid var(--glass-border);
    box-shadow: var(--elevation-1);
    font-size: 14px;
    transition: all var(--duration-normal) var(--ease-out);
  }
  .result-item:hover {
    background: var(--glass-hover);
    border-color: rgba(255, 255, 255, 0.12);
  }
  .user-item {
    display: flex;
    align-items: center;
    gap: 12px;
  }
  .user-avatar {
    width: 44px;
    height: 44px;
    border-radius: 50%;
    object-fit: cover;
  }
  .user-avatar.placeholder {
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--accent);
    color: white;
    font-weight: 600;
  }
  .user-meta {
    overflow: hidden;
  }
  .user-name {
    font-size: 14px;
    font-weight: 500;
  }
  .user-sub {
    font-size: 12px;
    color: var(--text-secondary);
    margin-top: 2px;
  }
  .badge {
    display: inline-block;
    padding: 2px 8px;
    border-radius: 6px;
    background: var(--accent);
    color: white;
    font-size: 11px;
    margin-right: 8px;
  }
  .status {
    text-align: center;
    padding: 40px 0;
    color: var(--text-secondary);
  }
  .status.error {
    color: var(--danger);
  }
</style>
