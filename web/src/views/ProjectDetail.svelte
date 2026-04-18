<script lang="ts">
  import { fetchProject } from "../lib/api";
  import type { ProjectDetail as PD } from "../lib/types";
  import RadarChart from "../lib/RadarChart.svelte";

  let { id }: { id: number } = $props();
  let detail: PD | null = $state(null);
  let loading = $state(true);

  $effect(() => {
    fetchProject(id).then((data) => {
      detail = data;
      loading = false;
    });
  });

  function actionColour(action: string): string {
    switch (action) {
      case "KILL": return "var(--danger)";
      case "PIVOT": return "var(--warning)";
      case "PUSH": return "var(--success)";
      case "GROOM": return "var(--accent)";
      case "INTEGRATE": return "#9b59b6";
      case "SUSTAIN": return "#3498db";
      default: return "var(--ink-soft)";
    }
  }

  function stageProgress(stage: number): number {
    return (stage / 5) * 100;
  }
</script>

{#if loading}
  <div class="loading">Loading...</div>
{:else if detail}
  <div class="detail-header">
    <h2>{detail.name}</h2>
    <span class="type-badge">{detail.archetype}</span>
    <span class="stage-badge">{detail.stage_label}</span>
  </div>

  <div class="detail-grid">
    <div class="detail-card">
      <h3>Score</h3>
      <div class="big-number">{detail.score}</div>
      <div class="action-display" style="color: {actionColour(detail.action)}">
        {detail.action}
        {#if detail.action_target}
          <span class="action-target">&rarr; {detail.action_target}</span>
        {/if}
      </div>
    </div>

    <div class="detail-card">
      <h3>Axes</h3>
      <RadarChart
        labels={["Velocity", "Fit", "Distinct", "Leverage"]}
        values={[
          detail.velocity ?? 0,
          detail.fit_signal ?? 0,
          detail.distinctness ?? 0,
          detail.leverage ?? 0,
        ]}
        max={10}
      />
    </div>

    <div class="detail-card">
      <h3>Lifecycle</h3>
      <div class="stage-bar">
        <div class="stage-fill" style="width: {stageProgress(detail.stage)}%"></div>
      </div>
      <div class="stage-labels">
        {#each ["idea", "spike", "proto", "valid", "ship", "traction"] as label, i}
          <span class:active={detail.stage >= i}>{label}</span>
        {/each}
      </div>
    </div>

    <div class="detail-card">
      <h3>Meta</h3>
      <dl>
        <dt>Stale</dt><dd>{detail.days_stale}d</dd>
        <dt>Pivots</dt><dd>{detail.pivot_count}</dd>
        {#if detail.sunk_cost_days != null}
          <dt>Sunk</dt><dd>{detail.sunk_cost_days}d</dd>
        {/if}
        <dt>Created</dt><dd>{detail.created_at}</dd>
        {#if detail.path}
          <dt>Path</dt><dd class="mono">{detail.path}</dd>
        {/if}
      </dl>
    </div>
  </div>

  {#if detail.research_summary}
    <details class="detail-section" open>
      <summary>Research</summary>
      <div class="detail-section-body research-body">{detail.research_summary}</div>
    </details>
  {/if}

  {#if detail.inbox_note}
    <details class="detail-section" open>
      <summary>Notes</summary>
      <div class="detail-section-body">{detail.inbox_note}</div>
    </details>
  {/if}
{:else}
  <div class="empty-state">Project not found.</div>
{/if}
