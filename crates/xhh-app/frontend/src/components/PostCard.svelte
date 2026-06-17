<script lang="ts">
  import { renderTextSync, getEmojiVersion } from "../lib/render.svelte";
  let { post, onLike, onOpen }: { post: any; onLike?: () => Promise<void>; onOpen?: () => void } = $props();

  let title = $derived(post?.title ?? "(无标题)");
  let emojiVer = $derived(getEmojiVersion());

  function rt(text: string): string {
    void emojiVer;
    return renderTextSync(text);
  }
  let author = $derived(post?.user?.username ?? "?");
  let avatar = $derived(post?.user?.avatar ?? post?.user?.avatar_url ?? "");
  let comments = $derived(post?.comment_num ?? 0);
  let likes = $derived(post?.link_award_num ?? 0);
  let liked = $derived(post?.is_award_link === 1 || post?.is_award_link === true);
  let topic = $derived(post?.topics?.[0]?.name ?? "");
  let imgs = $derived<any[]>(post?.imgs ?? []);
  let linkId = $derived(String(post?.linkid ?? "?"));
  let likeBusy = $state(false);

  async function handleLike(e: MouseEvent) {
    e.stopPropagation();
    if (likeBusy || !onLike) return;
    likeBusy = true;
    try {
      await onLike();
    } finally {
      likeBusy = false;
    }
  }
</script>

<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
<div
  class="post-card"
  class:clickable={!!onOpen}
  role={onOpen ? "button" : undefined}
  tabindex={onOpen ? 0 : undefined}
  onclick={onOpen}
  onkeydown={(e) => {
    if (onOpen && (e.key === "Enter" || e.key === " ")) {
      e.preventDefault();
      onOpen();
    }
  }}
>
  <div class="post-header">
    <div class="author-block">
      <div class="author-avatar" aria-hidden="true">
        {#if avatar}
          <img src={avatar} alt="" />
        {:else}
          {author.charAt(0) || "?"}
        {/if}
      </div>
      <div class="author-copy">
        <div class="author">{author}</div>
        <div class="meta-line">帖子 ID {linkId}</div>
      </div>
    </div>
    {#if topic}<span class="topic">{topic}</span>{/if}
  </div>

  <h3 class="title">{@html rt(title)}</h3>

  {#if imgs.length > 0}
    <div class="imgs" class:single={imgs.length === 1}>
      {#each imgs.slice(0, 3) as img, index}
        <img src={img} alt={`${title} 配图 ${index + 1}`} class="thumb" />
      {/each}
    </div>
  {/if}

  <div class="post-footer">
    <span class="stat">{comments} 评论</span>
    <span class="stat">{likes} 点赞</span>
    <button
      class="action"
      class:liked={liked}
      class:busy={likeBusy}
      disabled={likeBusy}
      onclick={handleLike}
      aria-label={liked ? "取消点赞" : "点赞"}
    >
      <svg width="15" height="15" viewBox="0 0 24 24" fill={liked ? "currentColor" : "none"} stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
        <path d="M20.84 4.61a5.5 5.5 0 0 0-7.78 0L12 5.67l-1.06-1.06a5.5 5.5 0 0 0-7.78 7.78L12 21.23l8.84-8.84a5.5 5.5 0 0 0 0-7.78Z"/>
      </svg>
      <span>{liked ? "已赞" : "点赞"}</span>
    </button>
  </div>
</div>

<style>
  .post-card {
    position: relative;
    overflow: hidden;
    background: linear-gradient(180deg, color-mix(in srgb, var(--bg-soft) 76%, transparent), color-mix(in srgb, var(--bg-soft) 57%, transparent));
    backdrop-filter: var(--glass-blur);
    -webkit-backdrop-filter: var(--glass-blur);
    border-radius: var(--radius-lg);
    padding: 18px;
    border: 1px solid var(--glass-border);
    box-shadow: var(--elevation-1);
    transition: transform var(--duration-normal) var(--ease-out), border-color var(--duration-normal) var(--ease-out), box-shadow var(--duration-normal) var(--ease-out), background var(--duration-normal) var(--ease-out);
  }

  .post-card::before {
    content: "";
    position: absolute;
    inset: 0;
    pointer-events: none;
    background: linear-gradient(135deg, color-mix(in srgb, var(--accent) 12%, transparent), transparent 38%, color-mix(in srgb, var(--accent-warm) 8%, transparent));
    opacity: 0;
    transition: opacity var(--duration-normal) var(--ease-out);
  }

  .post-card:hover {
    transform: translateY(-2px);
    border-color: color-mix(in srgb, var(--accent-hover) 30%, transparent);
    box-shadow: var(--elevation-2);
    background: linear-gradient(180deg, color-mix(in srgb, var(--bg-soft) 86%, transparent), color-mix(in srgb, var(--bg-soft) 66%, transparent));
  }

  .post-card:hover::before {
    opacity: 1;
  }

  .post-card.clickable {
    cursor: pointer;
  }

  .post-header,
  .title,
  .imgs,
  .post-footer {
    position: relative;
  }

  .post-header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 12px;
    margin-bottom: 14px;
  }

  .author-block {
    display: flex;
    align-items: center;
    gap: 10px;
    min-width: 0;
  }

  .author-avatar {
    width: 38px;
    height: 38px;
    display: grid;
    place-items: center;
    flex-shrink: 0;
    border-radius: 14px;
    overflow: hidden;
    background: linear-gradient(135deg, var(--accent), var(--accent-warm));
    color: white;
    font-size: 14px;
    font-weight: 800;
    box-shadow: 0 0 0 3px color-mix(in srgb, var(--accent) 12%, transparent);
  }

  .author-avatar img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .author-copy {
    min-width: 0;
  }

  .author {
    font-size: 14px;
    font-weight: 750;
    color: var(--text-strong);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .meta-line {
    margin-top: 3px;
    font-size: 12px;
    color: var(--text-muted);
  }

  .topic {
    max-width: 180px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 12px;
    font-weight: 700;
    padding: 6px 10px;
    border-radius: 999px;
    background: var(--accent-soft);
    color: var(--on-accent-soft);
    border: 1px solid color-mix(in srgb, var(--accent-hover) 18%, transparent);
  }

  .title {
    font-size: 17px;
    font-weight: 750;
    line-height: 1.55;
    color: var(--text-strong);
    letter-spacing: -0.01em;
    margin-bottom: 14px;
  }

  .imgs {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 10px;
    margin-bottom: 14px;
  }

  .imgs.single {
    grid-template-columns: minmax(0, 1fr);
  }

  .thumb {
    width: 100%;
    height: 112px;
    object-fit: cover;
    border-radius: 14px;
    border: 1px solid rgba(148, 163, 184, 0.18);
    background: color-mix(in srgb, var(--bg-soft) 72%, transparent);
  }

  .imgs.single .thumb {
    height: 220px;
  }

  .post-footer {
    display: flex;
    align-items: center;
    gap: 10px;
    color: var(--text-secondary);
    font-size: 12px;
  }

  .stat {
    display: inline-flex;
    align-items: center;
    min-height: 28px;
    padding: 4px 10px;
    border-radius: 999px;
    background: rgba(148, 163, 184, 0.08);
    border: 1px solid rgba(148, 163, 184, 0.1);
  }

  .action {
    margin-left: auto;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 7px;
    min-height: 34px;
    padding: 7px 13px;
    border-radius: 999px;
    background: color-mix(in srgb, var(--bg-soft) 72%, transparent);
    border: 1px solid rgba(148, 163, 184, 0.18);
    color: var(--text-secondary);
    font-size: 12px;
    font-weight: 700;
    transition: all var(--duration-fast) var(--ease-out);
  }

  .action:hover:not(:disabled) {
    background: var(--accent-soft);
    color: var(--on-accent-soft);
    border-color: color-mix(in srgb, var(--accent-hover) 30%, transparent);
  }

  .action.liked {
    background: linear-gradient(135deg, var(--accent), var(--accent-strong));
    color: white;
    border-color: rgba(191, 219, 254, 0.22);
    box-shadow: 0 10px 22px color-mix(in srgb, var(--accent-strong) 28%, transparent);
  }

  .action.busy {
    opacity: 0.58;
  }

  .title :global(.emoji) {
    width: 1em;
    height: 1em;
    vertical-align: middle;
    display: inline-block;
  }

  @media (max-width: 720px) {
    .imgs {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }

    .thumb {
      height: 96px;
    }
  }
</style>
