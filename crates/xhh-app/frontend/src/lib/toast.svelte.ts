// 轻量 toast 队列（Svelte 5 runes）
export type ToastKind = "info" | "success" | "error";

export interface ToastItem {
  id: number;
  kind: ToastKind;
  title: string;
  desc?: string;
}

let _toasts = $state<ToastItem[]>([]);
let _seq = 0;

export function getToasts() {
  return _toasts;
}

function push(kind: ToastKind, title: string, desc?: string, ttl = 5000) {
  const id = ++_seq;
  _toasts = [..._toasts, { id, kind, title, desc }];
  if (ttl > 0) {
    setTimeout(() => dismiss(id), ttl);
  }
  return id;
}

export function toastInfo(title: string, desc?: string) {
  return push("info", title, desc);
}

export function toastSuccess(title: string, desc?: string) {
  return push("success", title, desc);
}

export function toastError(title: string, desc?: string) {
  return push("error", title, desc, 8000);
}

export function dismiss(id: number) {
  _toasts = _toasts.filter((t) => t.id !== id);
}
