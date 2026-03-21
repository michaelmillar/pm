<script lang="ts">
  import { fetchProjects } from "../lib/api";
  import type { Project } from "../lib/types";
  import ProjectIcon from "../lib/ProjectIcon.svelte";

  let projects: Project[] = $state([]);
  let loading = $state(true);
  let sortKey: keyof Project = $state("priority_score");
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

  function readinessSegments(pct: number): boolean[] {
    const filled = Math.round(pct / 10);
    return Array.from({ length: 10 }, (_, i) => i < filled);
  }

  function scoreColour(score: number): string {
    if (score >= 50) return "var(--success)";
    if (score >= 30) return "var(--accent)";
    if (score >= 15) return "var(--warning)";
    return "var(--danger)";
  }

  function staleColour(days: number): string {
    if (days > 30) return "var(--danger)";
    if (days > 14) return "var(--warning)";
    return "var(--ink-soft)";
  }

  type SortableKey = keyof Project;
  const columns: { key: SortableKey; label: string; numeric: boolean }[] = [
    { key: "name", label: "Project", numeric: false },
    { key: "readiness", label: "Ready", numeric: true },
    { key: "days_stale", label: "Stale", numeric: true },
    { key: "priority_score", label: "Score", numeric: true },
    { key: "impact", label: "Impact", numeric: true },
    { key: "monetization", label: "Monet", numeric: true },
    { key: "defensibility", label: "Def", numeric: true },
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
        <th>Next milestone</th>
      </tr>
    </thead>
    <tbody>
      {#each sorted as p}
        <tr onclick={() => (window.location.hash = `#/project/${p.id}`)}>
          <td class="name-cell"><ProjectIcon name={p.name} size={22} /> {p.name}</td>
          <td>
            <span class="readiness-bar">
              {#each readinessSegments(p.readiness) as filled}
                <span class="segment" class:filled></span>
              {/each}
            </span>
            <span class="readiness-pct">{p.readiness}%</span>
          </td>
          <td class="num" style="color: {staleColour(p.days_stale)}">{p.days_stale}d</td>
          <td class="num">
            <span class="score-pill" style="color: {scoreColour(p.priority_score)}">
              {p.priority_score}
            </span>
          </td>
          <td class="num">{p.impact}</td>
          <td class="num">{p.monetization}</td>
          <td class="num">{p.defensibility}</td>
          <td style="color: var(--ink-soft); font-size: 0.85rem">{p.next_milestone ?? "\u2014"}</td>
        </tr>
      {/each}
    </tbody>
  </table>
{/if}
