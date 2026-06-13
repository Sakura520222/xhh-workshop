// Tauri IPC 调用封装
import { invoke } from "@tauri-apps/api/core";

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

export const authStatus = (): Promise<LoginResult> => invoke("auth_status");

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
}

export const postDetail = (params: PostDetailParams): Promise<any> =>
  invoke("post_detail", {
    linkId: params.link_id,
    page: params.page,
    index: params.index,
    limit: params.limit,
    isFirst: params.is_first,
    ownerOnly: params.owner_only,
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

export const favourite = (link_id: string, folder_id?: string): Promise<any> =>
  invoke("favourite", { linkId: link_id, folderId: folder_id });

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
    const { listen } = await import("@tauri-apps/api/event");
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

// AI (streaming via Tauri events)
export async function aiAnalyzeStream(
  prompt: string,
  images: string[] | undefined,
  onChunk: (text: string) => void,
  onDone: () => void,
  onError: (err: string) => void,
): Promise<void> {
  const { listen } = await import("@tauri-apps/api/event");

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
