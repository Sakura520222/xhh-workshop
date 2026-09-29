<script lang="ts">
  // Linux(GNOME Wayland) 无边框窗口没有可用的系统缩放边框（WebKitGTK 不向 GtkWindow 冒泡指针事件，
  // tao 内置的边缘 hit-test 收不到），用不可见边缘条转发 startResizeDragging
  import { getCurrentWindow, type ResizeDirection } from "@tauri-apps/api/window";

  const win = getCurrentWindow();
  const isLinux = navigator.userAgent.includes("Linux");

  const edges: { dir: ResizeDirection; pos: string; cursor: string }[] = [
    { dir: "North", pos: "top: 0; left: 14px; right: 14px; height: 6px", cursor: "ns-resize" },
    { dir: "South", pos: "bottom: 0; left: 14px; right: 14px; height: 6px", cursor: "ns-resize" },
    { dir: "West", pos: "left: 0; top: 14px; bottom: 14px; width: 6px", cursor: "ew-resize" },
    { dir: "East", pos: "right: 0; top: 14px; bottom: 14px; width: 6px", cursor: "ew-resize" },
    { dir: "NorthWest", pos: "top: 0; left: 0; width: 14px; height: 14px", cursor: "nwse-resize" },
    { dir: "NorthEast", pos: "top: 0; right: 0; width: 14px; height: 14px", cursor: "nesw-resize" },
    { dir: "SouthWest", pos: "bottom: 0; left: 0; width: 14px; height: 14px", cursor: "nesw-resize" },
    { dir: "SouthEast", pos: "bottom: 0; right: 0; width: 14px; height: 14px", cursor: "nwse-resize" },
  ];

  function start(e: MouseEvent, dir: ResizeDirection) {
    if (e.button !== 0) return;
    e.preventDefault();
    win.startResizeDragging(dir);
  }
</script>

{#if isLinux}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  {#each edges as edge (edge.dir)}
    <div class="resize-edge" style="{edge.pos}; cursor: {edge.cursor}" onmousedown={(e) => start(e, edge.dir)}></div>
  {/each}
{/if}

<style>
  .resize-edge {
    position: fixed;
    z-index: 9999;
  }
</style>
