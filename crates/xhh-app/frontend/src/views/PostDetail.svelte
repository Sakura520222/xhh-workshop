<script lang="ts">
  import { onMount } from "svelte";
  import { postDetail, commentCreate, likePost, likeComment, saveImage as saveImageApi, subComments, aiAnalyzeStream, aiCacheGet, aiCacheSave, favourite, favourFolders, createFavouriteFolder } from "../lib/api";
  import type { AiCacheItem } from "../lib/api";
  import { getSelectedLinkId, setView, getPrevView, getFavState, setFavState, clearFavState } from "../lib/stores.svelte";
  import { toastSuccess, toastInfo, toastError } from "../lib/toast.svelte";
  import { renderTextSync, renderAiMarkdown, preloadEmoji, getEmojiVersion } from "../lib/render.svelte";
  import { parsePostContent, type ContentSegment } from "../lib/content";

  let linkId = $derived(getSelectedLinkId());
  let post = $state<any>(null);
  let floors = $state<any[]>([]);
  let loading = $state(true);
  let emojiVer = $derived(getEmojiVersion());

  $effect(() => { void emojiVer; });

  function rt(text: string): string {
    void emojiVer;
    return renderTextSync(text);
  }
  let error = $state("");
  let commentText = $state("");
  let busy = $state(false);
  let replyTo = $state<{ rootId: string; commentId: string; username: string } | null>(null);

  // --- AI 分析 ---
  let aiPanel = $state(false);
  let aiLoading = $state(false);
  let aiResult = $state("");
  let aiError = $state("");
  let aiStreamId = $state(0);
  let aiCacheEntries: AiCacheItem[] = $state([]);

 function closeAiPanel() {
   aiPanel = false;
 }

 // 打开 AI 面板：首次进入（无结果/未加载/无报错）立即触发总结
 function openAiPanel() {
   aiPanel = true;
   if (!aiResult && !aiLoading && !aiError) {
     summarize();
   }
 }

  async function loadAiCache() {
    if (!linkId) { aiCacheEntries = []; return; }
    try {
      const items = await aiCacheGet(linkId);
      aiCacheEntries = items ?? [];
    } catch { aiCacheEntries = []; }
  }

  $effect(() => {
    if (aiPanel && linkId) loadAiCache();
  });

  function summarize() {
    if (!post || aiLoading) return;
    const id = ++aiStreamId;
    aiLoading = true;
    aiError = "";
    aiResult = "";

    const text = bodySegments
      .filter((s: any) => s.kind === "text" || s.kind === "html")
      .map((s: any) => s.kind === "html" ? s.value.replace(/<[^>]+>/g, "") : s.value)
      .join("\n");
    const content = text || post.description || "(无正文)";
    const comments = floors
      .map((floor) => {
        const list = floor.comment ?? [];
        return list
          .map((c: any, i: number) => `${i === 0 ? "" : "  "}${c.user?.username ?? "?"}: ${c.text}`)
          .join("\n");
      })
      .join("\n---\n");

    const hasImages = allImages.length > 0;
    const hasComments = floors.length > 0;

    let prompt = `请对以下帖子进行全面总结，用清晰的段落结构输出中文。\n\n## 帖子标题\n${post.title ?? "(无标题)"}\n\n## 正文内容\n${content}`;

    if (hasImages) {
      prompt += `\n\n## 要求\n帖子包含 ${allImages.length} 张图片，请结合图片和文字内容一起分析总结。`;
    }

    if (hasComments && comments) {
      prompt += `\n\n## 评论区\n${comments}\n\n## 要求\n请同时总结评论区的整体氛围、主要观点和争议点。`;
    }

    aiAnalyzeStream(
      prompt,
      hasImages ? allImages : undefined,
      (chunk) => { if (aiStreamId === id) aiResult += chunk; },
      () => {
        if (aiStreamId === id) {
          aiLoading = false;
          if (linkId && aiResult) {
            aiCacheSave(linkId, "summary", aiResult);
            aiCacheEntries = [
              ...aiCacheEntries.filter(e => e.kind !== "summary"),
              { link_id: linkId, kind: "summary", content: aiResult, updated_at: Math.floor(Date.now() / 1000) },
            ];
          }
        }
      },
      (err) => { if (aiStreamId === id) { aiError = err; aiLoading = false; } },
    );
  }

  // --- 图片查看器 ---
  let viewerUrl = $state("");
  let viewerZoom = $state(1);
  let viewerPanX = $state(0);
  let viewerPanY = $state(0);
  let viewerDragging = $state(false);
  let dragStartX = 0;
  let dragStartY = 0;
  let panStartX = 0;
  let panStartY = 0;
  let viewerImages: string[] = $state([]);
  let viewerIndex = $derived(viewerImages.indexOf(viewerUrl));

  function openViewer(url: string, images?: string[]) {
    viewerImages = images ?? allImages;
    viewerUrl = url;
    viewerZoom = 1;
    viewerPanX = 0;
    viewerPanY = 0;
  }
  function closeViewer() {
    viewerUrl = "";
    viewerZoom = 1;
    viewerPanX = 0;
    viewerPanY = 0;
  }
  function viewerPrev() {
    if (viewerIndex > 0) { viewerUrl = viewerImages[viewerIndex - 1]; viewerZoom = 1; viewerPanX = 0; viewerPanY = 0; }
  }
  function viewerNext() {
    if (viewerIndex < viewerImages.length - 1) { viewerUrl = viewerImages[viewerIndex + 1]; viewerZoom = 1; viewerPanX = 0; viewerPanY = 0; }
  }
  function handleViewerWheel(e: WheelEvent) {
    e.preventDefault();
    const delta = e.deltaY > 0 ? -0.15 : 0.15;
    viewerZoom = Math.max(0.2, Math.min(5, viewerZoom + delta));
  }
  function handleViewerMouseDown(e: MouseEvent) {
    e.preventDefault();
    e.stopPropagation();
    viewerDragging = true;
    dragStartX = e.clientX;
    dragStartY = e.clientY;
    panStartX = viewerPanX;
    panStartY = viewerPanY;
  }
  function handleViewerMouseMove(e: MouseEvent) {
    if (!viewerDragging) return;
    viewerPanX = panStartX + (e.clientX - dragStartX);
    viewerPanY = panStartY + (e.clientY - dragStartY);
  }
  function handleViewerMouseUp() {
    viewerDragging = false;
  }
  function handleViewerClick(e: MouseEvent) {
    // 仅在未拖拽时响应点击
    if (Math.abs(e.clientX - dragStartX) > 5 || Math.abs(e.clientY - dragStartY) > 5) return;
    const rect = (e.currentTarget as HTMLElement).getBoundingClientRect();
    const x = e.clientX - rect.left;
    if (x < rect.width * 0.3) viewerPrev();
    else if (x > rect.width * 0.7) viewerNext();
    else closeViewer();
  }

  // --- 评论区无限滚动 ---
  let commentPage = $state(1);
  let commentHasMore = $state(true);
  let commentLoading = $state(false);
  let commentSentinel: HTMLElement | undefined = $state();
  let commentObserver: IntersectionObserver | null = null;
  let prevSentinel: HTMLElement | undefined;

  $effect(() => {
    if (prevSentinel) {
      commentObserver?.unobserve(prevSentinel);
      prevSentinel = undefined;
    }
    if (!commentSentinel) return;
    if (!commentObserver) {
      const scroller = document.querySelector(".content") as HTMLElement | null;
      commentObserver = new IntersectionObserver(
        (entries) => {
          if (entries[0]?.isIntersecting) loadMoreComments();
        },
        { root: scroller, rootMargin: "200px" }
      );
    }
    commentObserver.observe(commentSentinel);
    prevSentinel = commentSentinel;
  });

  async function loadMoreComments() {
    if (commentLoading || !commentHasMore || !linkId) return;
    commentLoading = true;
    const nextPage = commentPage + 1;
    try {
      const p = await postDetail({ link_id: linkId, page: nextPage, is_first: 0, limit: 20 });
      const newFloors = p?.result?.comments ?? [];
      if (newFloors.length === 0 || p?.result?.has_more_floors !== 1) {
        commentHasMore = false;
      } else {
        floors = [...floors, ...newFloors];
        commentPage = nextPage;
      }
    } catch (e) {
      console.warn("[comments] load more failed:", e);
    } finally {
      commentLoading = false;
    }
  }

  // --- 数据加载 ---
  async function loadAll() {
    if (!linkId) return;
    loading = true;
    error = "";
    commentPage = 1;
    try {
      const p = await postDetail({ link_id: linkId, is_first: 1, limit: 20 });
      post = p?.result?.link ?? null;
      const treeComments = p?.result?.comments;
      if (Array.isArray(treeComments) && treeComments.length > 0) {
        floors = treeComments;
      } else {
        floors = [];
      }
      commentHasMore = p?.result?.has_more_floors === 1;
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  async function refresh() {
    try {
      const p = await postDetail({ link_id: linkId });
      post = p?.result?.link ?? null;
      // refresh 时不重新加载评论（保留已加载和分页状态）
    } catch (e) {
      console.error(e);
    }
  }

  async function send() {
    const text = commentText.trim();
    if (!text || busy) return;
    busy = true;
    try {
      if (replyTo) {
        await commentCreate({
          link_id: linkId,
          text,
          reply_id: replyTo.commentId,
          root_id: replyTo.rootId,
        });
      } else {
        await commentCreate({ link_id: linkId, text });
      }
      commentText = "";
      replyTo = null;
      await refresh();
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  function startReply(rootId: string, commentId: string, username: string) {
    replyTo = { rootId, commentId, username };
  }

  async function copyText(text: string) {
    try {
      await navigator.clipboard.writeText(text ?? "");
    } catch { /* ignore */ }
  }

  function cancelReply() {
    replyTo = null;
  }

  async function handleLikePost() {
    try {
      const liked = post?.is_award_link === 1 || post?.is_award_link === true;
      await likePost(linkId, liked ? 0 : 1);
      await refresh();
    } catch (e) {
      console.error(e);
    }
  }

  function findComment(commentId: string): { floor: any; comment: any } | null {
    for (const floor of floors) {
      for (const c of floor.comment ?? []) {
        if (String(c.commentid) === commentId) return { floor, comment: c };
      }
    }
    return null;
  }

  async function handleLikeComment(commentId: string) {
    const target = findComment(commentId);
    if (!target) return;
    const wasLiked = target.comment.is_support === 1;
    target.comment.is_support = wasLiked ? 2 : 1;
    target.comment.up = (target.comment.up ?? 0) + (wasLiked ? -1 : 1);
    floors = floors;
    try {
      await likeComment(commentId);
    } catch (e) {
      target.comment.is_support = wasLiked ? 1 : 2;
      target.comment.up = (target.comment.up ?? 0) + (wasLiked ? 1 : -1);
      floors = floors;
      console.error("[like comment] err:", e);
    }
  }

  let contentBlocks = $derived(parsePostContent(post?.text));
  let firstContentSegment = $derived(contentBlocks[0]);
  let hasTextImages = $derived(contentBlocks.some((s: any) => s.kind === "images"));
  let topImageUrls = $derived<string[]>(
    firstContentSegment?.kind === "images" && firstContentSegment.top
      ? firstContentSegment.urls
      : post?.imgs?.length && !hasTextImages
        ? post.imgs
        : []
  );
  let bodySegments = $derived<ContentSegment[]>(
    firstContentSegment?.kind === "images" && firstContentSegment.top
      ? contentBlocks.slice(1)
      : contentBlocks
  );
  let allImages = $derived<string[]>([
    ...topImageUrls,
    ...bodySegments.filter((s: any) => s.kind === "images").flatMap((s: any) => s.urls)
  ]);

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

  function back() {
    setView(getPrevView());
  }

  async function saveImage(url: string) {
    try {
      await saveImageApi(url, post?.title);
    } catch (e) {
      console.error("save image failed:", e);
    }
  }

  let expandedRoots = $state<Set<string>>(new Set());
  let loadingSubs = $state<Set<string>>(new Set());

  async function expandSubComments(rootCommentId: string, floorIdx: number) {
    if (expandedRoots.has(rootCommentId) || loadingSubs.has(rootCommentId)) return;
    loadingSubs.add(rootCommentId);
    loadingSubs = loadingSubs;
    try {
      const floor = floors[floorIdx];
      const list = floor?.comment ?? [];
      const lastSub = list[list.length - 1];
      const lastval = lastSub ? String(lastSub.commentid) : "";
      const v = await subComments(rootCommentId, lastval);
      const extras = v?.result?.comments ?? [];
      if (extras.length > 0) {
        const newList = [...list, ...extras];
        floors[floorIdx] = { ...floor, comment: newList };
        floors = floors;
      }
      expandedRoots.add(rootCommentId);
      expandedRoots = expandedRoots;
    } catch (e) {
      console.error("expand sub comments failed:", e);
    } finally {
      loadingSubs.delete(rootCommentId);
      loadingSubs = loadingSubs;
    }
  }

  // 键盘事件（通过 svelte:window 绑定）
  function handleKeydown(e: KeyboardEvent) {
    if (aiPanel) {
      if (e.key === "Escape") closeAiPanel();
      return;
    }
    if (favPanel) {
      if (e.key === "Escape") closeFavPanel();
      return;
    }
    if (viewerUrl) {
      if (e.key === "Escape") closeViewer();
      else if (e.key === "ArrowLeft") viewerPrev();
      else if (e.key === "ArrowRight") viewerNext();
    } else {
      if (e.key === "Escape") back();
    }
  }

  // --- 收藏 ---
  let favPanel = $state(false);
  let favFolders = $state<any[] | null>(null);
  let favBusy = $state(false);
  let newFolderName = $state("");
  let newFolderBusy = $state(false);
  let curFav = $derived(getFavState(linkId));
  let localFaved = $derived(!!curFav);
  let lastFolderId = $derived(curFav?.folderId ?? "");
  let lastFolderName = $derived(curFav?.folderName ?? "");

  async function openFavPanel() {
    if (!post) return;
    favPanel = true;
    if (favFolders === null) {
      try {
        const v = await favourFolders();
        favFolders = v?.result?.folders ?? [];
      } catch (e) {
        console.error("load favour folders failed:", e);
        favFolders = [];
      }
    }
  }

  function closeFavPanel() {
    favPanel = false;
    newFolderName = "";
  }

  async function doFavourite(folderId: string, folderName: string) {
    if (favBusy || !linkId) return;
    favBusy = true;
    try {
      await favourite(linkId, folderId || undefined, 1);
      setFavState(linkId, { folderId, folderName });
      toastSuccess("已收藏到 " + folderName);
      closeFavPanel();
    } catch (e) {
      toastError("收藏失败", String(e));
    } finally {
      favBusy = false;
    }
  }

  async function doCreateFavouriteFolder() {
    const name = newFolderName.trim();
    if (newFolderBusy || !name) return;
    newFolderBusy = true;
    try {
      const v = await createFavouriteFolder(name);
      const folder = v?.result?.folder ?? v?.result ?? null;
      const fid = folder ? String(folder.id ?? "") : "";
      const fname = folder?.name ?? name;
      favFolders = [...(favFolders ?? []), { id: fid, name: fname, count: 0 }];
      newFolderName = "";
      await doFavourite(fid, fname);
    } catch (e) {
      toastError("创建收藏夹失败", String(e));
    } finally {
      newFolderBusy = false;
    }
  }

  async function doUnfavourite() {
    if (favBusy || !linkId) return;
    favBusy = true;
    try {
      await favourite(linkId, lastFolderId || undefined, 2);
      clearFavState(linkId);
      toastInfo("已取消收藏");
      closeFavPanel();
    } catch (e) {
      toastError("取消收藏失败", String(e));
    } finally {
      favBusy = false;
    }
  }

  onMount(() => {
    loadAll();
    preloadEmoji();
    return () => {
      commentObserver?.disconnect();
      commentObserver = null;
    };
  });
</script>

<svelte:window onkeydown={handleKeydown} />

<div class="detail-page">
  {#snippet avatar(user: any)}
    {#if user?.avatar}
      <img src={user.avatar} alt="" class="c-avatar" />
    {:else}
      <div class="c-avatar placeholder">{user?.username?.charAt(0) ?? "?"}</div>
    {/if}
  {/snippet}

  <div class="topbar">
    <button class="back-btn" onclick={back}>返回</button>
    <span class="topbar-title">帖子详情</span>
    <button class="ai-btn" aria-label="AI 助手" onclick={openAiPanel} class:active={aiPanel} disabled={!post}>
      <svg width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
        <path d="M8 1l1.5 3.5L13 6l-3.5 1.5L8 11l-1.5-3.5L3 6l3.5-1.5z"/>
        <path d="M12.5 9.5l.75 1.75L15 12l-1.75.75L12.5 14.5l-.75-1.75L10 12l1.75-.75z"/>
      </svg>
    </button>
  </div>

 {#if aiPanel}
   <!-- svelte-ignore a11y_click_events_have_key_events -->
   <!-- svelte-ignore a11y_no_static_element_interactions -->
   <div class="ai-overlay" onclick={closeAiPanel}>
     <div class="ai-panel" onclick={(e) => e.stopPropagation()}>
       <div class="ai-header">
         <span class="ai-title">AI 助手</span>
         <button class="ai-close" onclick={closeAiPanel}>关闭</button>
       </div>
       <div class="ai-body">
         {#if !aiLoading && !aiResult && !aiError}
           {#if aiCacheEntries.length > 0}
             <div class="ai-history">
               {#each aiCacheEntries as entry}
                 <button class="ai-history-item" onclick={() => { aiResult = entry.content; }}>
                   <span class="ai-history-kind">总结</span>
                   <span class="ai-history-time">{fmtTime(entry.updated_at)}</span>
                 </button>
               {/each}
             </div>
           {/if}
           <button class="ai-action-main" onclick={summarize} disabled={!post}>
             <svg width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.4"><rect x="2" y="2" width="12" height="12" rx="2"/><line x1="5" y1="5" x2="11" y2="5"/><line x1="5" y1="8" x2="11" y2="8"/><line x1="5" y1="11" x2="8" y2="11"/></svg>
             总结
           </button>
         {:else if aiError && !aiResult}
           <div class="ai-error">{aiError}</div>
           <div class="ai-error-actions">
             <button class="ai-retry" onclick={() => { aiError = ""; }}>重试</button>
             <button class="ai-retry" onclick={closeAiPanel}>关闭</button>
           </div>
         {:else}
           <div class="ai-result selectable">{@html renderAiMarkdown(aiResult)}{#if aiLoading}<span class="ai-cursor"></span>{/if}</div>
           {#if !aiLoading}
             <button class="ai-back-btn" onclick={() => { aiResult = ""; }}>重新选择</button>
           {/if}
         {/if}
       </div>
     </div>
   </div>
 {/if}

 {#if favPanel}
   <!-- svelte-ignore a11y_click_events_have_key_events -->
   <!-- svelte-ignore a11y_no_static_element_interactions -->
   <div class="ai-overlay" onclick={closeFavPanel}>
     <div class="ai-panel fav-panel" onclick={(e) => e.stopPropagation()}>
       <div class="ai-header">
         <span class="ai-title">收藏到</span>
         <button class="ai-close" onclick={closeFavPanel}>关闭</button>
       </div>
       <div class="ai-body fav-body">
         {#if localFaved}
           <div class="fav-status">
             <span>已收藏于：{lastFolderName || "默认收藏夹"}</span>
             <button class="fav-unfav" onclick={doUnfavourite} disabled={favBusy}>取消收藏</button>
           </div>
         {/if}
         <button class="fav-item fav-default" onclick={() => doFavourite("", "默认收藏夹")} disabled={favBusy}>
           <span class="fav-name">默认收藏夹</span>
         </button>
         {#if favFolders && favFolders.length > 0}
           <div class="fav-list">
             {#each favFolders as f}
               <button class="fav-item" onclick={() => doFavourite(String(f.id), String(f.name ?? ""))} disabled={favBusy}>
                 <span class="fav-name">{f.name}</span>
                 <span class="fav-count">{f.count ?? 0}</span>
               </button>
             {/each}
           </div>
         {/if}
         <div class="fav-new">
           <input class="fav-input" type="text" placeholder="新收藏夹名称" bind:value={newFolderName} onkeydown={(e) => { if (e.key === "Enter") doCreateFavouriteFolder(); }} />
           <button class="fav-new-btn" onclick={doCreateFavouriteFolder} disabled={newFolderBusy || !newFolderName.trim()}>
             {newFolderBusy ? "..." : "新建并收藏"}
           </button>
         </div>
       </div>
     </div>
   </div>
 {/if}

  {#if loading}
    <div class="status">加载中...</div>
  {:else if error}
    <div class="status error">{error}</div>
  {:else if post}
    <article class="post">
      <div class="post-author">
        {#if post.user?.avatar}
          <img src={post.user.avatar} alt="" class="avatar" />
        {:else}
          <div class="avatar placeholder">{post.user?.username?.charAt(0) ?? "?"}</div>
        {/if}
        <span class="username">{post.user?.username ?? "未知"}</span>
        {#if post.create_at}<span class="time">{fmtTime(post.create_at)}</span>{/if}
      </div>

      <h1 class="post-title selectable">{@html rt(post.title ?? "(无标题)")}</h1>

      {#if topImageUrls.length > 0}
        <div class="img-gallery top-gallery">
          {#each topImageUrls as url}
            <div class="img-wrap">
              <button type="button" class="image-button gallery-button" onclick={() => openViewer(url)} aria-label="查看图片">
                <img src={url} alt="" class="post-img-gallery clickable-img" />
              </button>
              <button class="save-img-btn" onclick={() => saveImage(url)}>保存</button>
            </div>
          {/each}
        </div>
      {/if}

      {#if bodySegments.length > 0}
        <div class="post-content selectable">
          {#each bodySegments as seg}
            {#if seg.kind === "images"}
              <div class="inline-imgs">
                {#each seg.urls as url}
                  <div class="img-wrap inline">
                    <button type="button" class="image-button" onclick={() => openViewer(url)} aria-label="查看图片">
                      <img src={url} alt="" class="post-img-inline clickable-img" />
                    </button>
                    <button class="save-img-btn" onclick={() => saveImage(url)}>保存</button>
                  </div>
                {/each}
              </div>
            {:else if seg.kind === "html"}
              <div class="post-html selectable">{@html seg.value}</div>
            {:else}
              <p>{@html rt(seg.value)}</p>
            {/if}
          {/each}
        </div>
      {:else if post.description && topImageUrls.length === 0}
        <div class="post-content selectable">{@html rt(post.description)}</div>
      {/if}

      {#if post.topics?.length}
        <div class="topics">
          {#each post.topics as t}
            <span class="topic">#{t.name}</span>
          {/each}
        </div>
      {/if}

      <div class="post-actions">
        <button class="action-btn" class:liked={post?.is_award_link === 1} onclick={handleLikePost}>
          {post?.is_award_link === 1 ? "已赞" : "点赞"} {post?.link_award_num ?? 0}
        </button>
        <button class="action-btn" class:faved={localFaved} onclick={openFavPanel} disabled={favBusy}>
          {localFaved ? "已收藏" : "收藏"}
        </button>
        <span class="meta">{post.comment_num ?? 0} 评论</span>
      </div>
    </article>

    <div class="comments-section">
      <h3 class="section-title">评论</h3>
      {#if floors.length === 0 && !loading}
        <div class="status small">暂无评论，快来抢沙发</div>
      {:else}
        <div class="floor-list">
          {#each floors as floor, floorIdx}
            {@const list = floor.comment ?? []}
            {@const rootId = list[0]?.commentid}
            {#if list.length > 0}
              {@const root = list[0]}
              <div class="floor">
                <div class="comment root">
                  {@render avatar(root.user)}
                  <div class="c-body">
                    <div class="c-user">{root.user?.username ?? "?"}</div>
                    <div class="c-text selectable">{@html rt(root.text)}</div>
                    {#if root.imgs?.length}
                      <div class="c-imgs">
                        {#each root.imgs as img}
                          {@const rootImgUrl = img.url ?? img.thumb}
                          <button
                            type="button"
                            class="c-img-btn"
                            aria-label="查看图片"
                            onclick={() => openViewer(rootImgUrl, root.imgs.map((i: any) => i.url ?? i.thumb))}
                          >
                            <img src={rootImgUrl} alt="" class="c-img" />
                          </button>
                        {/each}
                      </div>
                    {/if}
                    <div class="c-meta">
                      <span class="time">{fmtTime(root.create_at)}</span>
                      <button class="link-btn" onclick={() => startReply(String(root.commentid), String(root.commentid), root.user?.username ?? "?")}>回复</button>
                      <button class="link-btn liked" onclick={() => handleLikeComment(String(root.commentid))}>{root.is_support === 1 ? "已赞" : "赞"} {root.up ?? 0}</button>
                      <button class="link-btn" onclick={() => copyText(root.text)}>复制</button>
                    </div>
                  </div>
                </div>
                {#each list.slice(1) as sub}
                  <div class="comment sub">
                    {@render avatar(sub.user)}
                    <div class="c-body">
                      <div class="c-user">
                        {sub.user?.username ?? "?"}
                        {#if sub.replyuser?.username}<span class="reply-to"> 回复 @{sub.replyuser.username}</span>{/if}
                      </div>
                      <div class="c-text selectable">{@html rt(sub.text)}</div>
                      {#if sub.imgs?.length}
                        <div class="c-imgs">
                          {#each sub.imgs as img}
                            {@const subImgUrl = img.url ?? img.thumb}
                            <button
                              type="button"
                              class="c-img-btn"
                              aria-label="查看图片"
                              onclick={() => openViewer(subImgUrl, sub.imgs.map((i: any) => i.url ?? i.thumb))}
                            >
                              <img src={subImgUrl} alt="" class="c-img" />
                            </button>
                          {/each}
                        </div>
                      {/if}
                      <div class="c-meta">
                        <span class="time">{fmtTime(sub.create_at)}</span>
                        <button class="link-btn" onclick={() => startReply(String(root.commentid), String(sub.commentid), sub.user?.username ?? "?")}>回复</button>
                        <button class="link-btn liked" onclick={() => handleLikeComment(String(sub.commentid))}>{sub.is_support === 1 ? "已赞" : "赞"} {sub.up ?? 0}</button>
                        <button class="link-btn" onclick={() => copyText(sub.text)}>复制</button>
                      </div>
                    </div>
                  </div>
                {/each}
                {#if root.has_more === 1 && !expandedRoots.has(String(rootId))}
                  <div
                    class="expand-btn"
                    role="button"
                    tabindex="0"
                    onclick={() => expandSubComments(String(rootId), floorIdx)}
                    onkeydown={(e) => { if (e.key === "Enter" || e.key === " ") { e.preventDefault(); expandSubComments(String(rootId), floorIdx); } }}
                  >
                    {loadingSubs.has(String(rootId)) ? "加载中..." : "展开更多回复"}
                  </div>
                {/if}
              </div>
            {/if}
          {/each}
          <!-- 评论区无限滚动哨兵 -->
          {#if commentHasMore}
            <div bind:this={commentSentinel} class="sentinel"></div>
          {/if}
          {#if commentLoading}
            <div class="status small">加载更多评论...</div>
          {/if}
        </div>
      {/if}
    </div>
  {/if}

  <div class="comment-bar">
    {#if replyTo}
      <div class="reply-hint">
        回复 @{replyTo.username}
        <button class="link-btn" onclick={cancelReply}>取消</button>
      </div>
    {/if}
    <div class="input-row">
      <input
        bind:value={commentText}
        placeholder={replyTo ? `回复 @${replyTo.username}` : "写评论..."}
        class="c-input"
      />
      <button class="send-btn" onclick={send} disabled={busy || !commentText.trim()}>
        {busy ? "发送中" : "发送"}
      </button>
    </div>
  </div>

  {#if viewerUrl}
    <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div
      class="viewer-overlay"
      onclick={handleViewerClick}
      onkeydown={handleKeydown}
      onmousedown={handleViewerMouseDown}
      onmousemove={handleViewerMouseMove}
      onmouseup={handleViewerMouseUp}
      onmouseleave={handleViewerMouseUp}
      tabindex="0"
    >
      <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
      <!-- svelte-ignore a11y_click_events_have_key_events -->
      <img
        src={viewerUrl}
        alt=""
        class="viewer-img"
        style="transform: scale({viewerZoom}) translate({viewerPanX / viewerZoom}px, {viewerPanY / viewerZoom}px)"
        onclick={(e) => e.stopPropagation()}
        onwheel={handleViewerWheel}
        class:dragging={viewerDragging}
      />
      <!-- svelte-ignore a11y_click_events_have_key_events -->
      <div class="viewer-actions" onclick={(e) => e.stopPropagation()}>
        {#if viewerIndex > 0}
          <button class="viewer-btn" onclick={viewerPrev}>上一张</button>
        {/if}
        <button class="viewer-btn" onclick={() => saveImage(viewerUrl)}>保存</button>
        <button class="viewer-btn" onclick={closeViewer}>关闭</button>
        {#if viewerIndex < viewerImages.length - 1}
          <button class="viewer-btn" onclick={viewerNext}>下一张</button>
        {/if}
      </div>
    </div>
  {/if}
</div>

<style>
  .detail-page {
    max-width: 720px;
    margin: 0 auto;
    padding-bottom: 100px;
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
  .post {
    background: var(--glass-bg);
    backdrop-filter: var(--glass-blur);
    -webkit-backdrop-filter: var(--glass-blur);
    border-radius: var(--radius);
    padding: 20px;
    margin-bottom: 20px;
    border: 0.5px solid var(--glass-border);
    box-shadow: var(--elevation-1);
  }
  .post-author {
    display: flex;
    align-items: center;
    gap: 10px;
    margin-bottom: 14px;
  }
  .avatar {
    width: 40px;
    height: 40px;
    border-radius: 50%;
    object-fit: cover;
  }
  .avatar.placeholder {
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--accent);
    color: white;
    font-weight: 600;
  }
  .username {
    font-size: 14px;
    font-weight: 500;
  }
  .time {
    font-size: 12px;
    color: var(--text-secondary);
    margin-left: auto;
  }
  .post-title {
    font-size: 20px;
    font-weight: 600;
    line-height: 1.4;
    margin-bottom: 12px;
  }
  .post-title :global(.emoji) {
    width: 1em;
    height: 1em;
    vertical-align: middle;
    display: inline-block;
    margin: 0 1px;
  }
  .post-content {
    font-size: 14px;
    line-height: 1.8;
    color: var(--text);
    white-space: pre-wrap;
    margin-bottom: 12px;
  }
  .post-content p {
    margin: 0 0 8px;
  }
  .post-html {
    font-size: 14px;
    line-height: 1.8;
    color: var(--text);
    margin-bottom: 8px;
  }
  .post-html :global(img) {
    max-width: 100%;
    max-height: 500px;
    object-fit: contain;
    border-radius: 8px;
    margin: 8px 0;
    display: block;
  }
  .post-html :global(p) {
    margin: 0 0 8px;
  }
  .selectable {
    user-select: text;
    -webkit-user-select: text;
    cursor: auto;
  }
  .post-img-inline {
    max-width: 100%;
    max-height: 500px;
    object-fit: contain;
    border-radius: 8px;
    margin: 8px 0;
    display: block;
  }
  .image-button {
    display: block;
    padding: 0;
    border: 0;
    background: none;
    line-height: 0;
    cursor: zoom-in;
  }
  .gallery-button {
    width: 100%;
  }
  .gallery-button .post-img-gallery {
    display: block;
  }
  .img-gallery {
    display: grid;
    gap: 8px;
    margin-bottom: 12px;
  }
  .img-gallery .img-wrap { display: inline-block; }
  .post-img-gallery {
    width: 100%;
    max-height: 420px;
    object-fit: cover;
    border-radius: 8px;
  }
  .inline-imgs {
    margin: 8px 0;
  }
  .inline-imgs .img-wrap {
    display: inline-block;
    margin-right: 8px;
    margin-bottom: 4px;
  }
  .inline-imgs .post-img-inline {
    max-width: 320px;
    max-height: 320px;
  }
  .topics {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    margin-bottom: 12px;
  }
  .topic {
    font-size: 12px;
    color: var(--accent);
    background: rgba(255, 107, 53, 0.12);
    padding: 2px 8px;
    border-radius: 4px;
  }
  .img-wrap {
    position: relative;
    display: inline-block;
  }
  .img-wrap .save-img-btn {
    position: absolute;
    bottom: 8px;
    right: 8px;
    padding: 4px 10px;
    border-radius: 6px;
    background: rgba(0, 0, 0, 0.55);
    color: white;
    font-size: 12px;
    opacity: 0;
    transition: opacity 0.2s;
  }
  .img-wrap:hover .save-img-btn {
    opacity: 1;
  }
  .post-actions {
    display: flex;
    align-items: center;
    gap: 16px;
    padding-top: 12px;
    border-top: 0.5px solid var(--glass-border);
  }
  .action-btn {
    padding: 6px 16px;
    border-radius: 10px;
    background: rgba(255, 255, 255, 0.06);
    border: 0.5px solid rgba(255, 255, 255, 0.08);
    font-size: 13px;
    transition: all var(--duration-fast) var(--ease-out);
  }
  .action-btn:hover {
    background: var(--accent);
    border-color: var(--accent);
  }
  .action-btn.liked {
    background: var(--accent);
    color: white;
  }
  .meta {
    font-size: 12px;
    color: var(--text-secondary);
  }
  .comments-section {
    margin-top: 8px;
  }
  .section-title {
    font-size: 15px;
    font-weight: 600;
    margin-bottom: 14px;
  }
  .floor-list {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }
  .floor {
    background: var(--glass-bg);
    backdrop-filter: var(--glass-blur);
    -webkit-backdrop-filter: var(--glass-blur);
    border-radius: var(--radius);
    padding: 14px 16px;
    border: 0.5px solid var(--glass-border);
    box-shadow: var(--elevation-1);
  }
  .comment {
    display: flex;
    gap: 10px;
  }
  .comment.sub {
    margin-top: 12px;
    margin-left: 38px;
    padding-top: 12px;
    border-top: 0.5px solid var(--glass-border);
  }
  .c-avatar {
    width: 32px;
    height: 32px;
    border-radius: 50%;
    object-fit: cover;
    flex-shrink: 0;
  }
  .c-avatar.placeholder {
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--accent);
    color: white;
    font-size: 12px;
    font-weight: 600;
  }
  .c-body {
    flex: 1;
    min-width: 0;
  }
  .c-user {
    font-size: 13px;
    font-weight: 500;
    color: var(--accent);
  }
  .reply-to {
    font-weight: 400;
    color: var(--text-secondary);
    font-size: 12px;
  }
  .c-text {
    font-size: 14px;
    line-height: 1.6;
    margin: 4px 0;
    word-break: break-word;
  }
  .c-imgs {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    margin-bottom: 6px;
  }
  .c-img {
    display: block;
    max-width: 200px;
    max-height: 200px;
    object-fit: cover;
    border-radius: 8px;
  }
  .c-img-btn {
    padding: 0;
    border: 0;
    background: none;
    line-height: 0;
    cursor: zoom-in;
  }
  .c-meta {
    display: flex;
    align-items: center;
    gap: 14px;
    font-size: 12px;
    color: var(--text-secondary);
  }
  .link-btn {
    font-size: 12px;
    color: var(--text-secondary);
    padding: 0;
  }
  .link-btn:hover {
    color: var(--accent);
  }
  .link-btn.liked {
    color: var(--accent);
  }
  .expand-btn {
    margin-top: 10px;
    margin-left: 42px;
    font-size: 12px;
    color: var(--accent);
    cursor: pointer;
  }
  .expand-btn:hover {
    text-decoration: underline;
  }
 .comment-bar {
   position: fixed;
   bottom: 12px;
   left: var(--sidebar-width);
   right: 0;
   max-width: 720px;
   margin: 0 auto;
    padding: 14px 16px;
    border-radius: 100px;
    background: linear-gradient(
      180deg,
      rgba(255, 255, 255, 0.18) 0%,
      rgba(255, 255, 255, 0.08) 100%
    );
    backdrop-filter: blur(40px) saturate(1.8) brightness(1.05);
    -webkit-backdrop-filter: blur(40px) saturate(1.8) brightness(1.05);
    border: 0.5px solid rgba(255, 255, 255, 0.2);
    box-shadow:
      inset 0 1px 0 rgba(255, 255, 255, 0.5),
      inset 0 -0.5px 0 rgba(255, 255, 255, 0.1),
      0 8px 40px rgba(0, 0, 0, 0.12),
      0 2px 12px rgba(0, 0, 0, 0.06);
    z-index: 100;
  }
  .reply-hint {
    display: flex;
    align-items: center;
    justify-content: space-between;
    font-size: 12px;
    color: var(--text-secondary);
    margin-bottom: 8px;
  }
  .input-row {
    display: flex;
    gap: 8px;
  }
  .c-input {
    flex: 1;
    padding: 10px 14px;
    border-radius: 14px;
    background: rgba(255, 255, 255, 0.08);
    color: var(--text);
    border: 0.5px solid rgba(255, 255, 255, 0.15);
    font-size: 14px;
    outline: none;
    box-shadow: inset 0 0.5px 0 rgba(255, 255, 255, 0.2);
    transition: background 0.2s, border-color 0.2s, box-shadow 0.2s;
  }
  .c-input:focus {
    border-color: rgba(255, 255, 255, 0.3);
    background: rgba(255, 255, 255, 0.14);
    box-shadow:
      inset 0 0.5px 0 rgba(255, 255, 255, 0.3),
      0 0 0 3px rgba(255, 255, 255, 0.1);
  }
  .c-input::placeholder {
    color: #222;
  }
  .send-btn {
    padding: 0 22px;
    border-radius: 14px;
    background: var(--accent);
    color: white;
    font-size: 14px;
    border: 0.5px solid rgba(255, 255, 255, 0.15);
    box-shadow:
      0 2px 10px rgba(0, 0, 0, 0.12),
      inset 0 0.5px 0 rgba(255, 255, 255, 0.25);
  }
  .send-btn:hover:not(:disabled) {
    background: var(--accent-hover);
  }
  .send-btn:disabled {
    opacity: 0.5;
  }
  .status {
    text-align: center;
    padding: 40px 0;
    color: var(--text-secondary);
  }
  .status.small {
    padding: 24px 0;
    font-size: 13px;
  }
  .status.error {
    color: #f87171;
  }
  .clickable-img {
    cursor: zoom-in;
  }
  .viewer-overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.85);
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    z-index: 9999;
  }
  .viewer-img {
    max-width: 90vw;
    max-height: 80vh;
    object-fit: contain;
    border-radius: 4px;
    transition: transform 0.15s ease-out;
    cursor: grab;
    user-select: none;
    -webkit-user-drag: none;
  }
  .viewer-img.dragging {
    cursor: grabbing;
    transition: none;
  }
  .viewer-actions {
    margin-top: 16px;
    display: flex;
    gap: 12px;
  }
  .viewer-btn {
    padding: 8px 20px;
    border-radius: 12px;
    background: rgba(255, 255, 255, 0.12);
    border: 0.5px solid rgba(255, 255, 255, 0.15);
    color: white;
    font-size: 13px;
    transition: all var(--duration-fast) var(--ease-out);
  }
  .viewer-btn:hover {
    background: rgba(255, 255, 255, 0.22);
  }
  .sentinel {
    height: 1px;
  }
  .post-content :global(.emoji),
  .post-content p :global(.emoji) {
    width: 1em;
    height: 1em;
    vertical-align: middle;
    display: inline-block;
    margin: 0 1px;
  }
  .c-text :global(.emoji) {
    width: 1em;
    height: 1em;
  }
  .ai-result :global(.md-code) {
    padding: 2px 6px;
    border-radius: 4px;
    background: rgba(255, 255, 255, 0.1);
    font-size: 13px;
    font-family: monospace;
  }
  .ai-result :global(.md-code-block) {
    padding: 12px 16px;
    border-radius: 8px;
    background: rgba(0, 0, 0, 0.3);
    overflow-x: auto;
    margin: 8px 0;
  }
  .ai-result :global(.md-code-block code) {
    font-size: 13px;
    font-family: monospace;
    line-height: 1.6;
  }
  .ai-result :global(.md-link) {
    color: var(--accent);
    text-decoration: none;
  }
  .ai-result :global(.md-link:hover) {
    text-decoration: underline;
  }
  .ai-btn {
    margin-left: auto;
    padding: 6px 8px;
    border-radius: 12px;
    background: rgba(255, 255, 255, 0.08);
    border: 0.5px solid rgba(255, 255, 255, 0.15);
    color: var(--text-secondary);
    display: flex;
    align-items: center;
    gap: 4px;
    transition: all var(--duration-fast) var(--ease-out);
    box-shadow: inset 0 0.5px 0 rgba(255, 255, 255, 0.2);
  }
  .ai-btn:hover:not(:disabled) {
    background: rgba(255, 107, 53, 0.15);
    border-color: rgba(255, 107, 53, 0.3);
    color: var(--accent);
  }
  .ai-btn.active {
    background: var(--accent);
    border-color: var(--accent);
    color: white;
  }
  .ai-btn:disabled {
    opacity: 0.4;
  }
 .ai-overlay {
   position: fixed;
   top: 0;
   bottom: 0;
   left: var(--sidebar-width);
   right: 0;
   background: rgba(0, 0, 0, 0.5);
   z-index: 9998;
   display: flex;
   align-items: center;
   justify-content: center;
   padding: 20px;
 }
  .ai-panel {
    background: var(--glass-bg);
    backdrop-filter: var(--glass-blur);
    -webkit-backdrop-filter: var(--glass-blur);
    border-radius: var(--radius);
    border: 0.5px solid var(--glass-border);
    box-shadow: var(--elevation-1);
    padding: 16px 18px;
    width: 100%;
    max-width: 640px;
    max-height: 80vh;
    display: flex;
    flex-direction: column;
    animation: aiSlideIn 0.2s ease-out;
  }
  .ai-body {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
  }
  @keyframes aiSlideIn {
    from { opacity: 0; transform: scale(0.96) translateY(8px); }
    to { opacity: 1; transform: scale(1) translateY(0); }
  }
  .ai-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 14px;
    padding-bottom: 10px;
    border-bottom: 0.5px solid var(--glass-border);
  }
  .ai-title {
    font-size: 14px;
    font-weight: 500;
  }
  .ai-close {
    font-size: 12px;
    color: var(--text-secondary);
    padding: 4px 10px;
    border-radius: 8px;
    background: rgba(255, 255, 255, 0.06);
    border: 0.5px solid var(--glass-border);
    transition: all var(--duration-fast) var(--ease-out);
  }
  .ai-close:hover {
    background: rgba(255, 255, 255, 0.12);
    color: var(--text);
  }
  .ai-history {
    display: flex;
    flex-direction: column;
    gap: 6px;
    margin-bottom: 12px;
  }
  .ai-history-item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 8px 12px;
    border-radius: 10px;
    background: rgba(255, 107, 53, 0.06);
    border: 0.5px solid rgba(255, 107, 53, 0.15);
    font-size: 12px;
    color: var(--text);
    transition: all var(--duration-fast) var(--ease-out);
  }
  .ai-history-item:hover {
    background: rgba(255, 107, 53, 0.12);
    border-color: rgba(255, 107, 53, 0.3);
  }
  .ai-history-kind {
    color: var(--accent);
    font-weight: 500;
  }
  .ai-history-time {
    color: var(--text-secondary);
    font-size: 11px;
  }
  .ai-action-main {
    width: 100%;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    padding: 12px;
    border-radius: 12px;
    background: rgba(255, 255, 255, 0.04);
    border: 0.5px solid rgba(255, 255, 255, 0.08);
    color: var(--text);
    font-size: 13px;
    transition: all var(--duration-fast) var(--ease-out);
  }
  .ai-action-main:hover:not(:disabled) {
    background: rgba(255, 107, 53, 0.1);
    border-color: rgba(255, 107, 53, 0.25);
    color: var(--accent);
  }
  .ai-action-main:disabled {
    opacity: 0.35;
    cursor: not-allowed;
  }
  .ai-action-main svg {
    opacity: 0.6;
  }
  .ai-action-main:hover:not(:disabled) svg {
    opacity: 1;
  }
 .ai-result {
   font-size: 14px;
   line-height: 1.8;
   padding: 4px 0;
   animation: aiFadeIn 0.3s ease-out;
 }
  .ai-result :global(p) {
    margin: 0 0 8px;
  }
  .ai-result :global(p:last-child) {
    margin-bottom: 0;
  }
  .ai-result :global(.ai-md-h1) {
    font-size: 18px;
    font-weight: 700;
    margin: 16px 0 8px;
    line-height: 1.4;
  }
  .ai-result :global(.ai-md-h2) {
    font-size: 16px;
    font-weight: 600;
    margin: 14px 0 6px;
    line-height: 1.4;
    padding-bottom: 4px;
    border-bottom: 0.5px solid rgba(255, 255, 255, 0.1);
  }
  .ai-result :global(.ai-md-h3) {
    font-size: 15px;
    font-weight: 600;
    margin: 10px 0 4px;
    line-height: 1.4;
  }
  .ai-result :global(.ai-md-h4),
  .ai-result :global(.ai-md-h5),
  .ai-result :global(.ai-md-h6) {
    font-size: 14px;
    font-weight: 600;
    margin: 8px 0 4px;
  }
  .ai-result :global(.ai-md-ul),
  .ai-result :global(.ai-md-ol) {
    margin: 4px 0 8px;
    padding-left: 20px;
  }
  .ai-result :global(.ai-md-ul) {
    list-style: disc;
  }
  .ai-result :global(.ai-md-ol) {
    list-style: decimal;
  }
  .ai-result :global(.ai-md-ul li),
  .ai-result :global(.ai-md-ol li) {
    margin: 2px 0;
    line-height: 1.7;
  }
  .ai-result :global(.ai-md-blockquote) {
    margin: 6px 0;
    padding: 6px 12px;
    border-left: 3px solid var(--accent);
    background: rgba(255, 107, 53, 0.06);
    border-radius: 0 6px 6px 0;
    color: var(--text-secondary);
    font-size: 13px;
  }
  .ai-result :global(.ai-md-pre) {
    margin: 8px 0;
    padding: 12px 14px;
    border-radius: 8px;
    background: rgba(0, 0, 0, 0.3);
    overflow-x: auto;
  }
  .ai-result :global(.ai-md-pre code) {
    font-size: 13px;
    font-family: monospace;
    line-height: 1.6;
    color: var(--text);
  }
  .ai-result :global(.ai-md-code) {
    padding: 1px 5px;
    border-radius: 4px;
    background: rgba(255, 255, 255, 0.1);
    font-size: 13px;
    font-family: monospace;
  }
  .ai-result :global(.ai-md-link) {
    color: var(--accent);
    text-decoration: none;
  }
  .ai-result :global(.ai-md-link:hover) {
    text-decoration: underline;
  }
  .ai-result :global(.ai-md-hr) {
    border: none;
    border-top: 0.5px solid rgba(255, 255, 255, 0.12);
    margin: 12px 0;
  }
  .ai-result :global(.ai-md-table-wrap) {
    margin: 10px 0;
    overflow-x: auto;
    border: 1px solid rgba(255, 255, 255, 0.12);
    border-radius: 8px;
  }
  .ai-result :global(.ai-md-table) {
    width: 100%;
    min-width: max-content;
    border-collapse: collapse;
    font-size: 13px;
  }
  .ai-result :global(.ai-md-table th),
  .ai-result :global(.ai-md-table td) {
    padding: 7px 10px;
    border-right: 1px solid rgba(255, 255, 255, 0.1);
    border-bottom: 1px solid rgba(255, 255, 255, 0.1);
    vertical-align: top;
  }
  .ai-result :global(.ai-md-table th:last-child),
  .ai-result :global(.ai-md-table td:last-child) {
    border-right: none;
  }
  .ai-result :global(.ai-md-table tbody tr:last-child td) {
    border-bottom: none;
  }
  .ai-result :global(.ai-md-table th) {
    background: rgba(255, 255, 255, 0.08);
    color: var(--text);
    font-weight: 600;
  }
  .ai-result :global(.ai-md-table tbody tr:nth-child(even)) {
    background: rgba(255, 255, 255, 0.03);
  }
  .ai-result :global(.ai-md-align-left) {
    text-align: left;
  }
  .ai-result :global(.ai-md-align-center) {
    text-align: center;
  }
  .ai-result :global(.ai-md-align-right) {
    text-align: right;
  }
  .ai-result :global(.emoji) {
    width: 1.35em;
    height: 1.35em;
    margin: 0 2px;
    object-fit: contain;
    vertical-align: -0.28em;
  }
  .ai-result :global(strong) {
    font-weight: 600;
  }
  .ai-result :global(del) {
    opacity: 0.5;
  }
  @keyframes aiFadeIn {
    from { opacity: 0; }
    to { opacity: 1; }
  }
  .ai-cursor {
    display: inline-block;
    width: 2px;
    height: 1.1em;
    background: var(--accent);
    margin-left: 1px;
    vertical-align: text-bottom;
    animation: aiBlink 1s step-end infinite;
  }
  @keyframes aiBlink {
    50% { opacity: 0; }
  }
  .ai-error {
    padding: 12px 14px;
    border-radius: 10px;
    background: rgba(248, 113, 113, 0.1);
    border: 0.5px solid rgba(248, 113, 113, 0.2);
    color: #f87171;
    font-size: 13px;
    line-height: 1.5;
  }
  .ai-error-actions {
    display: flex;
    gap: 8px;
    margin-top: 10px;
    justify-content: flex-end;
  }
  .ai-retry {
    font-size: 12px;
    color: var(--text-secondary);
    padding: 5px 14px;
    border-radius: 8px;
    background: rgba(255, 255, 255, 0.06);
    border: 0.5px solid var(--glass-border);
    transition: all var(--duration-fast) var(--ease-out);
  }
  .ai-retry:hover {
    background: rgba(255, 255, 255, 0.12);
    color: var(--text);
  }
  .ai-back-btn {
    margin-top: 12px;
    padding: 6px 14px;
    border-radius: 8px;
    font-size: 12px;
    color: var(--text-secondary);
    background: rgba(255, 255, 255, 0.04);
    border: 0.5px solid var(--glass-border);
    transition: all var(--duration-fast) var(--ease-out);
  }
  .ai-back-btn:hover {
    background: rgba(255, 255, 255, 0.1);
    color: var(--text);
  }
  .action-btn.faved {
    background: var(--accent);
    color: white;
  }
  .fav-panel {
    max-width: 480px;
  }
  .fav-body {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .fav-status {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 10px 12px;
    border-radius: 10px;
    background: rgba(255, 107, 53, 0.08);
    border: 0.5px solid rgba(255, 107, 53, 0.2);
    font-size: 13px;
    margin-bottom: 4px;
  }
  .fav-unfav {
    font-size: 12px;
    padding: 4px 10px;
    border-radius: 8px;
    background: rgba(248, 113, 113, 0.12);
    border: 0.5px solid rgba(248, 113, 113, 0.3);
    color: #f87171;
    transition: all var(--duration-fast) var(--ease-out);
  }
  .fav-unfav:hover:not(:disabled) {
    background: rgba(248, 113, 113, 0.2);
  }
  .fav-item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 11px 14px;
    border-radius: 10px;
    background: rgba(255, 255, 255, 0.06);
    border: 0.5px solid var(--glass-border);
    font-size: 13px;
    transition: all var(--duration-fast) var(--ease-out);
    text-align: left;
  }
  .fav-item:hover:not(:disabled) {
    background: var(--accent);
    border-color: var(--accent);
    color: white;
  }
  .fav-item:disabled {
    opacity: 0.5;
  }
  .fav-default {
    background: rgba(255, 107, 53, 0.1);
    border-color: rgba(255, 107, 53, 0.25);
    font-weight: 500;
  }
  .fav-name {
    flex: 1;
  }
  .fav-count {
    font-size: 11px;
    color: var(--text-secondary);
  }
  .fav-item:hover:not(:disabled) .fav-count {
    color: rgba(255, 255, 255, 0.8);
  }
  .fav-new {
    display: flex;
    gap: 8px;
    margin-top: 4px;
    padding-top: 10px;
    border-top: 0.5px solid var(--glass-border);
  }
  .fav-input {
    flex: 1;
    padding: 8px 12px;
    border-radius: 10px;
    background: rgba(255, 255, 255, 0.06);
    border: 0.5px solid var(--glass-border);
    color: var(--text);
    font-size: 13px;
    outline: none;
    transition: all var(--duration-fast) var(--ease-out);
  }
  .fav-input:focus {
    border-color: var(--accent);
    background: rgba(255, 255, 255, 0.1);
  }
  .fav-new-btn {
    padding: 8px 14px;
    border-radius: 10px;
    background: var(--accent);
    border: 0.5px solid var(--accent);
    color: white;
    font-size: 13px;
    transition: all var(--duration-fast) var(--ease-out);
  }
  .fav-new-btn:hover:not(:disabled) {
    opacity: 0.9;
  }
  .fav-new-btn:disabled {
    opacity: 0.4;
  }
</style>
