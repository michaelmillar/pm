<script lang="ts">
  import { fetchProjects } from "../lib/api";
  import type { Project } from "../lib/types";
  import ProjectIcon from "../lib/ProjectIcon.svelte";

  let projects: Project[] = $state([]);
  let loading = $state(true);
  let sortKey: keyof Project = $state("score");
  let sortAsc = $state(false);

  let sorted = $derived(
    [...projects].sort((a, b) => {
      const av = a[sortKey];
      const bv = b[sortKey];
      if (av == null && bv == null) return 0;
      if (av == null) return 1;
      if (bv == null) return -1;
      if (typeof av === "string" && typeof bv === "string") {
        return sortAsc ? av.localeCompare(bv) : bv.localeCompare(av);
      }
      return sortAsc ? Number(av) - Number(bv) : Number(bv) - Number(av);
    })
  );

  $effect(() => {
    fetchProjects().then((data) => {
      projects = data;
      loading = false;
    });
  });

  function toggleSort(key: keyof Project) {
    if (sortKey === key) {
      sortAsc = !sortAsc;
    } else {
      sortKey = key;
      sortAsc = key === "name";
    }
  }

  function sortIndicator(key: string): string {
    if (sortKey !== key) return "";
    return sortAsc ? " \u25B4" : " \u25BE";
  }

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
    return "var(--ink-soft)";
  }

  function axisDisplay(v: number | null): string {
    return v != null ? String(v) : "\u2014";
  }

  type SortableKey = keyof Project;
  const columns: { key: SortableKey; label: string; numeric: boolean }[] = [
    { key: "name", label: "Project", numeric: false },
    { key: "archetype", label: "Type", numeric: false },
    { key: "stage_label", label: "Stage", numeric: false },
    { key: "score", label: "Score", numeric: true },
    { key: "action", label: "Action", numeric: false },
    { key: "velocity", label: "V", numeric: true },
    { key: "fit_signal", label: "F", numeric: true },
    { key: "distinctness", label: "D", numeric: true },
    { key: "leverage", label: "L", numeric: true },
    { key: "days_stale", label: "Stale", numeric: true },
  ];
</script>

<p class="eyebrow">Active projects</p>

{#if loading}
  <div class="loading">Loading...</div>
{:else if projects.length === 0}
  <div class="empty-state">No active projects. Add one with <code>pm add</code>.</div>
{:else}
  <table>
    <thead>
      <tr>
        {#each columns as col}
          <th
            class:sorted={sortKey === col.key}
            onclick={() => toggleSort(col.key)}
          >
            {col.label}{sortIndicator(col.key)}
          </th>
        {/each}
      </tr>
    </thead>
    <tbody>
      {#each sorted as p}
        <tr onclick={() => (window.location.hash = `#/project/${p.id}`)}>
          <td class="name-cell">
            <ProjectIcon name={p.name} size={22} />
            <span class="name-col">
              <span class="project-name">{p.name}</span>
            </span>
          </td>
          <td><span class="type-badge">{p.archetype}</span></td>
          <td>{p.stage_label}</td>
          <td class="num">{p.score}</td>
          <td>
            <span class="action-pill" style="color: {actionColour(p.action)}">
              {p.action}
            </span>
            {#if p.action_target}
              <span class="action-target">&rarr; {p.action_target}</span>
            {/if}
          </td>
          <td class="num">{axisDisplay(p.velocity)}</td>
          <td class="num">{axisDisplay(p.fit_signal)}</td>
          <td class="num">{axisDisplay(p.distinctness)}</td>
          <td class="num">{axisDisplay(p.leverage)}</td>
          <td class="num" style="color: {staleColour(p.days_stale)}">{p.days_stale}d</td>
        </tr>
      {/each}
    </tbody>
  </table>
{/if}
