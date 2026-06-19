// Tauri IPC 调用封装
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

export interface QrCodeResp {
  qr_url: string;
  raw_query: string;
  expire: number;
}

export interface LoginResult {
  ok: boolean;
  nickname: string;
  heybox_id: string;
  avatar: string;
  message: string;
}

export interface AgentResult {
  final_output: string;
  tool_calls: string[];
  loops_used: number;
}

export interface CommentReq {
  link_id: string;
  text: string;
  reply_id?: string;
  root_id?: string;
}

export interface SearchReq {
  q: string;
  search_type?: string;
  topic_id?: number;
  limit?: number;
}

// Auth
export const authGetQrCode = (): Promise<QrCodeResp> =>
  invoke("auth_get_qr_code");

export const authLogin = (raw_query: string, device_id: string): Promise<LoginResult> =>
  invoke("auth_login", { rawQuery: raw_query, deviceId: device_id });

// 取消正在进行的扫码登录轮询（刷新二维码 / 放弃当前扫码）
export const authCancelLogin = (): Promise<void> => invoke("auth_cancel_login");

export const authStatus = (): Promise<LoginResult> => invoke("auth_status");

// 监听二维码已扫码（等待手机端确认）
export function onAuthScanned(cb: () => void): () => void {
  let unlisten: (() => void) | null = null;
  let done = false;
  listen("auth-scanned", () => {
    if (!done) cb();
  }).then((fn) => {
    if (done) fn();
    else unlisten = fn;
  });
  return () => {
    done = true;
    unlisten?.();
  };
}

export const authLogout = (): Promise<void> => invoke("auth_logout");

// Feeds
export const feedsList = (page = 1, limit = 20): Promise<any> =>
  invoke("feeds_list", { page, limit });

export interface PostDetailParams {
  link_id: string;
  page?: number;
  index?: number;
  limit?: number;
  is_first?: number;
  owner_only?: number;
  force?: boolean;
}

export const postDetail = (params: PostDetailParams): Promise<any> =>
  invoke("post_detail", {
    linkId: params.link_id,
    page: params.page,
    index: params.index,
    limit: params.limit,
    isFirst: params.is_first,
    ownerOnly: params.owner_only,
    force: params.force,
  });

export const communityFeeds = (topic_id: number, limit = 20): Promise<any> =>
  invoke("community_feeds", { topicId: topic_id, limit });

// Post
export const postCreate = (
  title: string,
  content: string,
  hashtags: string[],
  community_topic_id?: string,
  images?: { url: string; width: number; height: number }[]
): Promise<any> =>
  invoke("post_create", { title, content, hashtags, communityTopicId: community_topic_id, images });

export const postDelete = (link_id: string): Promise<any> =>
  invoke("post_delete", { linkId: link_id });

// Comment
export const commentCreate = (req: CommentReq): Promise<any> =>
  invoke("comment_create", { req });

export const commentList = (link_id: string, page = 1, limit = 20): Promise<any> =>
  invoke("comment_list", { linkId: link_id, page, limit });

export const subComments = (root_comment_id: string, lastval?: string): Promise<any> =>
  invoke("sub_comments", { rootCommentId: root_comment_id, lastval: lastval ?? "" });

// Interaction
export const likePost = (link_id: string, award_type: number): Promise<any> =>
  invoke("like_post", { linkId: link_id, awardType: award_type });

export const likeComment = (comment_id: string): Promise<any> =>
  invoke("like_comment", { commentId: comment_id });

export const favourite = (link_id: string, folder_id: string | undefined, favour_type: number): Promise<any> =>
  invoke("favourite", { linkId: link_id, folderId: folder_id, favourType: favour_type });

export const createFavouriteFolder = (name: string): Promise<any> =>
  invoke("create_favourite_folder", { name });

// Search
export const search = (req: SearchReq): Promise<any> => invoke("search", { req });

export const searchCommunity = (keyword: string): Promise<any> =>
  invoke("search_community", { keyword });

// User
export const userProfile = (userid?: string): Promise<any> =>
  invoke("user_profile", { userid });

// Agent
export const agentChat = (message: string): Promise<AgentResult> =>
  invoke("agent_chat", { message });

export const agentReset = (): Promise<void> => invoke("agent_reset");

export const agentGetConfig = (): Promise<any> => invoke("agent_get_config");

export const agentSaveConfig = (config: any): Promise<void> =>
  invoke("agent_save_config", { config });

export const agentAutoPost = (intent: string, hashtags: string[] = []): Promise<AgentResult> =>
  invoke("agent_auto_post", { intent, hashtags });

// Agent history
export interface AgentUiMsg {
  role: string;
  text: string;
  tools?: string[];
  loops?: number;
}

export const agentHistoryGet = (): Promise<AgentUiMsg[]> =>
  invoke("agent_history_get");

export const agentHistorySave = (messages: AgentUiMsg[]): Promise<void> =>
  invoke("agent_history_save", { messages });

export const agentHistoryClear = (): Promise<void> =>
  invoke("agent_history_clear");

// Agent sessions（多会话切换）
export interface SessionMeta {
  id: string;
  title: string;
  created_at: number;
  updated_at: number;
  message_count: number;
}

export const agentSessionList = (): Promise<SessionMeta[]> =>
  invoke("agent_session_list");

export const agentSessionActive = (): Promise<string> =>
  invoke("agent_session_active");

export const agentSessionCreate = (): Promise<string> =>
  invoke("agent_session_create");

export const agentSessionSwitch = (id: string): Promise<AgentUiMsg[]> =>
  invoke("agent_session_switch", { id });

export const agentSessionRename = (id: string, title: string): Promise<void> =>
  invoke("agent_session_rename", { id, title });

export const agentSessionDelete = (id: string): Promise<string> =>
  invoke("agent_session_delete", { id });

// Agent templates（任务模板）
export interface AgentTemplate {
  id: string;
  title: string;
  content: string;
  is_builtin: boolean;
  created_at: number;
  updated_at: number;
}

export const agentTemplateList = (): Promise<AgentTemplate[]> =>
  invoke("agent_template_list");

export const agentTemplateSave = (
  title: string,
  content: string,
  id?: string,
): Promise<AgentTemplate> =>
  invoke("agent_template_save", { id: id ?? null, title, content });

export const agentTemplateDelete = (id: string): Promise<void> =>
  invoke("agent_template_delete", { id });

// Agent (streaming)
export interface AgentStreamDone {
  tool_calls: string[];
  loops_used: number;
}

export interface AgentToolConfirmationRequest {
  tool_name: string;
  risk_level: "Low" | "Medium" | "High";
  summary: string;
  arguments_json: string;
}

export interface AgentToolConfirmationDecision {
  tool_name: string;
  arguments_json: string;
  approved: boolean;
}

export async function agentChatStream(
  message: string,
  onChunk: (text: string) => void,
  onTool: (name: string) => void,
  onDone: (info: AgentStreamDone) => void,
  onError: (err: string) => void,
  onConfirmation?: (req: AgentToolConfirmationRequest) => void,
  confirmations: AgentToolConfirmationDecision[] = [],
): Promise<void> {
  const unlisteners: Array<() => void> = [];
  let cleaned = false;
  function cleanup() {
    if (cleaned) return;
    cleaned = true;
    for (const fn of unlisteners) fn();
  }

  const finishError = (err: string) => {
    if (cleaned) return;
    cleanup();
    onError(err);
  };

  try {
    const registrations = await Promise.allSettled([
      listen<string>("agent-chunk", (e) => { if (!cleaned) onChunk(e.payload); }),
      listen<string>("agent-tool", (e) => { if (!cleaned) onTool(e.payload); }),
      listen<AgentToolConfirmationRequest>("agent-tool-confirmation", (e) => { if (!cleaned) onConfirmation?.(e.payload); }),
      listen<AgentStreamDone>("agent-done", (e) => { if (!cleaned) { cleanup(); onDone(e.payload); } }),
      listen<string>("agent-error", (e) => { finishError(e.payload); }),
    ]);
    for (const registration of registrations) {
      if (registration.status === "fulfilled") unlisteners.push(registration.value);
    }
    const failedRegistration = registrations.find(
      (registration): registration is PromiseRejectedResult => registration.status === "rejected",
    );
    if (failedRegistration) {
      throw new Error(`注册 Agent 事件监听失败: ${String(failedRegistration.reason)}`);
    }
    await invoke("agent_chat_stream", { message, confirmations });
  } catch (e) {
    finishError(String(e));
  }
}

// AI cache (persist by link_id)
export interface AiCacheItem {
  link_id: string;
  kind: string;
  content: string;
  updated_at: number;
}

export const aiCacheGet = (linkId: string): Promise<AiCacheItem[]> =>
  invoke("ai_cache_get", { linkId });

export const aiCacheSave = (linkId: string, kind: string, content: string): Promise<AiCacheItem> =>
  invoke("ai_cache_save", { linkId, kind, content });

// Window
export type WindowEffect = "none" | "mica" | "acrylic";

export const windowEffectGet = (): Promise<WindowEffect> =>
  invoke<WindowEffect>("window_effect_get");

export const windowEffectSet = (effect: WindowEffect): Promise<void> =>
  invoke("window_effect_set", { effect });

// AI (streaming via Tauri events)
export async function aiAnalyzeStream(
  prompt: string,
  images: string[] | undefined,
  onChunk: (text: string) => void,
  onDone: () => void,
  onError: (err: string) => void,
): Promise<void> {
  let cleaned = false;
  const unlisteners = await Promise.all([
    listen<string>("ai-chunk", (e) => { if (!cleaned) onChunk(e.payload); }),
    listen("ai-done", () => { cleanup(); onDone(); }),
    listen<string>("ai-error", (e) => { cleanup(); onError(e.payload); }),
  ]);

  function cleanup() {
    if (cleaned) return;
    cleaned = true;
    for (const fn of unlisteners) fn();
  }

  try {
    await invoke("ai_analyze_stream", { prompt, images });
  } catch (e) {
    cleanup();
    onError(String(e));
  }
}

// Image
export const saveImage = (url: string, title?: string): Promise<string> =>
  invoke("save_image", { url, title });

// Upload
export const uploadImage = (): Promise<{ url: string; width: number; height: number }> =>
  invoke("upload_image");

// Topic
export const searchTopic = (keyword: string): Promise<any> =>
  invoke("search_topic", { keyword });

// Emoji
export const emojiList = (): Promise<any> => invoke("emoji_list");

// Notifications
export const notifications = (offset?: number, limit?: number): Promise<any> =>
  invoke("notifications", { offset: offset ?? 0, limit: limit ?? 20 });

export interface NotificationUnreadCount {
  comment: number;
  award: number;
}

export const notificationUnreadCount = (): Promise<NotificationUnreadCount> =>
  invoke("notification_unread_count");

// Favourites
export const favourFolders = (): Promise<any> =>
  invoke("favour_folders");

export const favourFolder = (folderId?: string, offset?: number, limit?: number): Promise<any> =>
  invoke("favour_folder", { folderId, offset: offset ?? 0, limit: limit ?? 30 });

// Follow
export const followUser = (userid: string): Promise<any> =>
  invoke("follow_user", { userid });

export const unfollowUser = (userid: string): Promise<any> =>
  invoke("unfollow_user", { userid });

export const followingList = (userid: string, offset?: number, limit?: number): Promise<any> =>
  invoke("following_list", { userid, offset: offset ?? 0, limit: limit ?? 50 });

export const followerList = (userid: string, offset?: number, limit?: number): Promise<any> =>
  invoke("follower_list", { userid, offset: offset ?? 0, limit: limit ?? 50 });

// User posts
export const userEvents = (userid?: string, lastval?: string): Promise<any> =>
  invoke("user_events", { userid, lastval: lastval ?? "" });

// Content cache (post text + image bytes)
export interface CacheConfig {
  enabled: boolean;
  max_bytes: number;
}
export interface CacheAreaStats {
  count: number;
  bytes: number;
}
export interface CacheStats {
  enabled: boolean;
  max_bytes: number;
  used_bytes: number;
  posts: CacheAreaStats;
  images: CacheAreaStats;
}

export const cacheGetConfig = (): Promise<CacheConfig> => invoke("cache_get_config");
export const cacheSaveConfig = (enabled: boolean, maxBytes: number): Promise<CacheConfig> =>
  invoke("cache_save_config", { enabled, maxBytes });
export const cacheStats = (): Promise<CacheStats> => invoke("cache_stats");
export const cacheClear = (): Promise<void> => invoke("cache_clear");

// Post — 编辑 / 草稿 / 视频帖
export const postEdit = (
  linkId: string,
  title: string,
  content: string,
  hashtags: string[],
  communityTopicId?: string,
  images?: { url: string; width: number; height: number }[],
): Promise<any> =>
  invoke("post_edit", { linkId, title, content, hashtags, communityTopicId, images });

export const postDraft = (title: string, content: string, communityTopicId?: string): Promise<any> =>
  invoke("post_draft", { title, content, communityTopicId });

export const postCreateVideo = (
  title: string,
  videoUrl: string,
  videoThumb: string,
  content?: string,
  communityTopicId?: string,
): Promise<any> =>
  invoke("post_create_video", { title, videoUrl, videoThumb, content, communityTopicId });

// Search — 发现 / 欢迎页 / 推荐话题
export const searchFound = (): Promise<any> => invoke("search_found");
export const searchWelcomePage = (): Promise<any> => invoke("search_welcome_page");
export const topicIndex = (): Promise<any> => invoke("topic_index");

// User — 用户信息 / 用户帖子列表
export const userInfo = (): Promise<any> => invoke("user_info");
export const userLinkList = (
  userid?: string,
  offset?: number,
  limit?: number,
): Promise<any> =>
  invoke("user_link_list", { userid, offset: offset ?? 0, limit: limit ?? 20 });

// Image — 原图 URL
export const originalImage = (url: string): Promise<any> => invoke("original_image", { url });

// Upload — 视频
export const uploadVideo = (): Promise<{ url: string }> => invoke("upload_video");

// Community — 社区菜单 / 社区头条
export const topicMenu = (topicId: number): Promise<any> => invoke("topic_menu", { topicId });
export const communityFeedsNews = (
  topicId: number,
  cateId?: number,
  limit?: number,
): Promise<any> => invoke("community_feeds_news", { topicId, cateId, limit });
