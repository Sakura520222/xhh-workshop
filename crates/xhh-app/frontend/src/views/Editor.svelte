<script lang="ts">
  import { postCreate, uploadImage, searchTopic as searchHashtagApi, searchCommunity as searchCommunityApi } from "../lib/api";
  import { setView } from "../lib/stores.svelte";

  let title = $state("");
  let content = $state("");
  let images = $state<{ url: string; width: number; height: number }[]>([]);
  let busy = $state(false);
  let uploading = $state(false);
  let result = $state("");
  let error = $state("");

  let communityKeyword = $state("");
  let communityResults = $state<{ id: string; name: string }[]>([]);
  let selectedCommunity = $state<{ id: string; name: string } | null>(null);

  async function searchCommunity() {
    if (!communityKeyword.trim()) return;
    try {
      const resp = await searchCommunityApi(communityKeyword);
      communityResults = (resp?.result?.search_result ?? [])
        .filter((t: any) => t?.topic_id && t?.name)
        .map((t: any) => ({ id: String(t.topic_id), name: t.name }));
    } catch (e) {
      console.error("community search failed:", e);
    }
  }

  function selectCommunity(c: { id: string; name: string }) {
    selectedCommunity = c;
    communityResults = [];
    communityKeyword = "";
  }

  let hashtagKeyword = $state("");
  let hashtagResults = $state<{ name: string }[]>([]);
  let selectedHashtags = $state<string[]>([]);

  async function searchHashtags() {
    if (!hashtagKeyword.trim()) return;
    try {
      const resp = await searchHashtagApi(hashtagKeyword);
      hashtagResults = (resp?.result?.search_result ?? [])
        .filter((h: any) => h?.name)
        .map((h: any) => ({ name: h.name }));
    } catch (e) {
      console.error("hashtag search failed:", e);
    }
  }

  function addHashtag(name: string) {
    if (!selectedHashtags.includes(name)) {
      selectedHashtags = [...selectedHashtags, name];
    }
    hashtagResults = [];
    hashtagKeyword = "";
  }

  function removeHashtag(name: string) {
    selectedHashtags = selectedHashtags.filter((h) => h !== name);
  }

  async function pickImage() {
    if (uploading) return;
    uploading = true;
    error = "";
    try {
      const result = await uploadImage();
      images = [...images, result];
    } catch (e) {
      error = "图片上传失败: " + String(e);
    } finally {
      uploading = false;
    }
  }

  function removeImage(index: number) {
    images = images.filter((_, i) => i !== index);
  }

  async function submit() {
    if (!title.trim() || !content.trim() || busy) return;
    busy = true;
    error = "";
    result = "";
    try {
      const resp = await postCreate(
        title,
        content,
        selectedHashtags,
        selectedCommunity?.id ?? undefined,
        images.length > 0 ? images : undefined,
      );
      if (resp?.status === "ok") {
        result = "发帖成功";
        title = "";
        content = "";
        images = [];
        selectedCommunity = null;
        selectedHashtags = [];
      } else {
        error = resp?.msg ?? "发帖失败";
      }
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  function back() {
    setView("home");
  }
</script>

<div class="editor-page">
  <div class="page-head">
    <button class="back-btn" onclick={back} aria-label="返回主页">
      <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M19 12H5"/><path d="m12 19-7-7 7-7"/></svg>
      返回
    </button>
    <div>
      <span class="eyebrow">Content Studio</span>
      <h1>发布新内容</h1>
    </div>
    <button class="publish-top" onclick={submit} disabled={busy || !title.trim() || !content.trim()}>
      {busy ? "发布中" : "发布"}
    </button>
  </div>

  <div class="editor-grid">
    <form class="compose-card" onsubmit={(e) => { e.preventDefault(); submit(); }}>
      <label class="field-label" for="post-title">标题</label>
      <input id="post-title" type="text" bind:value={title} placeholder="输入一个清晰、有吸引力的标题" class="title-input" />

      <label class="field-label" for="post-content">正文</label>
      <textarea id="post-content" bind:value={content} placeholder="分享观点、攻略、体验或社区动态..." class="content-input" rows="12"></textarea>

      {#if images.length > 0}
        <div class="image-list" aria-label="已上传图片">
          {#each images as img, i}
            <div class="img-item">
              <img src={img.url} alt={`已上传图片 ${i + 1}`} class="img-thumb" />
              <button class="remove-btn" type="button" onclick={() => removeImage(i)} aria-label={`移除图片 ${i + 1}`}>
                <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.4" stroke-linecap="round" aria-hidden="true"><path d="M18 6 6 18"/><path d="m6 6 12 12"/></svg>
              </button>
            </div>
          {/each}
        </div>
      {/if}

      <div class="compose-actions">
        <button class="tool-btn" type="button" onclick={pickImage} disabled={uploading}>
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><rect x="3" y="3" width="18" height="18" rx="2"/><circle cx="8.5" cy="8.5" r="1.5"/><path d="m21 15-5-5L5 21"/></svg>
          {uploading ? "上传中" : "添加图片"}
        </button>
        <span class="draft-hint">{title.length} 字标题 · {content.length} 字正文</span>
      </div>

      {#if error}
        <div class="msg error" role="alert">{error}</div>
      {/if}
      {#if result}
        <div class="msg success" role="status">{result}</div>
      {/if}
    </form>

    <aside class="side-panel" aria-label="发布设置">
      <section class="section">
        <div class="section-head">
          <span class="section-kicker">Community</span>
          <h2>关联社区</h2>
        </div>
        <div class="search-row">
          <input
            type="text"
            bind:value={communityKeyword}
            onkeydown={(e) => { if (e.key === "Enter") { e.preventDefault(); searchCommunity(); } }}
            placeholder="搜索社区"
            class="search-input"
            aria-label="搜索社区"
          />
          <button class="tool-btn compact" type="button" onclick={searchCommunity}>搜索</button>
        </div>
        {#if selectedCommunity}
          <div class="tag selected">
            <span>{selectedCommunity.name}</span>
            <button class="tag-remove" type="button" onclick={() => (selectedCommunity = null)} aria-label="移除已选社区">
              <svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.4" stroke-linecap="round" aria-hidden="true"><path d="M18 6 6 18"/><path d="m6 6 12 12"/></svg>
            </button>
          </div>
        {/if}
        {#if communityResults.length > 0}
          <div class="results">
            {#each communityResults as c}
              <button class="option" type="button" onclick={() => selectCommunity(c)}>{c.name}</button>
            {/each}
          </div>
        {/if}
      </section>

      <section class="section">
        <div class="section-head">
          <span class="section-kicker">Topics</span>
          <h2>话题标签</h2>
        </div>
        <div class="search-row">
          <input
            type="text"
            bind:value={hashtagKeyword}
            onkeydown={(e) => { if (e.key === "Enter") { e.preventDefault(); searchHashtags(); } }}
            placeholder="搜索标签"
            class="search-input"
            aria-label="搜索标签"
          />
          <button class="tool-btn compact" type="button" onclick={searchHashtags}>搜索</button>
        </div>
        {#if selectedHashtags.length > 0}
          <div class="tag-list">
            {#each selectedHashtags as h}
              <div class="tag">
                <span>#{h}</span>
                <button class="tag-remove" type="button" onclick={() => removeHashtag(h)} aria-label={`移除标签 ${h}`}>
                  <svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.4" stroke-linecap="round" aria-hidden="true"><path d="M18 6 6 18"/><path d="m6 6 12 12"/></svg>
                </button>
              </div>
            {/each}
          </div>
        {/if}
        {#if hashtagResults.length > 0}
          <div class="results">
            {#each hashtagResults as h}
              <button class="option" type="button" onclick={() => addHashtag(h.name)}>#{h.name}</button>
            {/each}
          </div>
        {/if}
      </section>

      <section class="publish-card">
        <span>发布前检查</span>
        <ul>
          <li class:done={!!title.trim()}>标题已填写</li>
          <li class:done={!!content.trim()}>正文已填写</li>
          <li class:done={!!selectedCommunity || selectedHashtags.length > 0}>社区或标签已设置</li>
        </ul>
        <button class="submit-btn" type="button" onclick={submit} disabled={busy || !title.trim() || !content.trim()}>
          {busy ? "发布中" : "确认发布"}
        </button>
      </section>
    </aside>
  </div>
</div>

<style>
  .editor-page {
    width: min(1120px, 100%);
    margin: 0 auto;
  }

  .page-head {
    position: sticky;
    top: 0;
    z-index: 10;
    display: grid;
    grid-template-columns: auto 1fr auto;
    gap: 16px;
    align-items: center;
    padding: 14px;
    margin-bottom: 18px;
    border-radius: 24px;
    background: color-mix(in srgb, var(--bg) 76%, transparent);
    border: 1px solid rgba(148, 163, 184, 0.16);
    box-shadow: var(--elevation-1);
    backdrop-filter: blur(28px) saturate(1.4);
    -webkit-backdrop-filter: blur(28px) saturate(1.4);
  }

  .eyebrow,
  .section-kicker {
    color: var(--accent-hover);
    font-size: 11px;
    font-weight: 850;
    letter-spacing: 0.1em;
    text-transform: uppercase;
  }

  h1 {
    margin-top: 4px;
    font-size: 26px;
    font-weight: 900;
    letter-spacing: -0.04em;
    color: var(--text-strong);
  }

  .back-btn,
  .publish-top,
  .tool-btn,
  .submit-btn,
  .option {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    min-height: 44px;
    border-radius: 15px;
    font-weight: 750;
    transition: transform var(--duration-fast) var(--ease-out), background var(--duration-fast) var(--ease-out), border-color var(--duration-fast) var(--ease-out), filter var(--duration-fast) var(--ease-out);
  }

  .back-btn,
  .tool-btn,
  .option {
    padding: 0 14px;
    background: rgba(148, 163, 184, 0.1);
    color: var(--text-secondary);
    border: 1px solid rgba(148, 163, 184, 0.16);
    font-size: 13px;
  }

  .publish-top,
  .submit-btn {
    padding: 0 22px;
    background: linear-gradient(135deg, var(--accent), var(--accent-strong));
    color: white;
    box-shadow: 0 14px 30px color-mix(in srgb, var(--accent-strong) 26%, transparent);
    font-size: 14px;
  }

  .back-btn:hover:not(:disabled),
  .tool-btn:hover:not(:disabled),
  .option:hover {
    background: rgba(148, 163, 184, 0.16);
    color: var(--text-strong);
  }

  .publish-top:hover:not(:disabled),
  .submit-btn:hover:not(:disabled) {
    filter: brightness(1.08);
    transform: translateY(-1px);
  }

  button:disabled {
    opacity: 0.5;
  }

  .editor-grid {
    display: grid;
    grid-template-columns: minmax(0, 1fr) 340px;
    gap: 18px;
    align-items: start;
  }

  .compose-card,
  .section,
  .publish-card {
    border-radius: 28px;
    background: linear-gradient(180deg, color-mix(in srgb, var(--bg-soft) 76%, transparent), color-mix(in srgb, var(--bg-soft) 57%, transparent));
    border: 1px solid var(--glass-border);
    box-shadow: var(--elevation-1);
    backdrop-filter: var(--glass-blur);
    -webkit-backdrop-filter: var(--glass-blur);
  }

  .compose-card {
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 22px;
  }

  .field-label {
    font-size: 13px;
    font-weight: 750;
    color: var(--text-secondary);
  }

  .title-input,
  .content-input,
  .search-input {
    width: 100%;
    border-radius: 16px;
    background: color-mix(in srgb, var(--bg-soft) 72%, transparent);
    color: var(--text);
    border: 1px solid rgba(148, 163, 184, 0.16);
    outline: none;
    transition: border-color var(--duration-fast) var(--ease-out), box-shadow var(--duration-fast) var(--ease-out), background var(--duration-fast) var(--ease-out);
  }

  .title-input {
    padding: 16px;
    font-size: 24px;
    font-weight: 850;
    letter-spacing: -0.03em;
  }

  .content-input {
    min-height: 320px;
    padding: 16px;
    font-size: 15px;
    line-height: 1.9;
    resize: vertical;
  }

  .search-input {
    min-width: 0;
    min-height: 44px;
    padding: 0 13px;
    font-size: 13px;
  }

  .title-input:focus,
  .content-input:focus,
  .search-input:focus {
    background: color-mix(in srgb, var(--bg-soft) 90%, transparent);
    border-color: color-mix(in srgb, var(--accent-hover) 42%, transparent);
    box-shadow: 0 0 0 4px color-mix(in srgb, var(--accent) 14%, transparent);
  }

  .image-list {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(126px, 1fr));
    gap: 12px;
  }

  .img-item {
    position: relative;
    overflow: hidden;
    border-radius: 18px;
    border: 1px solid rgba(148, 163, 184, 0.16);
  }

  .img-thumb {
    width: 100%;
    height: 112px;
    display: block;
    object-fit: cover;
    background: color-mix(in srgb, var(--bg-soft) 72%, transparent);
  }

  .remove-btn {
    position: absolute;
    top: 8px;
    right: 8px;
    width: 30px;
    height: 30px;
    display: grid;
    place-items: center;
    border-radius: 999px;
    background: color-mix(in srgb, var(--bg) 76%, transparent);
    border: 1px solid rgba(255, 255, 255, 0.14);
    color: var(--text-strong);
    backdrop-filter: blur(10px);
  }

  .compose-actions {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    flex-wrap: wrap;
    padding-top: 4px;
  }

  .draft-hint {
    color: var(--text-muted);
    font-size: 12px;
  }

  .side-panel {
    position: sticky;
    top: 92px;
    display: flex;
    flex-direction: column;
    gap: 14px;
  }

  .section,
  .publish-card {
    padding: 18px;
  }

  .section-head h2 {
    margin-top: 6px;
    font-size: 18px;
    font-weight: 850;
    color: var(--text-strong);
  }

  .search-row {
    display: flex;
    gap: 8px;
    margin-top: 14px;
  }

  .tool-btn.compact {
    min-width: 72px;
  }

  .results,
  .tag-list {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
    margin-top: 12px;
  }

  .tag {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    min-height: 34px;
    padding: 6px 10px;
    border-radius: 999px;
    background: var(--accent-soft);
    color: #bfdbfe;
    border: 1px solid color-mix(in srgb, var(--accent-hover) 18%, transparent);
    font-size: 13px;
    font-weight: 750;
  }

  .tag.selected {
    margin-top: 12px;
  }

  .tag-remove {
    width: 20px;
    height: 20px;
    display: grid;
    place-items: center;
    border-radius: 999px;
    background: color-mix(in srgb, var(--bg-soft) 52%, transparent);
    color: inherit;
  }

  .option {
    min-height: 34px;
    padding: 0 12px;
    border-radius: 999px;
  }

  .publish-card span {
    display: block;
    color: var(--text-strong);
    font-size: 15px;
    font-weight: 850;
  }

  .publish-card ul {
    display: grid;
    gap: 8px;
    margin: 14px 0;
    padding: 0;
    list-style: none;
  }

  .publish-card li {
    color: var(--text-muted);
    font-size: 13px;
  }

  .publish-card li.done {
    color: #bbf7d0;
  }

  .submit-btn {
    width: 100%;
  }

  .msg {
    padding: 12px 14px;
    border-radius: 14px;
    font-size: 13px;
    line-height: 1.5;
  }

  .msg.error {
    background: var(--danger-soft);
    color: #fecaca;
    border: 1px solid rgba(248, 113, 113, 0.22);
  }

  .msg.success {
    background: var(--success-soft);
    color: #bbf7d0;
    border: 1px solid rgba(34, 197, 94, 0.22);
  }

  @media (max-width: 980px) {
    .editor-grid {
      grid-template-columns: 1fr;
    }

    .side-panel {
      position: static;
    }
  }

  @media (max-width: 680px) {
    .page-head {
      grid-template-columns: 1fr;
    }

    .publish-top,
    .back-btn {
      width: 100%;
    }
  }
</style>
