<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import {
    agentChatStream,
    agentReset,
    agentHistoryGet,
    agentHistorySave,
    agentHistoryClear,
    agentSessionList,
    agentSessionCreate,
    agentSessionSwitch,
    agentSessionRename,
    agentSessionDelete,
    agentTemplateList,
    agentTemplateSave,
    agentTemplateDelete,
    postDetail,
    subComments,
    type AgentStreamDone,
    type AgentToolConfirmationDecision,
    type AgentToolConfirmationRequest,
    type SessionMeta,
    type AgentTemplate,
  } from "../lib/api";
  import type { AgentUiMsg } from "../lib/api";
  import { parsePostContent } from "../lib/content";
  import { preloadEmoji, renderAiMarkdown } from "../lib/render.svelte";
  import { getView } from "../lib/stores.svelte";
  import AgentSidebar from "../components/AgentSidebar.svelte";
  import TemplateEditor from "../components/TemplateEditor.svelte";

  type ChatMsg = {
    role: "user" | "assistant";
    text: string;
    streaming?: boolean;
    tools?: string[];
    loops?: number;
    error?: boolean;
  };

  type DeletePreview = {
    post: {
      linkId: string;
      title: string;
      author: string;
      excerpt: string;
      image: string;
    };
    comment?: {
      commentId: string;
      author: string;
      text: string;
      image: string;
      verified: boolean;
    };
  };

  function toUiMsg(m: ChatMsg): AgentUiMsg {
    return { role: m.role, text: m.text, tools: m.tools, loops: m.loops };
  }

  let input = $state("");
  let busy = $state(false);
  let messages = $state<ChatMsg[]>([]);
  let pendingConfirmation = $state<AgentToolConfirmationRequest | null>(null);
  let pendingMessage = $state("");
  let approvedConfirmations = $state<AgentToolConfirmationDecision[]>([]);
  let autoApprove = $state(false);
  // 自动审批时暂存的确认请求：onConfirmation 收到后不弹框，等旧流报错清理后再自动批准
  let autoApproveQueued = $state<AgentToolConfirmationRequest | null>(null);
  let scrollEl: HTMLElement | null = $state(null);
  let cancelConfirmationButton: HTMLButtonElement | null = $state(null);
  let messageInput: HTMLTextAreaElement | null = $state(null);
  let deletePreview = $state<DeletePreview | null>(null);
  let deletePreviewLoading = $state(false);
  let deletePreviewError = $state("");
  let operationNotice = $state("");
  let deletePreviewRequestId = 0;
  let operationNoticeTimer: ReturnType<typeof setTimeout> | null = null;

  // 多会话 + 模板状态
  let sessions = $state<SessionMeta[]>([]);
  let activeSessionId = $state("");
  let templates = $state<AgentTemplate[]>([]);
  let templatesCollapsed = $state(false);
  let editingTemplate = $state<AgentTemplate | null>(null);
  let templateEditorOpen = $state(false);
  let templateToast = $state("");

  const TEMPLATES_COLLAPSED_KEY = "xhh_agent_templates_collapsed";
  const AUTO_APPROVE_KEY = "xhh_agent_auto_approve";

  function loadAutoApprove(): boolean {
    try {
      return localStorage.getItem(AUTO_APPROVE_KEY) === "1";
    } catch { return false; }
  }
  function saveAutoApprove(v: boolean) {
    try { localStorage.setItem(AUTO_APPROVE_KEY, v ? "1" : "0"); } catch { /* ignore */ }
  }
  function toggleAutoApprove() {
    autoApprove = !autoApprove;
    saveAutoApprove(autoApprove);
  }

  function loadTemplatesCollapsed() {
    try {
      return localStorage.getItem(TEMPLATES_COLLAPSED_KEY) === "1";
    } catch { return false; }
  }
  function saveTemplatesCollapsed(v: boolean) {
    try { localStorage.setItem(TEMPLATES_COLLAPSED_KEY, v ? "1" : "0"); } catch { /* ignore */ }
  }

  const POST_DETAIL_PAGE_LIMIT = 10;
  const SUB_COMMENT_PAGE_LIMIT = 10;
  const PREVIEW_REQUEST_DELAY_MS = 180;

  function showOperationNotice(message: string) {
    operationNotice = message;
    if (operationNoticeTimer) clearTimeout(operationNoticeTimer);
    operationNoticeTimer = setTimeout(() => {
      operationNotice = "";
      operationNoticeTimer = null;
    }, 5000);
  }

  async function waitForPreviewRequest(requestId: number) {
    await new Promise((resolve) => setTimeout(resolve, PREVIEW_REQUEST_DELAY_MS));
    if (requestId !== deletePreviewRequestId) throw new Error("预览请求已取消");
  }

  function resetDeletePreview() {
    deletePreviewRequestId++;
    deletePreview = null;
    deletePreviewLoading = false;
    deletePreviewError = "";
  }

  function plainText(value: unknown): string {
    if (typeof value !== "string") return "";
    const el = document.createElement("div");
    el.innerHTML = value;
    return (el.textContent ?? "").replace(/\s+/g, " ").trim();
  }

  function truncate(value: string, max = 180): string {
    const chars = Array.from(value);
    return chars.length <= max ? value : `${chars.slice(0, max).join("")}...`;
  }

  function imageUrl(image: any): string {
    if (typeof image === "string") return image;
    return image?.url ?? image?.thumb ?? "";
  }

  function postExcerpt(post: any): string {
    const text = parsePostContent(post?.text)
      .filter((segment) => segment.kind === "text" || segment.kind === "html")
      .map((segment) => plainText(segment.value))
      .filter(Boolean)
      .join(" ");
    return truncate(text || plainText(post?.description) || "(无正文)");
  }

  function findComment(floors: any[], commentId: string): any | null {
    for (const floor of floors) {
      for (const comment of floor?.comment ?? []) {
        if (String(comment?.commentid ?? "") === commentId) return comment;
      }
    }
    return null;
  }

  function findFloorByRoot(floors: any[], rootCommentId: string): any | null {
    return floors.find((floor) => String(floor?.comment?.[0]?.commentid ?? "") === rootCommentId) ?? null;
  }

  function focusMessageInput() {
    setTimeout(() => messageInput?.focus(), 0);
  }

  async function loadCommentFromApi(
    linkId: string,
    commentId: string,
    rootCommentId: string,
    requestId: number,
  ): Promise<{ post: any; comment: any }> {
    let page = 1;
    let post: any = null;
    let targetFloor: any = null;

    while (page <= POST_DETAIL_PAGE_LIMIT) {
      const response = await postDetail({ link_id: linkId, page, is_first: page === 1 ? 1 : 0, limit: 50 });
      if (requestId !== deletePreviewRequestId) throw new Error("预览请求已取消");
      post ??= response?.result?.link;
      const floors = Array.isArray(response?.result?.comments) ? response.result.comments : [];
      const directMatch = findComment(floors, commentId);
      if (directMatch) return { post, comment: directMatch };
      targetFloor = findFloorByRoot(floors, rootCommentId);
      if (targetFloor || response?.result?.has_more_floors !== 1) break;
      if (page >= POST_DETAIL_PAGE_LIMIT) break;
      page++;
      await waitForPreviewRequest(requestId);
    }

    if (!post) throw new Error("未找到对应帖子");
    if (!targetFloor) throw new Error("未找到目标评论所属楼层");

    const knownComments = targetFloor?.comment ?? [];
    const knownMatch = findComment([targetFloor], commentId);
    if (knownMatch) return { post, comment: knownMatch };

    let lastval = knownComments.length > 0
      ? String(knownComments[knownComments.length - 1]?.commentid ?? "")
      : "";
    for (let pageIndex = 0; pageIndex < SUB_COMMENT_PAGE_LIMIT; pageIndex++) {
      const response = await subComments(rootCommentId, lastval);
      if (requestId !== deletePreviewRequestId) throw new Error("预览请求已取消");
      const comments = Array.isArray(response?.result?.comments) ? response.result.comments : [];
      const matched = comments.find((comment: any) => String(comment?.commentid ?? "") === commentId);
      if (matched) return { post, comment: matched };
      if (comments.length === 0) break;
      const nextLastval = String(response?.result?.lastval ?? comments[comments.length - 1]?.commentid ?? "");
      if (!nextLastval || nextLastval === lastval) break;
      lastval = nextLastval;
      if (pageIndex + 1 < SUB_COMMENT_PAGE_LIMIT) {
        await waitForPreviewRequest(requestId);
      }
    }

    throw new Error("未能从帖子评论中精确匹配目标评论");
  }

  function isDeleteConfirmation(req = pendingConfirmation): boolean {
    return req?.tool_name === "delete_post" || req?.tool_name === "delete_comment";
  }

  function deletePreviewUnavailable(): boolean {
    return isDeleteConfirmation() && !deletePreview;
  }

  async function loadDeletePreview(req: AgentToolConfirmationRequest) {
    resetDeletePreview();
    if (req.tool_name !== "delete_post" && req.tool_name !== "delete_comment") return;

    const requestId = deletePreviewRequestId;
    deletePreviewLoading = true;
    try {
      const args = JSON.parse(req.arguments_json);
      const linkId = String(args?.link_id ?? "");
      if (!linkId) throw new Error("未提供帖子 ID，无法加载删除预览");

      let response: any = null;
      let post: any = null;
      let matchedComment: any = null;
      if (req.tool_name === "delete_comment") {
        const commentId = String(args?.comment_id ?? "");
        const rootCommentId = String(args?.root_comment_id ?? "");
        if (!commentId || !rootCommentId) throw new Error("未提供评论 ID 或根评论 ID");
        const verified = await loadCommentFromApi(linkId, commentId, rootCommentId, requestId);
        post = verified.post;
        matchedComment = verified.comment;
      } else {
        response = await postDetail({ link_id: linkId, is_first: 1, limit: 1 });
        if (requestId !== deletePreviewRequestId) return;
        post = response?.result?.link;
        if (!post) throw new Error("未找到对应帖子");
      }

      const segments = parsePostContent(post?.text);
      const contentImage = segments
        .filter((segment) => segment.kind === "images")
        .flatMap((segment) => segment.urls)[0] ?? "";
      const preview: DeletePreview = {
        post: {
          linkId,
          title: post?.title || "(无标题)",
          author: post?.user?.username || "?",
          excerpt: postExcerpt(post),
          image: contentImage || imageUrl(post?.imgs?.[0]),
        },
      };

      if (req.tool_name === "delete_comment") {
        const commentId = String(args?.comment_id ?? "");
        preview.comment = {
          commentId,
          author: matchedComment?.user?.username || "?",
          text: matchedComment?.text || "(无评论文本)",
          image: imageUrl(matchedComment?.imgs?.[0]),
          verified: true,
        };
      }

      deletePreview = preview;
    } catch (error) {
      if (requestId === deletePreviewRequestId) {
        deletePreviewError = error instanceof Error ? error.message : String(error);
      }
    } finally {
      if (requestId === deletePreviewRequestId) deletePreviewLoading = false;
    }
  }

  function scrollBottom() {
    setTimeout(() => {
      scrollEl?.scrollTo({ top: scrollEl.scrollHeight, behavior: "smooth" });
    }, 30);
  }

  async function persist() {
    const ui = messages.filter(m => !m.streaming && !m.error).map(toUiMsg);
    try {
      await agentHistorySave(ui);
      // 标题可能在后端自动更新了，刷新会话列表
      await refreshSessions();
    } catch { /* ignore */ }
  }

  async function refreshSessions() {
    try {
      sessions = await agentSessionList();
      // active_id 可能因新建/删除而变化，重新读一次
      const newActive = await (await import("../lib/api")).agentSessionActive();
      activeSessionId = newActive;
    } catch { /* ignore */ }
  }

  async function refreshTemplates() {
    try {
      templates = await agentTemplateList();
    } catch { /* ignore */ }
  }

  async function handleCreateSession() {
    if (busy) return;
    try {
      const id = await agentSessionCreate();
      activeSessionId = id;
      messages = [];
      await refreshSessions();
      focusMessageInput();
    } catch (e) {
      console.error("create session failed:", e);
    }
  }

  async function handleSelectSession(id: string) {
    if (busy || id === activeSessionId) return;
    try {
      const uiMsgs = await agentSessionSwitch(id);
      activeSessionId = id;
      messages = uiMsgs.map(m => ({
        role: m.role as "user" | "assistant",
        text: m.text,
        tools: m.tools,
        loops: m.loops,
        streaming: false,
        error: false,
      }));
      scrollBottom();
    } catch (e) {
      console.error("switch session failed:", e);
    }
  }

  async function handleRenameSession(id: string, title: string) {
    try {
      await agentSessionRename(id, title);
      await refreshSessions();
    } catch (e) {
      console.error("rename session failed:", e);
    }
  }

  async function handleDeleteSession(id: string) {
    if (busy) return;
    try {
      const newActiveId = await agentSessionDelete(id);
      activeSessionId = newActiveId;
      // 重新加载当前活跃会话的 messages
      const uiMsgs = await agentHistoryGet();
      messages = uiMsgs.map(m => ({
        role: m.role as "user" | "assistant",
        text: m.text,
        tools: m.tools,
        loops: m.loops,
        streaming: false,
        error: false,
      }));
      await refreshSessions();
      scrollBottom();
    } catch (e) {
      console.error("delete session failed:", e);
    }
  }

  function toggleTemplates() {
    templatesCollapsed = !templatesCollapsed;
    saveTemplatesCollapsed(templatesCollapsed);
  }

  function handleInsertTemplate(t: AgentTemplate) {
    input = t.content;
    setTimeout(() => messageInput?.focus(), 0);
  }

  function handleEditTemplate(t: AgentTemplate) {
    editingTemplate = t;
    templateEditorOpen = true;
  }

  function handleCreateTemplate() {
    editingTemplate = null;
    templateEditorOpen = true;
  }

  async function handleSaveAsTemplate() {
    const text = input.trim();
    if (!text) return;
    try {
      await agentTemplateSave("", text);
      await refreshTemplates();
      showTemplateToast("已保存为模板");
    } catch (e) {
      console.error("save as template failed:", e);
    }
  }

  async function handleCommitTemplate(title: string, content: string, id: string | null) {
    await agentTemplateSave(title, content, id ?? undefined);
    await refreshTemplates();
    templateEditorOpen = false;
    editingTemplate = null;
    showTemplateToast(id ? "模板已更新" : "模板已创建");
  }

  async function handleDeleteTemplate(id: string) {
    await agentTemplateDelete(id);
    await refreshTemplates();
    templateEditorOpen = false;
    editingTemplate = null;
    showTemplateToast("模板已删除");
  }

  function showTemplateToast(text: string) {
    templateToast = text;
    setTimeout(() => { templateToast = ""; }, 2000);
  }

  function startAgentStream(text: string, confirmations: AgentToolConfirmationDecision[] = []) {
    void agentChatStream(
      text,
      (chunk) => {
        const aiIdx = messages.length - 1;
        messages[aiIdx] = { ...messages[aiIdx], text: messages[aiIdx].text + chunk };
        scrollBottom();
      },
      (name) => {
        const aiIdx = messages.length - 1;
        const m = messages[aiIdx];
        const tools = [...(m.tools ?? []), name];
        messages[aiIdx] = { ...m, tools };
        scrollBottom();
      },
      (info: AgentStreamDone) => {
        const aiIdx = messages.length - 1;
        const m = messages[aiIdx];
        messages[aiIdx] = { ...m, streaming: false, text: m.text || "(无文本输出)", loops: info.loops_used };
        busy = false;
        pendingMessage = "";
        pendingConfirmation = null;
        approvedConfirmations = [];
        autoApproveQueued = null;
        resetDeletePreview();
        scrollBottom();
        persist();
        focusMessageInput();
      },
      (err) => {
        const aiIdx = messages.length - 1;
        const isConfirmationWait = err.includes("危险操作等待确认");
        if (isConfirmationWait && (pendingConfirmation || autoApproveQueued)) {
          if (autoApproveQueued) {
            // 自动审批：旧流已清理监听，此处安全地续发批准并重新发起流
            const req = autoApproveQueued;
            autoApproveQueued = null;
            submitConfirmation(req, true);
            return;
          }
          // 手动确认：隐藏思考气泡，等待用户操作
          const current = messages[aiIdx];
          if (current?.text.trim()) {
            messages[aiIdx] = { ...current, streaming: false };
          } else {
            messages.pop();
          }
          busy = false;
          scrollBottom();
          persist();
          return;
        }
        messages[aiIdx] = { role: "assistant", text: err, error: true, streaming: false };
        busy = false;
        autoApproveQueued = null;
        pendingConfirmation = null;
        scrollBottom();
        persist();
        focusMessageInput();
      },
      (req) => {
        pendingMessage = text;
        if (autoApprove) {
          // 自动审批：暂存请求，不弹框；待旧流报错清理后续发批准
          autoApproveQueued = req;
          return;
        }
        pendingConfirmation = req;
        void loadDeletePreview(req);
        setTimeout(() => cancelConfirmationButton?.focus(), 0);
      },
      confirmations,
    );
  }

  async function send() {
    const text = input.trim();
    if (!text || busy) return;
    busy = true;
    input = "";
    pendingConfirmation = null;
    pendingMessage = "";
    approvedConfirmations = [];
    autoApproveQueued = null;
    operationNotice = "";
    resetDeletePreview();
    messages.push({ role: "user", text });
    scrollBottom();

    messages.push({ role: "assistant", text: "", streaming: true });
    scrollBottom();

    startAgentStream(text);
  }

  function submitConfirmation(req: AgentToolConfirmationRequest, approved: boolean) {
    pendingConfirmation = null;
    resetDeletePreview();
    busy = true;
    if (approved) {
      operationNotice = "";
    } else {
      showOperationNotice(`已取消：${req.summary}`);
    }
    const decision: AgentToolConfirmationDecision = {
      tool_name: req.tool_name,
      arguments_json: req.arguments_json,
      approved,
    };
    approvedConfirmations = [...approvedConfirmations, decision];
    const lastIdx = messages.length - 1;
    const lastMessage = messages[lastIdx];
    if (lastMessage?.role === "assistant" && !lastMessage.error) {
      messages[lastIdx] = { ...lastMessage, streaming: true };
    } else {
      messages.push({ role: "assistant", text: "", streaming: true });
    }
    scrollBottom();
    startAgentStream(pendingMessage, approvedConfirmations);
  }

  function approveDangerousTool() {
    if (!pendingConfirmation || !pendingMessage || busy || deletePreviewLoading || deletePreviewUnavailable()) return;
    submitConfirmation(pendingConfirmation, true);
  }

  function denyDangerousTool() {
    if (!pendingConfirmation || busy) return;
    submitConfirmation(pendingConfirmation, false);
  }

  async function reset() {
    if (busy) return;
    try {
      await agentReset();
      messages = [];
      await refreshSessions();
    } catch (e) {
      console.error(e);
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      send();
    }
  }

  function handleWindowKeydown(e: KeyboardEvent) {
    // Agent 常驻挂载后，svelte:window 监听器全局生效；
    // 仅在当前位于 Agent 视图时响应 Escape，避免切到其他页面后误触发取消
    if (getView() !== "agent") return;
    if (e.key === "Escape" && pendingConfirmation && !busy) {
      e.preventDefault();
      denyDangerousTool();
    }
  }

  function riskLabel(risk: AgentToolConfirmationRequest["risk_level"]) {
    const labels = {
      Low: "低风险",
      Medium: "中风险",
      High: "高风险",
    } as const;
    return labels[risk];
  }

  onMount(async () => {
    preloadEmoji();
    templatesCollapsed = loadTemplatesCollapsed();
    autoApprove = loadAutoApprove();
    try {
      const [sessionList, tplList] = await Promise.all([agentSessionList(), agentTemplateList()]);
      sessions = sessionList;
      templates = tplList;
      activeSessionId = await (await import("../lib/api")).agentSessionActive();
      // 加载当前活跃会话的 UI 历史
      const saved = await agentHistoryGet();
      if (saved && saved.length > 0) {
        messages = saved.map(m => ({
          ...m,
          role: m.role as "user" | "assistant",
          streaming: false,
          error: false,
        }));
        scrollBottom();
      }
    } catch { /* ignore */ }
  });

  onDestroy(() => {
    if (operationNoticeTimer) clearTimeout(operationNoticeTimer);
  });
</script>

<svelte:window onkeydown={handleWindowKeydown} />

<div class="agent-layout" inert={pendingConfirmation !== null}>
  <AgentSidebar
    {sessions}
    activeId={activeSessionId}
    {busy}
    {templates}
    {templatesCollapsed}
    onToggleTemplates={toggleTemplates}
    onCreateSession={handleCreateSession}
    onSelectSession={handleSelectSession}
    onRenameSession={handleRenameSession}
    onDeleteSession={handleDeleteSession}
    onInsertTemplate={handleInsertTemplate}
    onEditTemplate={handleEditTemplate}
    onCreateTemplate={handleCreateTemplate}
    onDeleteTemplate={(id: string) => agentTemplateDelete(id).then(refreshTemplates)}
  />

  <div class="agent-page">
  <section class="agent-hero">
    <div class="hero-copy">
      <div class="hero-title">
        <h1>Agent 工作台</h1>
        <span class="eyebrow">AI Operations</span>
      </div>
      <p>多轮对话、上下文记忆与社区工具调用。</p>
    </div>
    <button class="reset-btn" onclick={reset} disabled={busy || messages.length === 0}>清空会话</button>
  </section>

  <section class="chat-shell">
    <div class="chat-toolbar">
      <div>
        <strong>实时会话</strong>
        <span>{messages.length} 条消息 · Enter 发送，Shift+Enter 换行</span>
        {#if operationNotice}
          <span class="operation-notice" aria-live="polite">{operationNotice}</span>
        {/if}
      </div>
      <div class="toolbar-controls">
        <button
          type="button"
          class="auto-approve-toggle"
          class:on={autoApprove}
          onclick={toggleAutoApprove}
          aria-pressed={autoApprove}
          title={autoApprove
            ? "自动审批已开启：工具调用将自动执行，不再逐次确认"
            : "开启自动审批：跳过工具调用确认"}
        >
          <span class="auto-approve-dot" aria-hidden="true"></span>
          自动审批{autoApprove ? "：开" : "：关"}
        </button>
        <div class="status-pill" class:busy={busy}>
          <span></span>
          {busy ? "执行中" : "待命"}
        </div>
      </div>
    </div>

    <div class="chat-area" bind:this={scrollEl}>
      {#if messages.length === 0}
        <div class="empty">
          <div class="empty-mark" aria-hidden="true">
            <svg width="30" height="30" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 2v4"/><path d="M12 18v4"/><path d="m4.93 4.93 2.83 2.83"/><path d="m16.24 16.24 2.83 2.83"/><path d="M2 12h4"/><path d="M18 12h4"/><path d="m4.93 19.07 2.83-2.83"/><path d="m16.24 7.76 2.83-2.83"/></svg>
          </div>
          <h2>开始一个自动化任务</h2>
          <p>例如：帮我搜索原神相关的热帖，并总结适合评论互动的切入点。</p>
        </div>
      {:else}
        {#each messages as m}
          <div class="msg {m.role}">
            <div class="avatar" aria-hidden="true">{m.role === "assistant" ? "A" : "我"}</div>
            <div class="bubble" class:error={m.error}>
              {#if m.role === "assistant"}
                <div class="md-body">{@html renderAiMarkdown(m.text)}{#if m.streaming}<span class="cursor"></span>{/if}</div>
              {:else}
                <div class="text">{m.text}</div>
              {/if}
              {#if m.tools && m.tools.length > 0}
                <div class="tools">
                  <span class="tools-label">工具调用 {m.tools.length}</span>
                  {#each m.tools as t}
                    <span class="tool-tag">{t}</span>
                  {/each}
                </div>
              {/if}
              {#if m.loops !== undefined}
                <div class="loops">{m.loops} 轮执行</div>
              {/if}
            </div>
          </div>
        {/each}
      {/if}
      {#if busy && messages.length > 0 && !messages[messages.length - 1]?.streaming}
        <div class="msg assistant">
          <div class="avatar" aria-hidden="true">A</div>
          <div class="bubble thinking">思考中…</div>
        </div>
      {/if}
    </div>

    <div class="input-area">
      <textarea
        bind:this={messageInput}
        bind:value={input}
        onkeydown={handleKeydown}
        placeholder="输入消息或运营目标…"
        rows="2"
        aria-label="Agent 消息输入"
      ></textarea>
      <button
        class="save-tpl-btn"
        onclick={handleSaveAsTemplate}
        disabled={busy || !input.trim()}
        title="将输入框内容保存为模板"
      >
        存为模板
      </button>
      <button class="send-btn" onclick={send} disabled={busy || !input.trim()}>
        {busy ? "执行中" : "发送"}
      </button>
    </div>
  </section>
  </div><!-- /.agent-page -->
</div><!-- /.agent-layout -->

{#if templateToast}
  <div class="tpl-toast" role="status" aria-live="polite">{templateToast}</div>
{/if}

{#if templateEditorOpen}
  <TemplateEditor
    template={editingTemplate}
    onCommit={handleCommitTemplate}
    onDelete={handleDeleteTemplate}
    onClose={() => { templateEditorOpen = false; editingTemplate = null; }}
  />
{/if}

{#if pendingConfirmation}
  <div class="confirm-backdrop">
    <div
      class="confirm-dialog"
      role="alertdialog"
      aria-modal="true"
      aria-labelledby="confirm-title"
      aria-describedby="confirm-summary"
    >
      <div class="confirm-icon" aria-hidden="true">
        <svg width="26" height="26" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M10.3 2.9 1.8 17a2 2 0 0 0 1.7 3h17a2 2 0 0 0 1.7-3L13.7 2.9a2 2 0 0 0-3.4 0Z"/>
          <path d="M12 9v4"/>
          <path d="M12 17h.01"/>
        </svg>
      </div>

      <div class="confirm-content">
        <span class="confirm-eyebrow">工具调用确认</span>
        <h2 id="confirm-title">此操作需要你的确认</h2>
        <p id="confirm-summary">{pendingConfirmation.summary}</p>

        <div class="confirm-meta">
          <div>
            <span>调用工具</span>
            <strong>{pendingConfirmation.tool_name}</strong>
          </div>
          <div>
            <span>风险等级</span>
            <strong class="risk-level">{riskLabel(pendingConfirmation.risk_level)}</strong>
          </div>
        </div>

        {#if pendingConfirmation.tool_name === "delete_post" || pendingConfirmation.tool_name === "delete_comment"}
          <div class="delete-preview" aria-live="polite">
            <div class="delete-preview-heading">
              <strong>删除内容预览</strong>
              <span>请确认这是你准备永久删除的内容</span>
            </div>

            {#if deletePreviewLoading}
              <div class="preview-loading">
                <span aria-hidden="true"></span>
                正在加载帖子和评论…
              </div>
            {:else if deletePreview}
              <article class="preview-post">
                {#if deletePreview.post.image}
                  <img src={deletePreview.post.image} alt="" />
                {/if}
                <div>
                  <span class="preview-type">帖子 · {deletePreview.post.author}</span>
                  <h3>{deletePreview.post.title}</h3>
                  <p>{deletePreview.post.excerpt}</p>
                  <small>ID: {deletePreview.post.linkId}</small>
                </div>
              </article>

              {#if deletePreview.comment}
                <article class="preview-comment">
                  <div class="preview-comment-meta">
                    <strong>{deletePreview.comment.author}</strong>
                    <span>已从帖子验证</span>
                  </div>
                  <p>{deletePreview.comment.text}</p>
                  {#if deletePreview.comment.image}
                    <img src={deletePreview.comment.image} alt="待删除评论中的图片" />
                  {/if}
                  <small>评论 ID: {deletePreview.comment.commentId}</small>
                </article>
              {/if}
            {:else if deletePreviewError}
              <div class="preview-error">
                <strong>预览加载失败，已阻止删除</strong>
                <span>{deletePreviewError}。请取消后让 Agent 重新查询目标内容。</span>
              </div>
            {/if}
          </div>
        {/if}

        <details class="confirm-arguments">
          <summary>查看调用参数</summary>
          <pre>{pendingConfirmation.arguments_json}</pre>
        </details>
      </div>

      <div class="confirm-actions">
        <button
          class="deny-btn"
          bind:this={cancelConfirmationButton}
          onclick={denyDangerousTool}
          disabled={busy}
        >取消</button>
        <button class="approve-btn" onclick={approveDangerousTool} disabled={busy || deletePreviewLoading || deletePreviewUnavailable()}>
          {deletePreviewLoading ? "加载预览中" : deletePreviewUnavailable() ? "预览不可用" : "确认执行"}
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  .agent-layout {
    display: flex;
    width: 100%;
    height: 100%;
    min-height: 0;
  }

  .agent-page {
    flex: 1;
    min-width: 0;
    max-width: 1080px;
    margin: 0 auto;
    padding: 0 18px;
    display: flex;
    flex-direction: column;
    height: 100%;
    min-height: 0;
  }

  .agent-hero {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    padding: 14px 18px;
    margin-bottom: 12px;
    border-radius: 20px;
    background:
      radial-gradient(circle at 14% 8%, color-mix(in srgb, var(--accent) 20%, transparent), transparent 34%),
      radial-gradient(circle at 86% 16%, color-mix(in srgb, var(--accent-warm) 12%, transparent), transparent 30%),
      linear-gradient(135deg, color-mix(in srgb, var(--bg-soft) 86%, transparent), color-mix(in srgb, var(--bg-soft) 57%, transparent));
    border: 1px solid var(--glass-border);
    box-shadow: var(--elevation-2);
    backdrop-filter: var(--glass-blur);
    -webkit-backdrop-filter: var(--glass-blur);
  }

  .hero-copy {
    min-width: 0;
  }

  .hero-title {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .eyebrow {
    display: inline-flex;
    padding: 4px 8px;
    border-radius: 999px;
    background: var(--accent-soft);
    border: 1px solid color-mix(in srgb, var(--accent-hover) 20%, transparent);
    color: #bfdbfe;
    font-size: 10px;
    font-weight: 800;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    white-space: nowrap;
  }

  h1 {
    font-size: 23px;
    font-weight: 850;
    line-height: 1.2;
    letter-spacing: -0.025em;
    color: var(--text-strong);
  }

  .hero-copy > p {
    margin-top: 4px;
    color: var(--text-secondary);
    font-size: 12px;
    line-height: 1.5;
  }

  .reset-btn,
  .send-btn {
    min-height: 44px;
    border-radius: 15px;
    font-size: 14px;
    font-weight: 750;
    transition: transform var(--duration-fast) var(--ease-out), background var(--duration-fast) var(--ease-out), filter var(--duration-fast) var(--ease-out), border-color var(--duration-fast) var(--ease-out);
  }

  .reset-btn {
    padding: 0 14px;
    background: rgba(148, 163, 184, 0.1);
    color: var(--text-secondary);
    border: 1px solid rgba(148, 163, 184, 0.16);
  }

  .reset-btn:hover:not(:disabled) {
    background: var(--danger-soft);
    color: #fecaca;
    border-color: rgba(248, 113, 113, 0.22);
  }

  button:disabled {
    opacity: 0.5;
  }

  .chat-shell {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    border-radius: 28px;
    background: linear-gradient(180deg, color-mix(in srgb, var(--bg-soft) 76%, transparent), color-mix(in srgb, var(--bg-soft) 57%, transparent));
    border: 1px solid var(--glass-border);
    box-shadow: var(--elevation-2);
    backdrop-filter: var(--glass-blur);
    -webkit-backdrop-filter: var(--glass-blur);
  }

  .chat-toolbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    padding: 16px 18px;
    border-bottom: 1px solid rgba(148, 163, 184, 0.14);
    background: color-mix(in srgb, var(--bg) 22%, transparent);
  }

  .chat-toolbar strong {
    display: block;
    font-size: 15px;
    color: var(--text-strong);
  }

  .chat-toolbar span {
    display: block;
    margin-top: 3px;
    font-size: 12px;
    color: var(--text-muted);
  }

  .chat-toolbar .operation-notice {
    color: #fcd34d;
    font-weight: 650;
  }

  .status-pill {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    min-height: 32px;
    padding: 0 11px;
    border-radius: 999px;
    background: var(--success-soft);
    color: #bbf7d0;
    border: 1px solid rgba(34, 197, 94, 0.18);
    font-size: 12px;
    font-weight: 800;
  }

  .status-pill.busy {
    background: var(--accent-soft);
    color: #bfdbfe;
    border-color: color-mix(in srgb, var(--accent-hover) 20%, transparent);
  }

  .status-pill span {
    width: 7px;
    height: 7px;
    margin: 0;
    border-radius: 50%;
    background: currentColor;
  }

  .toolbar-controls {
    display: flex;
    align-items: center;
    gap: 10px;
    flex-shrink: 0;
  }

  .auto-approve-toggle {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    min-height: 32px;
    padding: 0 12px;
    border-radius: 999px;
    background: rgba(148, 163, 184, 0.1);
    color: var(--text-secondary);
    border: 1px solid rgba(148, 163, 184, 0.16);
    font-size: 12px;
    font-weight: 750;
    cursor: pointer;
    transition: background var(--duration-fast) var(--ease-out), color var(--duration-fast) var(--ease-out), border-color var(--duration-fast) var(--ease-out);
  }

  .auto-approve-toggle:hover {
    color: var(--text-strong);
    border-color: rgba(148, 163, 184, 0.3);
  }

  .auto-approve-toggle:focus-visible {
    outline: 3px solid rgba(251, 191, 36, 0.55);
    outline-offset: 2px;
  }

  .auto-approve-dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: rgba(148, 163, 184, 0.5);
    transition: background var(--duration-fast) var(--ease-out), box-shadow var(--duration-fast) var(--ease-out);
  }

  .auto-approve-toggle.on {
    background: rgba(251, 191, 36, 0.16);
    color: #fcd34d;
    border-color: rgba(251, 191, 36, 0.4);
  }

  .auto-approve-toggle.on .auto-approve-dot {
    background: #fbbf24;
    box-shadow: 0 0 0 3px rgba(251, 191, 36, 0.22);
  }

  .chat-area {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding: 22px;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .empty {
    margin: auto;
    max-width: 430px;
    text-align: center;
    color: var(--text-secondary);
  }

  .empty-mark {
    width: 68px;
    height: 68px;
    display: grid;
    place-items: center;
    margin: 0 auto 18px;
    border-radius: 24px;
    color: #bfdbfe;
    background: var(--accent-soft);
    border: 1px solid color-mix(in srgb, var(--accent-hover) 18%, transparent);
  }

  .empty h2 {
    font-size: 22px;
    font-weight: 850;
    color: var(--text-strong);
  }

  .msg {
    display: flex;
    align-items: flex-start;
    gap: 10px;
  }

  .msg.user {
    flex-direction: row-reverse;
  }

  .avatar {
    width: 34px;
    height: 34px;
    display: grid;
    place-items: center;
    flex-shrink: 0;
    border-radius: 13px;
    background: rgba(148, 163, 184, 0.12);
    color: var(--text-secondary);
    font-size: 12px;
    font-weight: 850;
  }

  .msg.assistant .avatar {
    background: linear-gradient(135deg, var(--accent), var(--accent-warm));
    color: white;
  }

  .bubble {
    max-width: min(760px, 82%);
    padding: 14px 16px;
    border-radius: 18px;
    background: color-mix(in srgb, var(--bg-soft) 72%, transparent);
    border: 1px solid rgba(148, 163, 184, 0.16);
    box-shadow: 0 12px 28px rgba(2, 6, 23, 0.14);
    font-size: 14px;
    line-height: 1.75;
  }

  .msg.user .bubble {
    background: linear-gradient(135deg, var(--accent), var(--accent-strong));
    border-color: rgba(191, 219, 254, 0.22);
    color: white;
  }

  .bubble.error {
    color: #fecaca;
    background: var(--danger-soft);
    border-color: rgba(248, 113, 113, 0.24);
  }

  .text {
    white-space: pre-wrap;
    word-break: break-word;
  }

  .tools {
    display: flex;
    align-items: center;
    gap: 7px;
    flex-wrap: wrap;
    margin-top: 12px;
    padding-top: 12px;
    border-top: 1px solid rgba(148, 163, 184, 0.14);
  }

  .tools-label,
  .loops {
    font-size: 11px;
    color: var(--text-muted);
  }

  .tool-tag {
    padding: 3px 8px;
    border-radius: 999px;
    background: var(--accent-soft);
    color: #bfdbfe;
    border: 1px solid color-mix(in srgb, var(--accent-hover) 18%, transparent);
    font-size: 11px;
    font-weight: 750;
  }

  .loops {
    margin-top: 8px;
  }

  .thinking {
    color: var(--text-secondary);
    font-style: italic;
  }

  .cursor {
    display: inline-block;
    width: 2px;
    height: 1.1em;
    background: var(--accent-hover);
    margin-left: 2px;
    vertical-align: text-bottom;
    animation: blink 1s step-end infinite;
  }

  @keyframes blink {
    50% { opacity: 0; }
  }

  .md-body :global(p) { margin: 0 0 8px; }
  .md-body :global(p:last-child) { margin-bottom: 0; }
  .md-body :global(.ai-md-h1) { font-size: 18px; font-weight: 850; margin: 12px 0 6px; line-height: 1.4; color: var(--text-strong); }
  .md-body :global(.ai-md-h2) { font-size: 16px; font-weight: 800; margin: 10px 0 5px; line-height: 1.4; padding-bottom: 4px; border-bottom: 1px solid rgba(148, 163, 184, 0.14); color: var(--text-strong); }
  .md-body :global(.ai-md-h3) { font-size: 15px; font-weight: 800; margin: 8px 0 4px; color: var(--text-strong); }
  .md-body :global(.ai-md-h4),
  .md-body :global(.ai-md-h5),
  .md-body :global(.ai-md-h6) { font-size: 14px; font-weight: 750; margin: 6px 0 2px; }
  .md-body :global(.ai-md-ul),
  .md-body :global(.ai-md-ol) { margin: 4px 0 8px; padding-left: 20px; }
  .md-body :global(.ai-md-ul) { list-style: disc; }
  .md-body :global(.ai-md-ol) { list-style: decimal; }
  .md-body :global(.ai-md-ul li),
  .md-body :global(.ai-md-ol li) { margin: 3px 0; line-height: 1.75; }
  .md-body :global(.ai-md-blockquote) { margin: 8px 0; padding: 8px 12px; border-left: 3px solid var(--accent); background: color-mix(in srgb, var(--accent) 8%, transparent); border-radius: 0 10px 10px 0; color: var(--text-secondary); font-size: 13px; }
  .md-body :global(.ai-md-pre) { margin: 9px 0; padding: 13px 14px; border-radius: 12px; background: rgba(2, 6, 23, 0.55); overflow-x: auto; border: 1px solid rgba(148, 163, 184, 0.12); }
  .md-body :global(.ai-md-pre code) { font-size: 13px; font-family: "Cascadia Code", "JetBrains Mono", monospace; line-height: 1.6; }
  .md-body :global(.ai-md-code) { padding: 2px 6px; border-radius: 6px; background: rgba(148, 163, 184, 0.12); font-size: 13px; font-family: "Cascadia Code", "JetBrains Mono", monospace; }
  .md-body :global(.ai-md-link) { color: #93c5fd; text-decoration: none; }
  .md-body :global(.ai-md-link:hover) { text-decoration: underline; }
  .md-body :global(.ai-md-hr) { border: none; border-top: 1px solid rgba(148, 163, 184, 0.16); margin: 12px 0; }
  .md-body :global(.ai-md-table-wrap) { margin: 10px 0; overflow-x: auto; border: 1px solid rgba(148, 163, 184, 0.16); border-radius: 10px; }
  .md-body :global(.ai-md-table) { width: 100%; min-width: max-content; border-collapse: collapse; font-size: 13px; }
  .md-body :global(.ai-md-table th),
  .md-body :global(.ai-md-table td) { padding: 8px 11px; border-right: 1px solid rgba(148, 163, 184, 0.14); border-bottom: 1px solid rgba(148, 163, 184, 0.14); vertical-align: top; }
  .md-body :global(.ai-md-table th:last-child),
  .md-body :global(.ai-md-table td:last-child) { border-right: none; }
  .md-body :global(.ai-md-table tbody tr:last-child td) { border-bottom: none; }
  .md-body :global(.ai-md-table th) { background: rgba(148, 163, 184, 0.1); color: var(--text-strong); font-weight: 750; }
  .md-body :global(.ai-md-table tbody tr:nth-child(even)) { background: rgba(148, 163, 184, 0.04); }
  .md-body :global(.ai-md-align-left) { text-align: left; }
  .md-body :global(.ai-md-align-center) { text-align: center; }
  .md-body :global(.ai-md-align-right) { text-align: right; }
  .md-body :global(.emoji) { width: 1.35em; height: 1.35em; margin: 0 2px; object-fit: contain; vertical-align: -0.28em; }
  .md-body :global(strong) { font-weight: 800; color: var(--text-strong); }
  .md-body :global(del) { opacity: 0.55; }

  .input-area {
    display: flex;
    gap: 12px;
    padding: 16px;
    border-top: 1px solid rgba(148, 163, 184, 0.14);
    background: color-mix(in srgb, var(--bg) 28%, transparent);
  }

  .save-tpl-btn {
    align-self: stretch;
    padding: 0 16px;
    border-radius: 14px;
    background: rgba(148, 163, 184, 0.12);
    color: var(--text-secondary);
    border: 1px solid rgba(148, 163, 184, 0.18);
    font-size: 13px;
    font-weight: 650;
    transition: background var(--duration-fast) var(--ease-out), color var(--duration-fast) var(--ease-out), border-color var(--duration-fast) var(--ease-out);
  }

  .save-tpl-btn:hover:not(:disabled) {
    background: rgba(148, 163, 184, 0.2);
    color: var(--text-strong);
    border-color: rgba(148, 163, 184, 0.32);
  }

  .save-tpl-btn:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }

  .tpl-toast {
    position: fixed;
    bottom: 24px;
    left: 50%;
    transform: translateX(-50%);
    z-index: 1200;
    padding: 10px 18px;
    border-radius: 12px;
    background: color-mix(in srgb, var(--accent) 18%, var(--bg-soft));
    color: var(--text-strong);
    border: 1px solid color-mix(in srgb, var(--accent-hover) 32%, transparent);
    font-size: 13px;
    font-weight: 600;
    box-shadow: var(--elevation-2);
    animation: tpl-toast-in 200ms var(--ease-out);
  }

  @keyframes tpl-toast-in {
    from { opacity: 0; transform: translate(-50%, 8px); }
  }

  .confirm-backdrop {
    position: fixed;
    inset: 0;
    z-index: 1000;
    display: grid;
    place-items: center;
    padding: 24px;
    background: rgba(2, 6, 23, 0.68);
    backdrop-filter: blur(10px);
    -webkit-backdrop-filter: blur(10px);
    overscroll-behavior: contain;
    animation: backdrop-in 180ms var(--ease-out);
  }

  .confirm-dialog {
    width: min(520px, 100%);
    max-height: min(720px, calc(100vh - 48px));
    overflow-y: auto;
    padding: 26px;
    border-radius: 26px;
    background:
      radial-gradient(circle at 100% 0, color-mix(in srgb, var(--accent-warm) 12%, transparent), transparent 34%),
      linear-gradient(160deg, color-mix(in srgb, var(--bg-soft) 98%, transparent), color-mix(in srgb, var(--bg-soft) 98%, transparent));
    border: 1px solid rgba(248, 113, 113, 0.28);
    box-shadow: 0 28px 80px rgba(0, 0, 0, 0.48);
    animation: dialog-in 220ms var(--ease-out);
  }

  .confirm-icon {
    width: 52px;
    height: 52px;
    display: grid;
    place-items: center;
    margin-bottom: 18px;
    border-radius: 18px;
    color: #fecaca;
    background: var(--danger-soft);
    border: 1px solid rgba(248, 113, 113, 0.24);
  }

  .confirm-eyebrow {
    color: #fca5a5;
    font-size: 12px;
    font-weight: 850;
    letter-spacing: 0.08em;
    text-transform: uppercase;
  }

  .confirm-content h2 {
    margin-top: 8px;
    color: var(--text-strong);
    font-size: 24px;
    line-height: 1.25;
  }

  .confirm-content > p {
    margin: 12px 0 0;
    color: var(--text-strong);
    font-size: 15px;
    line-height: 1.7;
    overflow-wrap: anywhere;
  }

  .confirm-meta {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 10px;
    margin-top: 20px;
  }

  .confirm-meta div {
    min-width: 0;
    padding: 13px 14px;
    border-radius: 15px;
    background: rgba(148, 163, 184, 0.08);
    border: 1px solid rgba(148, 163, 184, 0.12);
  }

  .confirm-meta span,
  .confirm-meta strong {
    display: block;
  }

  .confirm-meta span {
    font-size: 12px;
    color: var(--text-muted);
  }

  .confirm-meta strong {
    margin-top: 5px;
    color: var(--text-strong);
    font-size: 13px;
    overflow-wrap: anywhere;
  }

  .confirm-meta .risk-level {
    color: #fca5a5;
  }

  .delete-preview {
    margin-top: 16px;
    padding: 14px;
    border-radius: 18px;
    background: rgba(2, 6, 23, 0.34);
    border: 1px solid rgba(248, 113, 113, 0.18);
  }

  .delete-preview-heading {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 12px;
    margin-bottom: 12px;
  }

  .delete-preview-heading strong {
    color: #fecaca;
    font-size: 13px;
  }

  .delete-preview-heading span {
    color: var(--text-muted);
    font-size: 11px;
    text-align: right;
  }

  .preview-loading,
  .preview-error {
    min-height: 68px;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 10px;
    border-radius: 14px;
    background: rgba(148, 163, 184, 0.06);
    color: var(--text-secondary);
    font-size: 13px;
  }

  .preview-loading > span {
    width: 16px;
    height: 16px;
    border-radius: 50%;
    border: 2px solid rgba(148, 163, 184, 0.24);
    border-top-color: #fca5a5;
    animation: preview-spin 700ms linear infinite;
  }

  .preview-error {
    flex-direction: column;
    align-items: flex-start;
    padding: 13px;
    color: #fecaca;
  }

  .preview-error span {
    color: var(--text-secondary);
  }

  .preview-post {
    display: grid;
    grid-template-columns: minmax(0, 1fr);
    gap: 12px;
    padding: 14px;
    border-radius: 15px;
    background: rgba(148, 163, 184, 0.07);
    border: 1px solid rgba(148, 163, 184, 0.12);
  }

  .preview-post:has(> img) {
    grid-template-columns: 92px minmax(0, 1fr);
  }

  .preview-post > img {
    width: 92px;
    height: 92px;
    object-fit: cover;
    border-radius: 12px;
  }

  .preview-type,
  .preview-post small,
  .preview-comment small {
    color: var(--text-muted);
    font-size: 11px;
  }

  .preview-post h3 {
    margin-top: 4px;
    color: var(--text-strong);
    font-size: 15px;
    line-height: 1.45;
  }

  .preview-post p,
  .preview-comment p {
    margin: 7px 0;
    color: var(--text-secondary);
    font-size: 13px;
    line-height: 1.6;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }

  .preview-comment {
    margin-top: 10px;
    padding: 14px;
    border-radius: 15px;
    background: rgba(248, 113, 113, 0.08);
    border-left: 3px solid rgba(248, 113, 113, 0.7);
  }

  .preview-comment-meta {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }

  .preview-comment-meta strong {
    color: var(--text-strong);
    font-size: 13px;
  }

  .preview-comment-meta span {
    color: var(--text-muted);
    font-size: 11px;
  }

  .preview-comment > img {
    width: min(180px, 100%);
    max-height: 150px;
    margin: 4px 0 8px;
    object-fit: cover;
    border-radius: 10px;
  }

  .confirm-arguments {
    margin-top: 12px;
    border-radius: 15px;
    background: rgba(2, 6, 23, 0.28);
    border: 1px solid rgba(148, 163, 184, 0.12);
  }

  .confirm-arguments summary {
    padding: 12px 14px;
    color: var(--text-secondary);
    font-size: 13px;
    font-weight: 750;
    cursor: pointer;
  }

  .confirm-arguments pre {
    margin: 0;
    padding: 0 14px 14px;
    color: var(--text-secondary);
    font-family: "Cascadia Code", "JetBrains Mono", monospace;
    font-size: 12px;
    line-height: 1.6;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }

  .confirm-actions {
    display: flex;
    justify-content: flex-end;
    gap: 10px;
    margin-top: 24px;
  }

  .deny-btn,
  .approve-btn {
    min-height: 44px;
    padding: 0 18px;
    border-radius: 14px;
    font-size: 14px;
    font-weight: 800;
    transition: transform var(--duration-fast) var(--ease-out), filter var(--duration-fast) var(--ease-out), border-color var(--duration-fast) var(--ease-out);
  }

  .deny-btn {
    background: rgba(148, 163, 184, 0.1);
    color: var(--text-secondary);
    border: 1px solid rgba(148, 163, 184, 0.16);
  }

  .deny-btn:focus-visible,
  .approve-btn:focus-visible {
    outline: 3px solid rgba(147, 197, 253, 0.72);
    outline-offset: 3px;
  }

  .deny-btn:hover:not(:disabled) {
    color: var(--text-strong);
    border-color: rgba(148, 163, 184, 0.3);
  }

  .approve-btn {
    background: linear-gradient(135deg, #ef4444, #f97316);
    color: white;
    box-shadow: 0 12px 26px rgba(239, 68, 68, 0.24);
  }

  .approve-btn:hover:not(:disabled) {
    filter: brightness(1.08);
  }

  .deny-btn:active:not(:disabled),
  .approve-btn:active:not(:disabled) {
    transform: scale(0.98);
  }

  @keyframes backdrop-in {
    from { opacity: 0; }
  }

  @keyframes dialog-in {
    from {
      opacity: 0;
      transform: translateY(10px) scale(0.98);
    }
  }

  @keyframes preview-spin {
    to { transform: rotate(360deg); }
  }

  textarea {
    flex: 1;
    min-height: 52px;
    max-height: 160px;
    padding: 14px;
    border-radius: 18px;
    background: color-mix(in srgb, var(--bg-soft) 78%, transparent);
    color: var(--text);
    border: 1px solid rgba(148, 163, 184, 0.16);
    font-size: 14px;
    line-height: 1.6;
    resize: none;
    outline: none;
    transition: all var(--duration-fast) var(--ease-out);
  }

  textarea:focus {
    border-color: color-mix(in srgb, var(--accent-hover) 42%, transparent);
    box-shadow: 0 0 0 4px color-mix(in srgb, var(--accent) 14%, transparent);
  }

  .send-btn {
    align-self: stretch;
    padding: 0 26px;
    background: linear-gradient(135deg, var(--accent), var(--accent-strong));
    color: white;
    box-shadow: 0 14px 30px color-mix(in srgb, var(--accent-strong) 26%, transparent);
  }

  .send-btn:hover:not(:disabled) {
    filter: brightness(1.08);
    transform: translateY(-1px);
  }

  @media (max-width: 720px) {
    .chat-toolbar,
    .input-area {
      flex-direction: column;
      align-items: stretch;
    }

    .agent-hero {
      align-items: center;
    }

    .bubble {
      max-width: 92%;
    }

    .confirm-backdrop {
      padding: 16px;
    }

    .confirm-dialog {
      padding: 22px;
      border-radius: 22px;
    }

    .confirm-meta {
      grid-template-columns: 1fr;
    }

    .delete-preview-heading,
    .preview-comment-meta {
      align-items: flex-start;
      flex-direction: column;
      gap: 4px;
    }

    .delete-preview-heading span {
      text-align: left;
    }

    .preview-post:has(> img) {
      grid-template-columns: 72px minmax(0, 1fr);
    }

    .preview-post > img {
      width: 72px;
      height: 72px;
    }

    .confirm-actions {
      flex-direction: column-reverse;
    }

    .deny-btn,
    .approve-btn {
      width: 100%;
    }
  }

  @media (max-width: 520px) {
    .agent-hero {
      align-items: stretch;
      padding: 13px 14px;
    }

    .hero-title {
      align-items: flex-start;
      flex-direction: column;
      gap: 5px;
    }

    .reset-btn {
      min-height: 40px;
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .confirm-backdrop,
    .confirm-dialog,
    .preview-loading > span {
      animation: none;
    }
  }
</style>
