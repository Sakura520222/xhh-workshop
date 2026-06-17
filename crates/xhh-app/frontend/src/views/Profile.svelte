<script lang="ts">
  import { onMount } from "svelte";
  import { userProfile, followingList, followerList, userEvents } from "../lib/api";
  import { setSelectedLinkId, setView } from "../lib/stores.svelte";
  import PostCard from "../components/PostCard.svelte";

  type DetailView = null | "following" | "followers" | "posts";

  let profile = $state<any>(null);
  let loading = $state(true);
  let avatarBroken = $state(false);
  let showAvatar = $derived(!!profile?.avatar && !avatarBroken);

  // 详情子视图
  let detailView = $state<DetailView>(null);
  let detailItems = $state<any[]>([]);
  let detailLoading = $state(false);
  let detailError = $state("");
  let detailOffset = $state(0);
  let detailHasMore = $state(true);
  let detailLastval = $state("");
  let detailSentinel: HTMLElement | undefined = $state();
  let detailObserver: IntersectionObserver | null = null;
  let detailPrevSentinel: HTMLElement | undefined;

  $effect(() => {
    if (detailPrevSentinel) { detailObserver?.unobserve(detailPrevSentinel); detailPrevSentinel = undefined; }
    if (!detailSentinel) return;
    if (!detailObserver) {
      const scroller = document.querySelector(".content") as HTMLElement | null;
      detailObserver = new IntersectionObserver(
        (entries) => { if (entries[0]?.isIntersecting) loadMoreDetail(); },
        { root: scroller, rootMargin: "200px" }
      );
    }
    detailObserver.observe(detailSentinel);
    detailPrevSentinel = detailSentinel;
  });

  onMount(() => {
    (async () => {
      try {
        const v = await userProfile();
        profile = v?.result?.account_detail ?? null;
      } finally {
        loading = false;
      }
    })();
    return () => { detailObserver?.disconnect(); detailObserver = null; };
  });

  function getUserid(): string {
    return String(profile?.userid ?? "");
  }

  async function openDetail(view: DetailView) {
    detailView = view;
    detailItems = [];
    detailOffset = 0;
    detailHasMore = true;
    detailLastval = "";
    detailError = "";
    detailLoading = true;
    try {
      const uid = getUserid();
      if (view === "following") {
        const v = await followingList(uid, 0, 50);
        detailItems = v?.follow_list ?? [];
        detailOffset = detailItems.length;
        if (detailItems.length < 50) detailHasMore = false;
      } else if (view === "followers") {
        const v = await followerList(uid, 0, 50);
        detailItems = v?.follow_list ?? [];
        detailOffset = detailItems.length;
        if (detailItems.length < 50) detailHasMore = false;
      } else if (view === "posts") {
        const v = await userEvents(uid, "");
        const moments = v?.result?.moments ?? [];
        detailItems = moments;
        detailLastval = v?.result?.lastval ?? "";
        detailOffset = moments.length;
        if (!detailLastval) detailHasMore = false;
      }
    } catch (e) {
      detailError = String(e);
    } finally {
      detailLoading = false;
    }
  }

  async function loadMoreDetail() {
    if (detailLoading || !detailHasMore || !detailView) return;
    detailLoading = true;
    const uid = getUserid();
    try {
      if (detailView === "following") {
        const v = await followingList(uid, detailOffset, 50);
        const items = v?.follow_list ?? [];
        if (items.length === 0) { detailHasMore = false; }
        else { detailItems = [...detailItems, ...items]; detailOffset += items.length; if (items.length < 50) detailHasMore = false; }
      } else if (detailView === "followers") {
        const v = await followerList(uid, detailOffset, 50);
        const items = v?.follow_list ?? [];
        if (items.length === 0) { detailHasMore = false; }
        else { detailItems = [...detailItems, ...items]; detailOffset += items.length; if (items.length < 50) detailHasMore = false; }
      } else if (detailView === "posts") {
        const v = await userEvents(uid, detailLastval);
        const moments = v?.result?.moments ?? [];
        if (moments.length === 0 || !v?.result?.lastval) { detailHasMore = false; }
        else { detailItems = [...detailItems, ...moments]; detailLastval = v.result.lastval; detailOffset += moments.length; }
      }
    } catch (e) {
      detailError = String(e);
      detailHasMore = false;
    } finally {
      detailLoading = false;
    }
  }

  function closeDetail() {
    detailView = null;
    detailItems = [];
  }

  function openPost(linkId: string | number) {
    if (!linkId) return;
    setSelectedLinkId(String(linkId));
    setView("detail");
  }

  const viewTitle = $derived(
    detailView === "following" ? "关注" :
    detailView === "followers" ? "粉丝" :
    detailView === "posts" ? "帖子" : ""
  );
</script>

<div class="profile-page">
  {#if loading}
    <div class="status">加载中...</div>
  {:else if detailView}
    <div class="topbar">
      <button class="back-btn" onclick={closeDetail}>返回</button>
      <span class="topbar-title">{viewTitle}</span>
    </div>
    {#if detailLoading && detailItems.length === 0}
      <div class="status">加载中...</div>
    {:else if detailError}
      <div class="status error">{detailError}</div>
    {:else if detailItems.length === 0}
      <div class="status">暂无数据</div>
    {:else}
      <div class="detail-list">
        {#each detailItems as item}
          {#if detailView === "posts"}
            <PostCard post={item} onOpen={() => openPost(item.linkid)} />
          {:else}
            <div class="user-item">
              {#if item.avatar}
                <img src={item.avatar} alt="" class="u-avatar" />
              {:else}
                <div class="u-avatar placeholder">{item.username?.charAt(0) ?? "?"}</div>
              {/if}
              <div class="u-info">
                <div class="u-name">{item.username ?? "未知用户"}</div>
                <div class="u-sub">ID: {item.userid}{#if item.level_info?.level} · Lv{item.level_info.level}{/if}</div>
              </div>
              {#if item.medals?.length}
                <div class="u-medals">
                  {#each item.medals.slice(0, 3) as m}
                    {#if m.img_url}
                      <img src={m.img_url} alt="" class="medal-icon" />
                    {/if}
                  {/each}
                </div>
              {/if}
            </div>
          {/if}
        {/each}
        {#if detailHasMore}
          <div bind:this={detailSentinel} class="sentinel"></div>
        {/if}
        {#if detailLoading}
          <div class="status small">加载更多...</div>
        {/if}
      </div>
    {/if}
  {:else if profile}
    <div class="profile-card">
      <div class="avatar">
        {#if showAvatar}
          <img src={profile.avatar} alt={profile.username} class="avatar-img" onerror={() => (avatarBroken = true)} />
        {:else}
          {profile.username?.charAt(0) ?? "?"}
        {/if}
      </div>
      <div class="info">
        <h2>{profile.username}</h2>
        <div class="meta-row">
          <span>Lv{profile?.level_info?.level ?? "?"}</span>
          {#if profile?.ip_location}<span>{profile.ip_location}</span>{/if}
        </div>
        {#if profile?.signature}
          <p class="signature">{profile.signature}</p>
        {/if}
      </div>
    </div>

    <div class="stats">
      <button class="stat" onclick={() => openDetail("following")}>
        <div class="num">{profile?.bbs_info?.follow_num ?? 0}</div>
        <div class="label">关注</div>
      </button>
      <button class="stat" onclick={() => openDetail("followers")}>
        <div class="num">{profile?.bbs_info?.fan_num ?? 0}</div>
        <div class="label">粉丝</div>
      </button>
      <button class="stat" onclick={() => openDetail("posts")}>
        <div class="num">{profile?.bbs_info?.post_link_num ?? 0}</div>
        <div class="label">帖子</div>
      </button>
      <div class="stat inactive">
        <div class="num">{profile?.bbs_info?.awd_num ?? 0}</div>
        <div class="label">获赞</div>
      </div>
    </div>
  {:else}
    <div class="status">获取主页失败</div>
  {/if}
</div>

<style>
  .profile-page {
    max-width: 720px;
    margin: 0 auto;
  }
  .topbar {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 8px;
    margin-left: -10px;
    margin-right: -10px;
    margin-bottom: 20px;
    position: sticky;
    top: 8px;
    z-index: 10;
    border-radius: 22px;
    background: linear-gradient(
      180deg,
      rgba(255, 255, 255, 0.22) 0%,
      rgba(255, 255, 255, 0.10) 100%
    );
    backdrop-filter: blur(40px) saturate(1.8) brightness(1.1);
    -webkit-backdrop-filter: blur(40px) saturate(1.8) brightness(1.1);
    border: 0.5px solid rgba(255, 255, 255, 0.35);
    box-shadow:
      inset 0 1px 0 rgba(255, 255, 255, 0.6),
      inset 0 -0.5px 0 rgba(255, 255, 255, 0.15),
      0 8px 40px rgba(0, 0, 0, 0.10),
      0 2px 12px rgba(0, 0, 0, 0.06);
  }
  .back-btn {
    padding: 6px 14px;
    border-radius: 14px;
    background: rgba(255, 255, 255, 0.12);
    border: 0.5px solid rgba(255, 255, 255, 0.2);
    color: var(--text);
    font-size: 13px;
    box-shadow: inset 0 0.5px 0 rgba(255, 255, 255, 0.3);
    transition: all var(--duration-fast) var(--ease-out);
  }
  .back-btn:hover {
    background: rgba(255, 255, 255, 0.2);
  }
  .topbar-title {
    font-size: 15px;
    font-weight: 500;
  }
  .profile-card {
    display: flex;
    gap: 20px;
    padding: 24px;
    background: var(--glass-bg);
    backdrop-filter: var(--glass-blur);
    -webkit-backdrop-filter: var(--glass-blur);
    border-radius: var(--radius);
    margin-bottom: 20px;
    border: 0.5px solid var(--glass-border);
    box-shadow: var(--elevation-1);
  }
  .avatar {
    width: 80px;
    height: 80px;
    border-radius: 50%;
    background: linear-gradient(135deg, var(--accent), #ff9558);
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 36px;
    font-weight: 600;
    overflow: hidden;
    flex-shrink: 0;
    box-shadow: 0 0 0 3px rgba(255, 107, 53, 0.2);
  }
  .avatar-img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  .info {
    flex: 1;
    min-width: 0;
  }
  h2 {
    font-size: 22px;
    margin-bottom: 8px;
  }
  .meta-row {
    display: flex;
    gap: 16px;
    font-size: 13px;
    color: var(--text-secondary);
    margin-bottom: 8px;
  }
  .signature {
    font-size: 14px;
    color: var(--text-secondary);
    line-height: 1.5;
  }
  .stats {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 12px;
  }
  .stat {
    background: var(--glass-bg);
    backdrop-filter: var(--glass-blur);
    -webkit-backdrop-filter: var(--glass-blur);
    border-radius: var(--radius);
    padding: 20px;
    text-align: center;
    cursor: pointer;
    border: none;
    color: var(--text);
    width: 100%;
    border: 0.5px solid var(--glass-border);
    box-shadow: var(--elevation-1);
    transition: all var(--duration-normal) var(--ease-out);
  }
  .stat:hover {
    background: var(--glass-hover);
    border-color: rgba(255, 255, 255, 0.12);
    box-shadow: var(--elevation-2);
  }
  .stat.inactive {
    cursor: default;
    opacity: 0.5;
  }
  .stat.inactive:hover {
    background: var(--glass-bg);
    border-color: var(--glass-border);
    box-shadow: var(--elevation-1);
  }
  .num {
    font-size: 24px;
    font-weight: 700;
  }
  .label {
    font-size: 12px;
    color: var(--text-secondary);
    margin-top: 4px;
  }
  .detail-list {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .user-item {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 14px 16px;
    border-radius: var(--radius);
    background: var(--glass-bg);
    border: 0.5px solid var(--glass-border);
    box-shadow: var(--elevation-1);
    transition: all var(--duration-normal) var(--ease-out);
  }
  .user-item:hover {
    background: var(--glass-hover);
    border-color: rgba(255, 255, 255, 0.12);
  }
  .u-avatar {
    width: 44px;
    height: 44px;
    border-radius: 50%;
    object-fit: cover;
    flex-shrink: 0;
  }
  .u-avatar.placeholder {
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--accent);
    color: white;
    font-weight: 600;
  }
  .u-info {
    flex: 1;
    min-width: 0;
    overflow: hidden;
  }
  .u-name {
    font-size: 14px;
    font-weight: 500;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .u-sub {
    font-size: 12px;
    color: var(--text-secondary);
    margin-top: 2px;
  }
  .u-medals {
    display: flex;
    gap: 4px;
    flex-shrink: 0;
  }
  .medal-icon {
    width: 18px;
    height: 18px;
    border-radius: 50%;
    object-fit: cover;
  }
  .status {
    text-align: center;
    padding: 60px 0;
    color: var(--text-secondary);
  }
  .status.small {
    padding: 16px 0;
    font-size: 13px;
  }
  .status.error {
    color: var(--danger);
  }
  .sentinel {
    height: 1px;
  }
</style>
