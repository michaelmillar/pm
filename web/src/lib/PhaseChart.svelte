<script lang="ts">
  import type { Phase } from "./types";

  let { phases }: { phases: Phase[] } = $props();

  const barH = 18;
  const gap = 10;
  const labelW = 130;
  const barW = 360;
  const pctW = 48;
  const weightW = 60;
  const pad = 4;
  const totalW = labelW + barW + pctW + weightW;
  let totalH = $derived(phases.length * (barH + gap) + pad);

  function fillColour(progress: number): string {
    if (progress >= 0.9) return "#1c7a3f";
    if (progress >= 0.5) return "#1e3a5f";
    if (progress >= 0.25) return "#b8860b";
    return "#999";
  }
</script>

<svg
  width="100%"
  viewBox="0 0 {totalW} {totalH}"
  xmlns="http://www.w3.org/2000/svg"
  style="font-family: 'STIX Two Text', Georgia, serif"
>
  {#each phases as phase, i}
    {@const y = i * (barH + gap) + pad}
    {@const pct = Math.round(phase.progress * 100)}
    <text
      x={labelW - 8}
      y={y + barH / 2 + 1}
      font-size="11"
      fill="#333"
      text-anchor="end"
      dominant-baseline="middle"
    >
      {phase.label}
    </text>
    <rect x={labelW} y={y} width={barW} height={barH} fill="#e8e8e8" rx="2" />
    <rect
      x={labelW}
      y={y}
      width={barW * phase.progress}
      height={barH}
      fill={fillColour(phase.progress)}
      rx="2"
    />
    <text
      x={labelW + barW + 8}
      y={y + barH / 2 + 1}
      font-size="11"
      fill="#333"
      dominant-baseline="middle"
      font-family="'Iosevka Term', 'Cascadia Code', 'SF Mono', monospace"
    >
      {pct}%
    </text>
    <text
      x={labelW + barW + pctW + 8}
      y={y + barH / 2 + 1}
      font-size="9"
      fill="#999"
      dominant-baseline="middle"
    >
      w={phase.weight}
    </text>
  {/each}
</svg>
