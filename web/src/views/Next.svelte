<script lang="ts">
  import { fetchNext } from "../lib/api";
  import type { NextRecommendation } from "../lib/types";
  import ProjectIcon from "../lib/ProjectIcon.svelte";

  let rec: NextRecommendation | null = $state(null);
  let loading = $state(true);

  $effect(() => {
    fetchNext().then((data) => {
      rec = data;
      loading = false;
    });
  });
</script>

<p class="eyebrow">What to work on next</p>

{#if loading}
  <div class="loading">Loading...</div>
{:else if rec?.project}
  <div class="next-card">
    <h2>
      <ProjectIcon name={rec.project.name} size={36} />
      <a href="#/project/{rec.project.id}">{rec.project.name}</a>
    </h2>
    <p class="next-reason">{rec.reason}</p>

    <div class="axes-row" style="justify-content: center; margin-top: 1.5rem">
      <div class="axis">
        <span class="axis-value">{rec.project.priority_score}</span>
        <span class="axis-label">Score</span>
      </div>
      <div class="axis">
        <span class="axis-value">{rec.project.readiness}%</span>
        <span class="axis-label">Readiness</span>
      </div>
      <div class="axis">
        <span class="axis-value">{rec.project.days_stale}d</span>
        <span class="axis-label">Stale</span>
      </div>
    </div>

    {#if rec.project.next_milestone}
      <p style="margin-top: 1.5rem; font-size: 0.95rem; color: var(--ink-soft)">
        Next milestone: <strong>{rec.project.next_milestone}</strong>
      </p>
    {/if}
  </div>
{:else}
  <div class="empty-state">{rec?.reason ?? "No recommendation available."}</div>
{/if}
