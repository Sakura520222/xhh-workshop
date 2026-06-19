// 通知轮询服务：未读红点 + 新消息提示
import { notifications, notificationUnreadCount, type NotificationUnreadCount } from "./api";
import {
  isPermissionGranted,
  requestPermission,
  sendNotification,
} from "@tauri-apps/plugin-notification";
import { getView } from "./stores.svelte";
import { toastInfo } from "./toast.svelte";
import {
  firstEmojiImageUrl,
  preloadEmoji,
  renderEmojiPlainText,
  renderEmojiText,
} from "./render.svelte";

const POLL_INTERVAL = 60_000; // 60s
const TOAST_FETCH_LIMIT = 5; // 每次拉取最新条数用于检测新消息
const SEEN_STORAGE_KEY = "xhh_seen_msg_ids";
const READ_BASELINE_KEY = "xhh_read_baseline";
const SEEN_MAX = 500; // 已读 id 上限，防止无限增长

// 未读状态
let _unread = $state<number>(0);
let _detail = $state<NotificationUnreadCount>({ comment: 0, award: 0 });
// 一键已读基线：标记已读时的服务端未读快照，之后仅当计数增长才重新亮起
let _readBaseline: number | null = null;
let _running = false;
let _timer: ReturnType<typeof setInterval> | null = null;
let _seenIds = new Set<string>();
let _inited = false;
let _nativePermissionRequested = false;
let _emojiReady: Promise<void> | null = null;

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

function loadReadBaseline() {
  try {
    const raw = localStorage.getItem(READ_BASELINE_KEY);
    _readBaseline = raw != null && raw !== "" ? Number(raw) || null : null;
  } catch { /* ignore */ }
}

function persistReadBaseline() {
  try {
    if (_readBaseline !== null) localStorage.setItem(READ_BASELINE_KEY, String(_readBaseline));
    else localStorage.removeItem(READ_BASELINE_KEY);
  } catch { /* ignore */ }
}

// 模块加载即恢复已读基线，刷新/重进后仍生效
loadReadBaseline();

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

function plainMessageText(value: unknown): string {
  const raw = String(value ?? "").trim();
  if (!raw) return "";
  const el = document.createElement("div");
  el.innerHTML = raw;
  return (el.textContent ?? "").replace(/\s+/g, " ").trim().slice(0, 180);
}

function ensureNotificationEmoji() {
  _emojiReady ??= preloadEmoji();
  return _emojiReady;
}

async function ensureNativePermission(): Promise<boolean> {
  try {
    if (await isPermissionGranted()) return true;
    if (_nativePermissionRequested) return false;
    _nativePermissionRequested = true;
    return (await requestPermission()) === "granted";
  } catch {
    return false;
  }
}

async function notifyNative(title: string, body: string, icon?: string | null) {
  try {
    if (!(await ensureNativePermission())) return;
    const options = { title, body: body || "点击查看通知", ...(icon ? { icon } : {}) };
    sendNotification(options);
  } catch { /* ignore */ }
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
    await ensureNotificationEmoji();
    const first = fresh[0];
    const name = first?.user_a?.username ?? "有人";
    if (fresh.length === 1) {
      const raw = first?.comment_a_text ?? first?.link_title ?? "";
      const title = `${name} ${msgLabel(first.message_type)}`;
      toastInfo(title, renderEmojiText(raw));
      await notifyNative(
        title,
        plainMessageText(renderEmojiPlainText(raw)),
        firstEmojiImageUrl(`${title} ${raw}`),
      );
    } else {
      toastInfo(`你有 ${fresh.length} 条新通知`, "点击查看");
      await notifyNative("黑盒工坊", `你有 ${fresh.length} 条新通知`);
    }
  } catch { /* 静默失败，不打断轮询 */ }
}

async function pollOnce() {
  // 未读计数
  try {
    const d = await notificationUnreadCount();
    _detail = d;
    applyServerUnread((d.comment ?? 0) + (d.award ?? 0));
  } catch { /* 未登录或网络异常，保持上次状态 */ }
  // 新消息提示
  await checkNewMessages();
}

export function startPolling() {
  if (!_inited) {
    loadSeen();
    _inited = true;
  }
  void ensureNotificationEmoji();
  if (_running) return;
  _running = true;
  pollOnce();
  _timer = setInterval(pollOnce, POLL_INTERVAL);
}

export function stopPolling() {
  if (_timer) { clearInterval(_timer); _timer = null; }
  _running = false;
}

// 未读计数应用：若处于一键已读基线之后，仅显示新增量
function applyServerUnread(total: number) {
  if (_readBaseline !== null) {
    _unread = total > _readBaseline ? total - _readBaseline : 0;
    if (total < _readBaseline) {
      _readBaseline = total;
      persistReadBaseline();
    }
  } else {
    _unread = total;
  }
}

// 立即刷新一次未读（进入通知页/手动刷新时调用）
export async function refreshUnread() {
  try {
    const d = await notificationUnreadCount();
    _detail = d;
    applyServerUnread((d.comment ?? 0) + (d.award ?? 0));
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

// 一键已读：以当前服务端未读为基线本地清零红点（服务端无已读接口，仅本地生效）
export async function markAllRead() {
  await markSeen();
  try {
    const d = await notificationUnreadCount();
    _detail = d;
    _readBaseline = (d.comment ?? 0) + (d.award ?? 0);
  } catch {
    _readBaseline = _unread;
  }
  _unread = 0;
  persistReadBaseline();
}
