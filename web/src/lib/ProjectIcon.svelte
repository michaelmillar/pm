<script lang="ts">
  import { projectColour, projectMonogram, projectEmoji } from "./icons";

  let { name, size = 28 }: { name: string; size?: number } = $props();

  let colour = $derived(projectColour(name));
  let mono = $derived(projectMonogram(name));
  let emoji = $derived(projectEmoji(name));
  let fontSize = $derived(Math.round(size * 0.42));
  let emojiFontSize = $derived(Math.round(size * 0.55));
  let textY = $derived(Math.round(size * 0.62));
  let emojiY = $derived(Math.round(size * 0.72));
  let radius = $derived(Math.round(size * 0.22));
</script>

<svg
  width={size}
  height={size}
  viewBox="0 0 {size} {size}"
  class="project-icon"
  aria-hidden="true"
>
  {#if emoji}
    <text
      x={size / 2}
      y={emojiY}
      font-size={emojiFontSize}
      text-anchor="middle"
    >{emoji}</text>
  {:else}
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
  {/if}
</svg>

<style>
  .project-icon {
    flex-shrink: 0;
    vertical-align: middle;
  }
</style>
