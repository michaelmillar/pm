<script lang="ts">
  import type { PortfolioStats } from "./types";

  let { stats }: { stats: PortfolioStats } = $props();

  const actionColours: Record<string, string> = {
    PUSH: "var(--success)",
    GROOM: "var(--accent)",
    KILL: "var(--danger)",
    PIVOT: "var(--warning)",
    OBSERVE: "var(--ink-faint)",
    SUSTAIN: "var(--accent)",
    INTEGRATE: "var(--ink-soft)",
    REPURPOSE: "var(--warning)",
  };

  let topActions = $derived(
    [...stats.by_action]
      .sort((a, b) => b.count - a.count)
      .slice(0, 3)
  );
</script>

<div class="stats-row">
  <div class="stat-card">
    <div class="stat-label">Total</div>
    <div class="stat-value">{stats.total}</div>
    <div class="stat-sub">{stats.scored} scored / {stats.unscored} unscored</div>
  </div>

  <div class="stat-card">
    <div class="stat-label">Avg Score</div>
    <div class="stat-value">{stats.avg_score.toFixed(1)}</div>
  </div>

  <div class="stat-card">
    <div class="stat-label">Actions</div>
    <div class="action-list">
      {#each topActions as item}
        <div class="action-item">
          <span class="action-dot" style="background: {actionColours[item.action] ?? 'var(--ink-faint)'}"></span>
          <span>{item.action}</span>
          <span class="action-count">{item.count}</span>
        </div>
      {/each}
    </div>
  </div>

  <div class="stat-card">
    <div class="stat-label">Avg Stale</div>
    <div class="stat-value">{Math.round(stats.avg_staleness)}d</div>
  </div>
</div>
