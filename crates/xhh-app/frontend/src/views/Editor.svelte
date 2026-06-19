<script lang="ts">
  import {
    postCreate,
    postEdit,
    postDraft,
    postCreateVideo,
    editInfo,
    draftList,
    deleteDraft,
    uploadImage,
    uploadVideo,
    searchTopic as searchHashtagApi,
    searchCommunity as searchCommunityApi,
    topicIndex,
  } from "../lib/api";
  import { setView, getEditTarget, clearEditTarget } from "../lib/stores.svelte";
  import { toastSuccess, toastError } from "../lib/toast.svelte";

  // 编辑模式：从详情页携带帖子数据进入，存在则走编辑流程
  // 组件创建时快照一次（编辑目标在本次会话不变），editTarget 仅用于切换标题/按钮态
  const initialEdit = getEditTarget();
  let editTarget = $state(initialEdit);
  let title = $state(initialEdit?.title ?? "");
  let content = $state(initialEdit?.content ?? "");
  let images = $state<{ url: string; width: number; height: number }[]>(initialEdit?.images ?? []);
  let busy = $state(false);
  let uploading = $state(false);
  // 图文 / 视频帖模式（编辑模式固定为图文）
  let mode = $state<"article" | "video">("article");
  let videoUrl = $state("");

  let communityKeyword = $state("");
  let communityResults = $state<{ id: string; name: string }[]>([]);
  let selectedCommunity = $state<{ id: string; name: string } | null>(
    initialEdit?.communityId ? { id: initialEdit.communityId, name: initialEdit.communityName ?? "" } : null,
  );

  let hashtagKeyword = $state("");
  let hashtagResults = $state<{ name: string }[]>([]);
  let selectedHashtags = $state<string[]>(initialEdit?.hashtags ?? []);

  // 推荐话题 / 社区（首屏预填，点击即填入关联社区）
  let recommendations = $state<{ topicId?: string; name: string }[]>([]);
  topicIndex()
    .then((resp) => {
      const topicList = resp?.result?.topic_list ?? [];
      recommendations = topicList
        .filter((t: any) => t?.name)
        .slice(0, 8)
        .map((t: any) => ({
          topicId: t.topic_id != null ? String(t.topic_id) : undefined,
          name: String(t.name),
        }));
    })
    .catch(() => {
      // 推荐失败不阻塞发帖
    });

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
    try {
      const r = await uploadImage();
      images = [...images, r];
    } catch (e) {
      toastError("图片上传失败", String(e));
    } finally {
      uploading = false;
    }
  }

  let uploadingVideo = $state(false);
  async function pickVideo() {
    if (uploadingVideo) return;
    uploadingVideo = true;
    try {
      const r = await uploadVideo();
      if (r?.url) videoUrl = r.url;
    } catch (e) {
      toastError("视频上传失败", String(e));
    } finally {
      uploadingVideo = false;
    }
  }

  let cover = $state<{ url: string; width: number; height: number } | null>(null);
  async function pickCover() {
    if (uploading) return;
    uploading = true;
    try {
      cover = await uploadImage();
    } catch (e) {
      toastError("封面上传失败", String(e));
    } finally {
      uploading = false;
    }
  }

  function removeImage(index: number) {
    images = images.filter((_, i) => i !== index);
  }

  function resetForm() {
    title = "";
    content = "";
    images = [];
    selectedCommunity = null;
    selectedHashtags = [];
    videoUrl = "";
    cover = null;
    draftLinkId = null;
  }

  async function submit() {
    if (busy) return;
    if (mode === "video") {
      if (title.trim().length < 6) {
        toastError("标题太短", "视频帖标题不得少于 6 个字");
        return;
      }
      if (!videoUrl.trim()) return;
      if (!cover) {
        toastError("请添加封面", "视频投稿需要封面图");
        return;
      }
      busy = true;
      try {
        const resp = await postCreateVideo(
          title,
          videoUrl,
          cover.url,
          content.trim() || undefined,
          selectedCommunity?.id ?? undefined,
        );
        if (resp?.status === "ok") {
          toastSuccess("视频帖发布成功");
          resetForm();
          mode = "article";
        } else {
          toastError("发布失败", resp?.msg ?? "");
        }
      } catch (e) {
        toastError("发布失败", String(e));
      } finally {
        busy = false;
      }
      return;
    }

    if (!title.trim() || !content.trim()) return;
    busy = true;
    try {
      const resp = editTarget
        ? await postEdit(
            editTarget.linkId,
            title,
            content,
            selectedHashtags,
            selectedCommunity?.id ?? undefined,
            images.length > 0 ? images : undefined,
          )
        : await postCreate(
            title,
            content,
            selectedHashtags,
            selectedCommunity?.id ?? undefined,
            images.length > 0 ? images : undefined,
          );
      if (resp?.status === "ok") {
        toastSuccess(editTarget ? "编辑成功" : "发帖成功");
        if (editTarget) {
          clearEditTarget();
          editTarget = null;
        }
        // 草稿恢复后发布：同步删除原草稿
        if (draftLinkId) {
          try {
            await deleteDraft(draftLinkId);
          } catch { /* 草稿清理失败不阻塞发帖成功 */ }
          draftLinkId = null;
        }
        resetForm();
      } else {
        toastError(editTarget ? "编辑失败" : "发帖失败", resp?.msg ?? "");
      }
    } catch (e) {
      toastError("操作失败", String(e));
    } finally {
      busy = false;
    }
  }

  let drafts = $state<any[]>([]);
  let draftsOpen = $state(false);
  let draftsLoading = $state(false);
  // 从草稿恢复后发布，需同步删除原草稿
  let draftLinkId = $state<string | null>(null);

  async function saveDraft() {
    if (busy || (!title.trim() && !content.trim())) return;
    busy = true;
    try {
      const resp = await postDraft(title, content, selectedCommunity?.id ?? undefined);
      if (resp?.status === "ok") {
        toastSuccess("草稿已保存");
      } else {
        toastError("保存草稿失败", resp?.msg ?? "");
      }
    } catch (e) {
      toastError("保存草稿失败", String(e));
    } finally {
      busy = false;
    }
  }

  async function openDraftsBox() {
    draftsOpen = true;
    draftsLoading = true;
    try {
      const v = await draftList(0, 40);
      drafts = v?.result?.links ?? [];
    } catch (e) {
      toastError("加载草稿箱失败", String(e));
      drafts = [];
    } finally {
      draftsLoading = false;
    }
  }

  function parseBlocks(raw: any): any[] {
    if (typeof raw !== "string" || !raw) return [];
    try {
      const a = JSON.parse(raw);
      return Array.isArray(a) ? a : [];
    } catch {
      return [];
    }
  }

  async function restoreDraftByLinkid(id: string) {
    busy = true;
    try {
      const p = await editInfo(id);
      const link = p?.result?.link;
      if (!link) {
        toastError("恢复草稿失败", "草稿内容为空");
        return;
      }
      draftLinkId = id;
      title = link.title ?? "";
      const blocks = parseBlocks(link.text);
      content = blocks
        .filter((b: any) => b?.type === "text" && b.text)
        .map((b: any) => b.text)
        .join("\n\n");
      images = blocks
        .filter((b: any) => b?.type === "img")
        .map((b: any) => ({ url: b.url ?? b.text ?? "", width: Number(b.width) || 0, height: Number(b.height) || 0 }))
        .filter((b: any) => b.url);
      mode = "article";
      draftsOpen = false;
      toastSuccess("已恢复草稿");
    } catch (e) {
      toastError("恢复草稿失败", String(e));
    } finally {
      busy = false;
    }
  }

  async function deleteDraftFromList(id: string) {
    try {
      const resp = await deleteDraft(id);
      if (resp?.status === "ok") {
        drafts = drafts.filter((d) => String(d.linkid) !== id);
      } else {
        toastError("删除草稿失败", resp?.msg ?? "");
      }
    } catch (e) {
      toastError("删除草稿失败", String(e));
    }
  }

  function back() {
    if (editTarget) {
      clearEditTarget();
      editTarget = null;
    }
    setView("home");
  }
</script>

<div class="editor-page">
  {#if draftsOpen}
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="drafts-overlay" onclick={() => (draftsOpen = false)}>
      <div class="drafts-panel" onclick={(e) => e.stopPropagation()}>
        <div class="drafts-head">
          <span>草稿箱</span>
          <button class="drafts-close" onclick={() => (draftsOpen = false)}>关闭</button>
        </div>
        <div class="drafts-body">
          {#if draftsLoading}
            <div class="drafts-status">加载中...</div>
          {:else if drafts.length === 0}
            <div class="drafts-status">暂无草稿</div>
          {:else}
            {#each drafts as d}
              <div class="draft-item">
                <button class="draft-main" onclick={() => restoreDraftByLinkid(String(d.linkid))}>
                  <div class="draft-title">{d.title || "(无标题)"}</div>
                  {#if d.description}
                    <div class="draft-desc">{d.description}</div>
                  {/if}
                </button>
                <button class="draft-del" aria-label="删除草稿" onclick={() => deleteDraftFromList(String(d.linkid))}>
                  <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
                    <path d="M3 6h18"/>
                    <path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6"/>
                    <path d="M8 6V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"/>
                  </svg>
                </button>
              </div>
            {/each}
          {/if}
        </div>
      </div>
    </div>
  {/if}
  <div class="page-head">
    <button class="back-btn" onclick={back} aria-label="返回主页">
      <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M19 12H5"/><path d="m12 19-7-7 7-7"/></svg>
      返回
    </button>
    <div>
      <span class="eyebrow">{editTarget ? "Editing" : "Content Studio"}</span>
      <h1>{editTarget ? "编辑内容" : "发布新内容"}</h1>
    </div>
    <button class="publish-top" onclick={submit} disabled={busy || (mode === "video" ? (title.trim().length < 6 || !videoUrl.trim() || !cover) : (!title.trim() || !content.trim()))}>
      {busy ? "处理中" : (editTarget ? "保存修改" : "发布")}
    </button>
  </div>

  <div class="editor-grid">
    <form class="compose-card" onsubmit={(e) => { e.preventDefault(); submit(); }}>
      {#if !editTarget}
        <button class="restore-draft" type="button" onclick={openDraftsBox}>草稿箱</button>
      {/if}
      <div class="mode-tabs" role="tablist" aria-label="内容类型">
        <button type="button" class:active={mode === "article"} onclick={() => (mode = "article")} role="tab" aria-selected={mode === "article"}>图文</button>
        <button type="button" class:active={mode === "video"} onclick={() => (mode = "video")} role="tab" aria-selected={mode === "video"} disabled={!!editTarget}>视频帖</button>
      </div>

      <label class="field-label" for="post-title">标题</label>
      <input id="post-title" type="text" bind:value={title} placeholder="输入一个清晰、有吸引力的标题" class="title-input" />

      {#if mode === "video"}
        <label class="field-label" for="post-video">视频地址</label>
        <input id="post-video" type="text" bind:value={videoUrl} placeholder="粘贴视频文件 URL（mp4 等）" class="video-input" />
        <button class="tool-btn" type="button" onclick={pickVideo} disabled={uploadingVideo}>
          {uploadingVideo ? "上传中" : "选择本地视频"}
        </button>
        <label class="field-label">封面（必填）</label>
        <div class="cover-row">
          {#if cover}
            <img src={cover.url} alt="封面" class="cover-preview" />
          {/if}
          <button class="tool-btn" type="button" onclick={pickCover} disabled={uploading}>
            {uploading ? "上传中" : "选择封面"}
          </button>
        </div>
        <label class="field-label" for="post-content">附加文字（可选）</label>
        <textarea id="post-content" bind:value={content} placeholder="为视频补充说明..." class="content-input" rows="6"></textarea>
      {:else}
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
      {/if}

      <div class="compose-actions">
        {#if mode === "article"}
          <div class="action-group">
            <button class="tool-btn" type="button" onclick={pickImage} disabled={uploading}>
              <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><rect x="3" y="3" width="18" height="18" rx="2"/><circle cx="8.5" cy="8.5" r="1.5"/><path d="m21 15-5-5L5 21"/></svg>
              {uploading ? "上传中" : "添加图片"}
            </button>
            {#if !editTarget}
              <button class="tool-btn" type="button" onclick={saveDraft} disabled={busy || (!title.trim() && !content.trim())}>
                存草稿
              </button>
            {/if}
          </div>
        {/if}
        <span class="draft-hint">{mode === "video" ? `${videoUrl.length} 字地址` : `${title.length} 字标题 · ${content.length} 字正文`}</span>
      </div>

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
        {#if recommendations.length > 0}
          <div class="rec-head">推荐社区</div>
          <div class="results">
            {#each recommendations as r}
              {#if r.topicId}
                <button class="option" type="button" onclick={() => selectCommunity({ id: r.topicId as string, name: r.name })}>{r.name}</button>
              {/if}
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
        <button class="submit-btn" type="button" onclick={submit} disabled={busy || (mode === "video" ? (title.trim().length < 6 || !videoUrl.trim() || !cover) : (!title.trim() || !content.trim()))}>
          {busy ? "处理中" : (editTarget ? "保存修改" : "确认发布")}
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

  .restore-draft {
    align-self: flex-start;
    padding: 6px 14px;
    border-radius: 12px;
    background: var(--accent-soft);
    color: var(--on-accent-soft);
    border: 1px solid color-mix(in srgb, var(--accent-hover) 18%, transparent);
    font-size: 13px;
    font-weight: 600;
  }
  .restore-draft:hover {
    background: color-mix(in srgb, var(--accent) 18%, transparent);
  }
  .drafts-overlay {
    position: fixed;
    inset: 0;
    background: var(--scrim);
    z-index: 9998;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 20px;
  }
  .drafts-panel {
    width: 100%;
    max-width: 480px;
    max-height: 80vh;
    display: flex;
    flex-direction: column;
    border-radius: var(--radius);
    background: var(--glass-bg);
    backdrop-filter: var(--glass-blur);
    -webkit-backdrop-filter: var(--glass-blur);
    border: 0.5px solid var(--glass-border);
    box-shadow: var(--elevation-1);
    overflow: hidden;
  }
  .drafts-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 16px 18px;
    font-size: 16px;
    font-weight: 700;
    border-bottom: 0.5px solid var(--glass-border);
  }
  .drafts-close {
    padding: 4px 12px;
    border-radius: 10px;
    background: var(--fill-strong);
    color: var(--text-secondary);
    font-size: 13px;
  }
  .drafts-body {
    flex: 1;
    overflow-y: auto;
    padding: 8px;
  }
  .drafts-status {
    text-align: center;
    padding: 40px 0;
    color: var(--text-secondary);
    font-size: 14px;
  }
  .draft-item {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 12px;
    border-radius: 14px;
    transition: background var(--duration-fast) var(--ease-out);
  }
  .draft-item:hover {
    background: var(--fill);
  }
  .draft-main {
    flex: 1;
    min-width: 0;
    text-align: left;
  }
  .draft-title {
    font-size: 14px;
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .draft-desc {
    margin-top: 4px;
    font-size: 12px;
    color: var(--text-secondary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .draft-del {
    flex-shrink: 0;
    width: 32px;
    height: 32px;
    display: grid;
    place-items: center;
    border-radius: 10px;
    color: var(--text-secondary);
    transition: all var(--duration-fast) var(--ease-out);
  }
  .draft-del:hover {
    background: var(--danger-soft);
    color: var(--danger);
  }

  .mode-tabs {
    display: inline-flex;
    gap: 4px;
    padding: 4px;
    border-radius: 14px;
    background: rgba(148, 163, 184, 0.12);
    border: 1px solid rgba(148, 163, 184, 0.16);
    align-self: flex-start;
  }

  .mode-tabs button {
    min-height: 34px;
    padding: 0 16px;
    border-radius: 10px;
    font-size: 13px;
    font-weight: 700;
    color: var(--text-secondary);
    transition: all var(--duration-fast) var(--ease-out);
  }

  .mode-tabs button.active {
    background: var(--accent);
    color: white;
  }

  .mode-tabs button:disabled {
    opacity: 0.4;
  }

  .action-group {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
  }

  .video-input {
    width: 100%;
    padding: 14px 16px;
    font-size: 15px;
    border-radius: 16px;
    background: color-mix(in srgb, var(--bg-soft) 72%, transparent);
    color: var(--text);
    border: 1px solid rgba(148, 163, 184, 0.16);
    outline: none;
    transition: border-color var(--duration-fast) var(--ease-out), box-shadow var(--duration-fast) var(--ease-out), background var(--duration-fast) var(--ease-out);
  }

  .video-input:focus {
    background: color-mix(in srgb, var(--bg-soft) 90%, transparent);
    border-color: color-mix(in srgb, var(--accent-hover) 42%, transparent);
    box-shadow: 0 0 0 4px color-mix(in srgb, var(--accent) 14%, transparent);
  }

  .rec-head {
    margin-top: 14px;
    font-size: 12px;
    font-weight: 700;
    color: var(--text-muted);
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
    color: var(--on-accent-soft);
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
    color: var(--success-fg);
  }

  .submit-btn {
    width: 100%;
  }

  .cover-row {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .cover-preview {
    width: 120px;
    height: 80px;
    object-fit: cover;
    border-radius: 12px;
    border: 1px solid rgba(148, 163, 184, 0.16);
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
