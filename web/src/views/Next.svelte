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
        <span class="axis-value">{rec.project.score}</span>
        <span class="axis-label">Score</span>
      </div>
      <div class="axis">
        <span class="axis-value">{rec.project.stage_label}</span>
        <span class="axis-label">Stage</span>
      </div>
      <div class="axis">
        <span class="axis-value">{rec.project.action}</span>
        <span class="axis-label">Action</span>
      </div>
      <div class="axis">
        <span class="axis-value">{rec.project.days_stale}d</span>
        <span class="axis-label">Stale</span>
      </div>
    </div>
  </div>
{:else}
  <div class="empty-state">{rec?.reason ?? "No recommendation available."}</div>
{/if}
