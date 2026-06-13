import assert from "node:assert/strict";

import { parsePostContent } from "./content.ts";

const htmlWithImageSummary = JSON.stringify([
  {
    type: "html",
    text: '<p class="com-text">第一段 <b>重点</b></p><p><img data-original="https://img.example/a.webp" src="https://thumb.example/a.webp" /></p><p>第二段</p>',
  },
  { type: "img", url: "https://img.example/a.webp" },
]);

assert.deepEqual(parsePostContent(htmlWithImageSummary), [
  { kind: "html", value: '<p class="com-text">第一段 <b>重点</b></p>' },
  { kind: "images", urls: ["https://img.example/a.webp"], top: false },
  { kind: "html", value: "<p>第二段</p>" },
]);

const topImagesThenText = JSON.stringify([
  { type: "img", url: "https://img.example/1.webp" },
  { type: "img", url: "https://img.example/2.webp" },
  { type: "text", text: "正文" },
]);

assert.deepEqual(parsePostContent(topImagesThenText), [
  { kind: "images", urls: ["https://img.example/1.webp", "https://img.example/2.webp"], top: true },
  { kind: "text", value: "正文" },
]);
