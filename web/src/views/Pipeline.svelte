<script lang="ts">
  import { fetchPipeline } from "../lib/api";
  import type { PipelineProject } from "../lib/types";
  import ProjectIcon from "../lib/ProjectIcon.svelte";

  let projects: PipelineProject[] = $state([]);
  let loading = $state(true);

  $effect(() => {
    fetchPipeline().then((data) => {
      projects = data.sort((a, b) => b.priority_score - a.priority_score);
      loading = false;
    });
  });

  function typeShort(t: string): string {
    if (t === "study") return "S";
    if (t === "library") return "L";
    return "";
  }

  const stages = [
    { label: "Idea", pos: 0 },
    { label: "PoC", pos: 15 },
    { label: "MVP", pos: 40 },
    { label: "Beta", pos: 70 },
    { label: "Shipped", pos: 90 },
  ];

  function stageLabel(readiness: number): string {
    if (readiness >= 91) return "Shipped";
    if (readiness >= 71) return "Beta";
    if (readiness >= 41) return "MVP";
    if (readiness >= 16) return "PoC";
    return "Idea";
  }

  function fillColour(readiness: number): string {
    if (readiness >= 91) return "var(--success)";
    if (readiness >= 71) return "var(--accent)";
    if (readiness >= 41) return "var(--warning)";
    return "var(--ink-faint)";
  }
</script>

<p class="eyebrow">Pipeline</p>

{#if loading}
  <div class="loading">Loading...</div>
{:else if projects.length === 0}
  <div class="empty-state">No active projects.</div>
{:else}
  <div class="pipeline">
    <div class="pipeline-header">
      <div class="pipeline-label-col"></div>
      <div class="pipeline-stages">
        {#each stages as s}
          <span class="stage-label" style="left: {s.pos}%">{s.label}</span>
        {/each}
      </div>
      <div class="pipeline-pct-col"></div>
    </div>

    {#each projects as p}
      <div class="pipeline-lane" onclick={() => (window.location.hash = `#/project/${p.id}`)}>
        <div class="pipeline-label-col">
          <ProjectIcon name={p.name} size={20} />
          <span class="pipeline-name">{p.name}</span>
          {#if typeShort(p.project_type)}
            <span class="pipeline-type">{typeShort(p.project_type)}</span>
          {/if}
        </div>
        <div class="pipeline-track-wrap">
          {#each stages.slice(1) as s}
            <div class="stage-divider" style="left: {s.pos}%"></div>
          {/each}
          <div class="track-fill" style="width: {p.readiness}%; background: {fillColour(p.readiness)}"></div>
          <div class="track-head" style="left: {p.readiness}%; border-left-color: {fillColour(p.readiness)}"></div>
          {#each p.milestones as m}
            <div
              class="milestone-dot"
              class:milestone-done={m.progress >= 1.0}
              style="left: {m.progress * 100}%"
              title="{m.name} ({Math.round(m.progress * 100)}%){m.target ? ' — target: ' + m.target : ''}"
            ></div>
          {/each}
        </div>
        <span class="pipeline-pct-col" style="color: {fillColour(p.readiness)}">{p.readiness}%</span>
      </div>
    {/each}
  </div>
{/if}
