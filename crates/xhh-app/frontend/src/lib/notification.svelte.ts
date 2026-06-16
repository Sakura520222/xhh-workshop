// 通知轮询服务：未读红点 + 新消息提示
import { notifications, notificationUnreadCount, type NotificationUnreadCount } from "./api";
import { getView } from "./stores.svelte";
import { toastInfo } from "./toast.svelte";

const POLL_INTERVAL = 60_000; // 60s
const TOAST_FETCH_LIMIT = 5; // 每次拉取最新条数用于检测新消息
const SEEN_STORAGE_KEY = "xhh_seen_msg_ids";
const SEEN_MAX = 500; // 已读 id 上限，防止无限增长

// 未读状态
let _unread = $state<number>(0);
let _detail = $state<NotificationUnreadCount>({ comment: 0, award: 0 });
let _running = false;
let _timer: ReturnType<typeof setInterval> | null = null;
let _seenIds = new Set<string>();
let _inited = false;

export function getUnread() {
  return _unread;
}

export function getUnreadDetail() {
  return _detail;
}

function loadSeen() {
  try {
    const raw = localStorage.getItem(SEEN_STORAGE_KEY);
    if (raw) {
      const arr = JSON.parse(raw) as string[];
      _seenIds = new Set(arr);
    }
  } catch { /* ignore */ }
}

function persistSeen() {
  try {
    const arr = [..._seenIds].slice(-SEEN_MAX);
    _seenIds = new Set(arr);
    localStorage.setItem(SEEN_STORAGE_KEY, JSON.stringify(arr));
  } catch { /* ignore */ }
}

function msgLabel(type: number): string {
  switch (type) {
    case 1: return "回复了你";
    case 2: return "评论了你";
    case 3: return "赞了你";
    case 4: return "关注了你";
    case 16: return "@了你";
    default: return "新消息";
  }
}

// 拉取最新消息并提示新增（按 message_id 去重）
async function checkNewMessages() {
  // 处于通知页时不弹提示，避免与列表内容重复
  if (getView() === "notifications") return;
  try {
    const v = await notifications(0, TOAST_FETCH_LIMIT);
    const items: any[] = v?.result?.messages ?? [];
    const fresh = items.filter((m) => {
      const id = String(m?.message_id ?? "");
      return id && !_seenIds.has(id);
    });
    if (fresh.length === 0) return;
    for (const m of fresh) _seenIds.add(String(m.message_id));
    persistSeen();
    const first = fresh[0];
    const name = first?.user_a?.username ?? "有人";
    if (fresh.length === 1) {
      const text = first?.comment_a_text ?? first?.link_title ?? "";
      toastInfo(`${name} ${msgLabel(first.message_type)}`, text);
    } else {
      toastInfo(`你有 ${fresh.length} 条新通知`, "点击查看");
    }
  } catch { /* 静默失败，不打断轮询 */ }
}

async function pollOnce() {
  // 未读计数
  try {
    const d = await notificationUnreadCount();
    _detail = d;
    _unread = (d.comment ?? 0) + (d.award ?? 0);
  } catch { /* 未登录或网络异常，保持上次状态 */ }
  // 新消息提示
  await checkNewMessages();
}

export function startPolling() {
  if (!_inited) {
    loadSeen();
    _inited = true;
  }
  if (_running) return;
  _running = true;
  pollOnce();
  _timer = setInterval(pollOnce, POLL_INTERVAL);
}

export function stopPolling() {
  if (_timer) { clearInterval(_timer); _timer = null; }
  _running = false;
}

// 立即刷新一次未读（进入通知页/手动刷新时调用）
export async function refreshUnread() {
  try {
    const d = await notificationUnreadCount();
    _detail = d;
    _unread = (d.comment ?? 0) + (d.award ?? 0);
  } catch { /* ignore */ }
}

// 标记当前最新消息为已见（进通知页后不再为它们弹提示）
export async function markSeen() {
  try {
    const v = await notifications(0, TOAST_FETCH_LIMIT);
    const items: any[] = v?.result?.messages ?? [];
    for (const m of items) {
      const id = String(m?.message_id ?? "");
      if (id) _seenIds.add(id);
    }
    persistSeen();
  } catch { /* ignore */ }
}
