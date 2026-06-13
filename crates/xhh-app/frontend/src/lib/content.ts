export type ContentSegment =
  | { kind: "text"; value: string }
  | { kind: "html"; value: string }
  | { kind: "images"; urls: string[]; top: boolean };

const IMG_RE = /<img\s[^>]*>/gi;
const IMG_SRC_RE = /(?:data-original|src)="([^"]+)"/i;

function imgSrc(tag: string): string | undefined {
  const m = tag.match(IMG_SRC_RE);
  return m?.[1];
}

function segmentsFromHtml(html: string): { segs: ContentSegment[]; imgUrls: string[] } {
  const segs: ContentSegment[] = [];
  const imgUrls: string[] = [];
  let pos = 0;
  let pStripped = false;

  for (const m of html.matchAll(IMG_RE)) {
    const imgStart = m.index!;
    const imgEnd = imgStart + m[0].length;
    let before = html.slice(pos, imgStart);

    // <img> inside <p>...</p> — remove the wrapping p tags
    const pOpen = before.match(/<p[^>]*>\s*$/i);
    if (pOpen) {
      before = before.slice(0, before.length - pOpen[0].length);
      pStripped = true;
    }

    // If we previously stripped a <p> and the tail starts with </p>, consume it
    let afterStart = imgEnd;
    if (pStripped) {
      const afterSlice = html.slice(imgEnd);
      const closeMatch = afterSlice.match(/^\s*<\/p>/i);
      if (closeMatch) {
        afterStart = imgEnd + closeMatch[0].length;
        pStripped = false;
      }
    }

    const trimmed = before.replace(/<p[^>]*>\s*<\/p>/gi, "").trim();
    if (trimmed) segs.push({ kind: "html", value: trimmed });

    const url = imgSrc(m[0]);
    if (url) {
      segs.push({ kind: "images", urls: [url], top: segs.length === 0 });
      imgUrls.push(url);
    }

    pos = afterStart;
    pStripped = false;
  }
  const tail = html.slice(pos).replace(/<p[^>]*>\s*<\/p>/gi, "").trim();
  if (tail) segs.push({ kind: "html", value: tail });
  return { segs, imgUrls };
}

export function parsePostContent(raw: any): ContentSegment[] {
  if (typeof raw !== "string" || !raw) return [];
  type Block = { type: "text"; value: string } | { type: "img"; value: string } | { type: "html"; value: string };
  let blocks: Block[] = [];
  try {
    const arr = JSON.parse(raw);
    if (Array.isArray(arr)) {
      blocks = arr
        .map((b: any) => {
          if (b?.type === "text" && b.text) return { type: "text" as const, value: b.text };
          if (b?.type === "img") {
            const url = b.url ?? b.text;
            if (url) return { type: "img" as const, value: url };
          }
          if (b?.type === "html" && b.text) return { type: "html" as const, value: b.text };
          return null;
        })
        .filter(Boolean) as Block[];
    }
  } catch { /* non-JSON fallback */ }
  if (blocks.length === 0 && raw) blocks = [{ type: "text", value: raw }];

  const emittedImgUrls = new Set<string>();
  const segments: ContentSegment[] = [];
  let i = 0;
  while (i < blocks.length) {
    const b = blocks[i];
    if (b.type === "img") {
      const urls: string[] = [];
      while (i < blocks.length && blocks[i].type === "img") { urls.push(blocks[i].value); i++; }
      const deduped = urls.filter((u) => !emittedImgUrls.has(u));
      if (deduped.length > 0) {
        segments.push({ kind: "images", urls: deduped, top: segments.length === 0 });
        deduped.forEach((u) => emittedImgUrls.add(u));
      }
    } else if (b.type === "html") {
      const { segs, imgUrls } = segmentsFromHtml(b.value);
      for (const s of segs) segments.push(s);
      imgUrls.forEach((u) => emittedImgUrls.add(u));
      i++;
    } else {
      const texts: string[] = [];
      while (i < blocks.length && blocks[i].type === "text") { texts.push((blocks[i] as any).value); i++; }
      segments.push({ kind: "text", value: texts.join("\n") });
    }
  }
  return segments;
}
