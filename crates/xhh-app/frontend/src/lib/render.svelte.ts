import { emojiList } from "./api";

// ─── Emoji Map ──────────────────────────────────────────

const EMOJI_CACHE_KEY = "xhh_emoji_cache";
const EMOJI_VERSION_KEY = "xhh_emoji_version";

let _emojiMap = $state<Map<string, string>>(new Map());
let _version = $state(0);

function loadCachedEmoji(): Map<string, string> | null {
  try {
    const ver = localStorage.getItem(EMOJI_VERSION_KEY);
    const raw = localStorage.getItem(EMOJI_CACHE_KEY);
    if (ver && raw) return new Map(JSON.parse(raw));
  } catch { /* ignore */ }
  return null;
}

// 启动时加载本地缓存
const cached = loadCachedEmoji();
if (cached) _emojiMap = cached;

async function ensureEmoji(): Promise<Map<string, string>> {
  try {
    const v = await emojiList();
    const map = new Map<string, string>();
    const groups = v?.result?.emoji_groups ?? [];
    for (const g of groups) {
      for (const e of g.emojis ?? []) {
        if ((e.type === 1 || e.type === 3) && e.code && e.img) {
          map.set(e.code, e.img);
        }
      }
    }
    localStorage.setItem(EMOJI_VERSION_KEY, v?.result?.emoji_version ?? "");
    localStorage.setItem(EMOJI_CACHE_KEY, JSON.stringify([...map.entries()]));
    _emojiMap = map;
    _version++;
  } catch {
    /* 网络失败时保留已有缓存 */
  }
  return _emojiMap;
}

export async function preloadEmoji() {
  await ensureEmoji();
}

export function getEmojiVersion() {
  return _version;
}

// ─── Markdown ──────────────────────────────────────────

function escapeHtml(s: string): string {
  return s
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;");
}

type TableAlignment = "left" | "center" | "right";

function isEscapedPipe(source: string, pipeIndex: number): boolean {
  let slashCount = 0;
  for (let i = pipeIndex - 1; i >= 0 && source[i] === "\\"; i--) {
    slashCount++;
  }
  return slashCount % 2 === 1;
}

function splitTableRow(line: string): string[] | null {
  let source = line.trim();
  let hasBoundary = false;

  if (source.startsWith("|")) {
    source = source.slice(1);
    hasBoundary = true;
  }
  if (source.endsWith("|") && !isEscapedPipe(source, source.length - 1)) {
    source = source.slice(0, -1);
    hasBoundary = true;
  }

  const cells: string[] = [];
  let cell = "";
  for (let i = 0; i < source.length; i++) {
    const char = source[i];
    if (char === "|" && isEscapedPipe(source, i)) {
      cell = cell.slice(0, -1);
      cell += "|";
    } else if (char === "|") {
      cells.push(cell.trim());
      cell = "";
      hasBoundary = true;
    } else {
      cell += char;
    }
  }
  cells.push(cell.trim());

  return hasBoundary ? cells : null;
}

function parseTableAlignments(line: string): TableAlignment[] | null {
  const cells = splitTableRow(line);
  if (!cells || cells.length === 0) return null;

  const alignments: TableAlignment[] = [];
  for (const cell of cells) {
    if (!/^:?-+:?$/.test(cell)) return null;
    if (cell.startsWith(":") && cell.endsWith(":")) {
      alignments.push("center");
    } else if (cell.endsWith(":")) {
      alignments.push("right");
    } else {
      alignments.push("left");
    }
  }
  return alignments;
}

function renderMarkdown(text: string): string {
  text = text.replace(/```([\s\S]*?)```/g, (_, code: string) => {
    return `<pre class="md-code-block"><code>${escapeHtml(code.trim())}</code></pre>`;
  });
  text = text.replace(/`([^`]+)`/g, (_, code: string) => {
    return `<code class="md-code">${escapeHtml(code)}</code>`;
  });
  text = text.replace(/\*\*\*(.+?)\*\*\*/g, "<strong><em>$1</em></strong>");
  text = text.replace(/\*\*(.+?)\*\*/g, "<strong>$1</strong>");
  text = text.replace(/(?<!\*)\*(?!\*)(.+?)(?<!\*)\*(?!\*)/g, "<em>$1</em>");
  text = text.replace(/~~(.+?)~~/g, "<del>$1</del>");
  text = text.replace(/\[([^\]]+)\]\(([^)]+)\)/g, '<a href="$2" target="_blank" rel="noopener" class="md-link">$1</a>');
  text = text.replace(/\n\n/g, "</p><p>");
  text = text.replace(/\n/g, "<br>");
  return text;
}

/// AI 输出 Markdown 渲染（完整支持标题/列表/引用/代码块等）
export function renderAiMarkdown(text: string): string {
  if (!text) return "";

  // 提取代码块，用占位符保护，避免内部被其他规则破坏
  const codeBlocks: string[] = [];
  text = text.replace(/```(\w*)\n?([\s\S]*?)```/g, (_, lang: string, code: string) => {
    const idx = codeBlocks.length;
    const langAttr = lang ? ` class="language-${escapeHtml(lang)}"` : "";
    codeBlocks.push(`<pre class="ai-md-pre"><code${langAttr}>${escapeHtml(code.trimEnd())}</code></pre>`);
    return `\x00CB${idx}\x00`;
  });

  const inlineCodes: string[] = [];
  text = text.replace(/`([^`]+)`/g, (_, code: string) => {
    const idx = inlineCodes.length;
    inlineCodes.push(`<code class="ai-md-code">${escapeHtml(code)}</code>`);
    return `\x00IC${idx}\x00`;
  });

  const lines = text.split("\n");
  const out: string[] = [];
  let inList = false;
  let inOl = false;
  let blockquoteDepth = 0;

  for (let lineIndex = 0; lineIndex < lines.length; lineIndex++) {
    const line = lines[lineIndex];

    // 代码块占位符行，直接输出
    if (/^\x00CB\d+\x00$/.test(line.trim())) {
      if (inList) { out.push("</ul>"); inList = false; }
      if (inOl) { out.push("</ol>"); inOl = false; }
      out.push(line.trim());
      continue;
    }

    // GFM 表格
    const headerCells = line.includes("|") ? splitTableRow(line) : null;
    const alignments = headerCells && lineIndex + 1 < lines.length && lines[lineIndex + 1].includes("|")
      ? parseTableAlignments(lines[lineIndex + 1])
      : null;
    if (headerCells && alignments && headerCells.length === alignments.length) {
      if (inList) { out.push("</ul>"); inList = false; }
      if (inOl) { out.push("</ol>"); inOl = false; }

      const header = headerCells
        .map((cell, idx) => `<th class="ai-md-align-${alignments[idx]}">${inlineFormat(cell)}</th>`)
        .join("");
      const rows: string[] = [];
      let rowIndex = lineIndex + 2;
      while (rowIndex < lines.length) {
        const rowCells = splitTableRow(lines[rowIndex]);
        if (!rowCells) break;
        const cells = Array.from(
          { length: headerCells.length },
          (_, idx) => `<td class="ai-md-align-${alignments[idx]}">${inlineFormat(rowCells[idx] ?? "")}</td>`,
        ).join("");
        rows.push(`<tr>${cells}</tr>`);
        rowIndex++;
      }

      out.push(
        `<div class="ai-md-table-wrap"><table class="ai-md-table"><thead><tr>${header}</tr></thead>` +
        `<tbody>${rows.join("")}</tbody></table></div>`,
      );
      lineIndex = rowIndex - 1;
      continue;
    }

    // 标题
    const headingMatch = line.match(/^(#{1,6})\s+(.+)/);
    if (headingMatch) {
      if (inList) { out.push("</ul>"); inList = false; }
      if (inOl) { out.push("</ol>"); inOl = false; }
      const level = headingMatch[1].length;
      out.push(`<h${level} class="ai-md-h${level}">${inlineFormat(headingMatch[2])}</h${level}>`);
      continue;
    }

    // 引用块
    const bqMatch = line.match(/^>\s?(.*)/);
    if (bqMatch) {
      if (inList) { out.push("</ul>"); inList = false; }
      if (inOl) { out.push("</ol>"); inOl = false; }
      out.push(`<div class="ai-md-blockquote">${inlineFormat(bqMatch[1])}</div>`);
      continue;
    }

    // 分隔线
    if (/^-{3,}$/.test(line.trim()) || /^\*{3,}$/.test(line.trim())) {
      if (inList) { out.push("</ul>"); inList = false; }
      if (inOl) { out.push("</ol>"); inOl = false; }
      out.push('<hr class="ai-md-hr" />');
      continue;
    }

    // 无序列表
    const ulMatch = line.match(/^(\s*)[-*+]\s+(.*)/);
    if (ulMatch) {
      if (inOl) { out.push("</ol>"); inOl = false; }
      if (!inList) { out.push('<ul class="ai-md-ul">'); inList = true; }
      out.push(`<li>${inlineFormat(ulMatch[2])}</li>`);
      continue;
    }

    // 有序列表
    const olMatch = line.match(/^(\s*)\d+[.)]\s+(.*)/);
    if (olMatch) {
      if (inList) { out.push("</ul>"); inList = false; }
      if (!inOl) { out.push('<ol class="ai-md-ol">'); inOl = true; }
      out.push(`<li>${inlineFormat(olMatch[2])}</li>`);
      continue;
    }

    // 普通段落
    if (inList) { out.push("</ul>"); inList = false; }
    if (inOl) { out.push("</ol>"); inOl = false; }

    const trimmed = line.trim();
    if (trimmed === "") {
      out.push("");
    } else {
      out.push(`<p>${inlineFormat(trimmed)}</p>`);
    }
  }

  if (inList) out.push("</ul>");
  if (inOl) out.push("</ol>");

  let html = out.join("\n");

  // 还原代码块
  html = html.replace(/\x00CB(\d+)\x00/g, (_, idx) => codeBlocks[parseInt(idx)]);
  html = html.replace(/\x00IC(\d+)\x00/g, (_, idx) => inlineCodes[parseInt(idx)]);

  return html;
}

function inlineFormat(text: string): string {
  const links: string[] = [];
  text = text.replace(/\[([^\]]+)\]\(([^)]+)\)/g, (_, label: string, href: string) => {
    const idx = links.length;
    const safeHref = sanitizeMarkdownHref(href);
    const labelHtml = formatInlineText(label);
    links.push(
      safeHref
        ? `<a href="${escapeHtml(safeHref)}" target="_blank" rel="noopener" class="ai-md-link">${labelHtml}</a>`
        : labelHtml,
    );
    return `\x00LK${idx}\x00`;
  });

  text = formatInlineText(text);
  return text.replace(/\x00LK(\d+)\x00/g, (_, idx) => links[parseInt(idx)]);
}

function formatInlineText(text: string): string {
  const emojis: string[] = [];
  text = text.replace(/\[([a-zA-Z]+_[^\]]+)\]/g, (full, code: string) => {
    const html = emojiHtml(code);
    if (!html) return full;
    const idx = emojis.length;
    emojis.push(html);
    return `\x00EM${idx}\x00`;
  });

  text = escapeHtml(text);
  text = text.replace(/\*\*\*(.+?)\*\*\*/g, "<strong><em>$1</em></strong>");
  text = text.replace(/\*\*(.+?)\*\*/g, "<strong>$1</strong>");
  text = text.replace(/(?<!\*)\*(?!\*)(.+?)(?<!\*)\*(?!\*)/g, "<em>$1</em>");
  text = text.replace(/~~(.+?)~~/g, "<del>$1</del>");
  return text.replace(/\x00EM(\d+)\x00/g, (_, idx) => emojis[parseInt(idx)]);
}

function sanitizeMarkdownHref(href: string): string | null {
  const trimmed = href.trim();
  const normalized = trimmed.replace(/[\u0000-\u0020\u007f]+/g, "").toLowerCase();
  if (
    normalized.startsWith("http://") ||
    normalized.startsWith("https://") ||
    normalized.startsWith("mailto:") ||
    normalized.startsWith("/") ||
    normalized.startsWith("#") ||
    normalized.startsWith("?")
  ) {
    return trimmed;
  }
  return null;
}

// ─── Emoji Parsing ──────────────────────────────────────

function emojiHtml(code: string): string | null {
  const key = code.replace(/^[a-zA-Z]+_/, "");
  const img = _emojiMap.get(key);
  if (!img) return null;
  return `<img src="${escapeHtml(img)}" alt="${escapeHtml(code)}" class="emoji" />`;
}

function renderEmoji(text: string): string {
  return text.replace(/\[([a-zA-Z]+_[^\]]+)\]/g, (full, code: string) => {
    return emojiHtml(code) ?? full;
  });
}

// ─── Public API ─────────────────────────────────────────

export function renderTextSync(text: string): string {
  if (!text) return "";
  const withEmoji = renderEmoji(text);
  return renderMarkdown(withEmoji);
}
