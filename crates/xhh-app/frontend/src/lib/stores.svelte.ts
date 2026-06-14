// 全局状态（Svelte 5 runes 风格）
import { authStatus, type LoginResult, type WindowEffect } from "./api";

export type View = "home" | "profile" | "agent" | "settings" | "detail" | "editor" | "search" | "notifications" | "favourites" | "user";

// 登录态
let _authState = $state<LoginResult>({ ok: false, nickname: "", heybox_id: "", avatar: "", message: "" });
let _authChecking = $state(true); // 启动时正在检查登录态，避免闪登录页
// 当前视图
let _view = $state<View>("home");
// 加载中
let _loading = $state(false);

export function getAuth() {
  return _authState;
}

export function setAuth(v: LoginResult) {
  _authState = v;
}

export function isAuthChecking() {
  return _authChecking;
}

export async function refreshAuth() {
  _loading = true;
  _authChecking = true;
  try {
    _authState = await authStatus();
  } finally {
    _loading = false;
    _authChecking = false;
  }
}

export function getView() {
  return _view;
}

export function setView(v: View) {
  if (_view !== "detail") {
    _prevView = _view;
  }
  _view = v;
}

export function getPrevView() {
  return _prevView;
}

let _prevView = $state<View>("home");

// 当前查看的帖子 link_id（详情页用）
let _selectedLinkId = $state("");

export function getSelectedLinkId() {
  return _selectedLinkId;
}

export function setSelectedLinkId(id: string) {
  _selectedLinkId = id;
}

// 查看他人用户主页的 userid
let _selectedUserId = $state("");

export function getSelectedUserId() {
  return _selectedUserId;
}

export function setSelectedUserId(id: string) {
  _selectedUserId = id;
}

// 主页滚动位置记忆（离开前保存，回来时恢复）
let _homeScrollTop = 0;

export function getHomeScrollTop() {
  return _homeScrollTop;
}

export function setHomeScrollTop(top: number) {
  _homeScrollTop = top;
}

// feeds 持久缓存（localStorage，关闭后重开仍保留）
const FEEDS_STORAGE_KEY = "xhh_feeds_cache";

function loadPersistedFeeds(): any[] {
  try {
    const raw = localStorage.getItem(FEEDS_STORAGE_KEY);
    if (raw) return JSON.parse(raw);
  } catch { /* ignore */ }
  return [];
}

function persistFeeds(feeds: any[]) {
  try {
    localStorage.setItem(FEEDS_STORAGE_KEY, JSON.stringify(feeds));
  } catch { /* ignore */ }
}

const initialFeeds = loadPersistedFeeds();
let _feedsCache = $state<any[]>(initialFeeds);
let _feedsLoaded = $state(initialFeeds.length > 0);

export function getFeedsCache() {
  return _feedsCache;
}

export function setFeedsCache(feeds: any[]) {
  _feedsCache = feeds;
  _feedsLoaded = true;
  persistFeeds(feeds);
}

export function isFeedsCached() {
  return _feedsLoaded;
}

export function getLoading() {
  return _loading;
}

export function setLoading(v: boolean) {
  _loading = v;
}

// 外观主题
export type Theme = "midnight" | "carbon" | "emerald" | "violet" | "rose" | "amber";

export const THEMES: { key: Theme; label: string; color: string }[] = [
  { key: "midnight", label: "午夜", color: "#3b82f6" },
  { key: "carbon", label: "碳灰", color: "#38bdf8" },
  { key: "emerald", label: "翡翠", color: "#10b981" },
  { key: "violet", label: "紫罗兰", color: "#8b5cf6" },
  { key: "rose", label: "玫瑰", color: "#f43f5e" },
  { key: "amber", label: "琥珀", color: "#f59e0b" },
];

const THEME_STORAGE_KEY = "xhh_theme";
const DEFAULT_THEME: Theme = "midnight";

function loadPersistedTheme(): Theme {
  try {
    const raw = localStorage.getItem(THEME_STORAGE_KEY) as Theme | null;
    if (raw && THEMES.some((t) => t.key === raw)) return raw;
  } catch { /* ignore */ }
  return DEFAULT_THEME;
}

const _initialTheme = loadPersistedTheme();
let _theme = $state<Theme>(_initialTheme);

function applyTheme(t: Theme) {
  document.documentElement.dataset.theme = t;
}

// 模块加载时立即应用，避免首屏主题闪烁
if (typeof document !== "undefined") {
  applyTheme(_initialTheme);
}

export function getTheme() {
  return _theme;
}

export function setTheme(t: Theme) {
  _theme = t;
  applyTheme(t);
  try {
    localStorage.setItem(THEME_STORAGE_KEY, t);
  } catch { /* ignore */ }
}

// 窗口效果（前端 attr 控制 body 背景透明度，实际效果由后端应用）
const WINDOW_EFFECT_STORAGE_KEY = "xhh_window_effect";
const VALID_WINDOW_EFFECTS: WindowEffect[] = ["none", "mica", "acrylic"];
const DEFAULT_WINDOW_EFFECT: WindowEffect = "mica";

function loadPersistedWindowEffect(): WindowEffect {
  try {
    const raw = localStorage.getItem(WINDOW_EFFECT_STORAGE_KEY) as WindowEffect | null;
    if (raw && VALID_WINDOW_EFFECTS.includes(raw)) return raw;
  } catch { /* ignore */ }
  return DEFAULT_WINDOW_EFFECT;
}

function applyWindowEffectAttr(effect: WindowEffect) {
  document.documentElement.dataset.windowEffect = effect;
}

// 模块加载时立即应用，避免首屏背景闪烁
if (typeof document !== "undefined") {
  applyWindowEffectAttr(loadPersistedWindowEffect());
}

export function setWindowEffectAttr(effect: WindowEffect) {
  applyWindowEffectAttr(effect);
  try {
    localStorage.setItem(WINDOW_EFFECT_STORAGE_KEY, effect);
  } catch { /* ignore */ }
}
