<script lang="ts">
  import type { Project } from "./types";
  import ProjectIcon from "./ProjectIcon.svelte";

  let { project }: { project: Project } = $props();

  function actionColour(action: string): string {
    switch (action) {
      case "KILL": return "var(--danger)";
      case "PIVOT": return "var(--warning)";
      case "PUSH": return "var(--success)";
      case "GROOM": return "var(--accent)";
      case "INTEGRATE": return "var(--info, #9b59b6)";
      case "SUSTAIN": return "var(--info, #3498db)";
      case "REPURPOSE": return "var(--warning)";
      default: return "var(--ink-soft)";
    }
  }

  function staleColour(days: number): string {
    if (days > 30) return "var(--danger)";
    if (days > 14) return "var(--warning)";
    return "var(--ink-faint)";
  }

  let colour = $derived(actionColour(project.action));

  const axes: { key: "velocity" | "fit_signal" | "distinctness" | "leverage"; label: string }[] = [
    { key: "velocity", label: "V" },
    { key: "fit_signal", label: "F" },
    { key: "distinctness", label: "D" },
    { key: "leverage", label: "L" },
  ];
</script>

<div
  class="project-card"
  onclick={() => (window.location.hash = `#/project/${project.id}`)}
  role="button"
  tabindex="0"
  onkeydown={(e) => { if (e.key === "Enter") window.location.hash = `#/project/${project.id}`; }}
>
  <div class="card-header">
    <ProjectIcon name={project.name} size={36} />
    <span class="card-name">{project.name}</span>
    <span class="action-pill" style="color: {colour}">{project.action}</span>
  </div>

  <div class="score-bar-wrap">
    <div class="score-bar">
      <div
        class="score-bar-fill"
        style="width: {project.score}%; background: {colour}"
      ></div>
    </div>
    <span class="score-num" style="color: {colour}">{project.score}</span>
  </div>

  <div class="axes-mini">
    {#each axes as ax}
      <div class="axis-mini">
        <span class="axis-mini-label">{ax.label}</span>
        {#if project[ax.key] != null}
          <div class="axis-mini-bar">
            <div
              class="axis-mini-bar-fill"
              style="width: {(project[ax.key]! / 10) * 100}%"
            ></div>
          </div>
        {:else}
          <div class="axis-mini-bar missing"></div>
        {/if}
      </div>
    {/each}
  </div>

  <div class="card-footer">
    <span>{project.stage_label}</span>
    <span style="color: {staleColour(project.days_stale)}">{project.days_stale}d stale</span>
  </div>
</div>
