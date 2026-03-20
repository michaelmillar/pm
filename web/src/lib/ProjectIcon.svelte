<script lang="ts">
  import { projectColour, projectMonogram } from "./icons";

  let { name, size = 28 }: { name: string; size?: number } = $props();

  let colour = $derived(projectColour(name));
  let mono = $derived(projectMonogram(name));
  let fontSize = $derived(Math.round(size * 0.42));
  let textY = $derived(Math.round(size * 0.62));
  let radius = $derived(Math.round(size * 0.22));
</script>

<svg
  width={size}
  height={size}
  viewBox="0 0 {size} {size}"
  class="project-icon"
  aria-hidden="true"
>
  <rect width={size} height={size} rx={radius} fill={colour} />
  <text
    x={size / 2}
    y={textY}
    font-family="system-ui,-apple-system,sans-serif"
    font-size={fontSize}
    font-weight="600"
    fill="#fff"
    text-anchor="middle"
  >{mono}</text>
</svg>

<style>
  .project-icon {
    flex-shrink: 0;
    vertical-align: middle;
  }
</style>
