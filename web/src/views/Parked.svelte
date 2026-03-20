<script lang="ts">
  import { fetchParked } from "../lib/api";
  import type { Project } from "../lib/types";

  let projects: Project[] = $state([]);
  let loading = $state(true);

  $effect(() => {
    fetchParked().then((data) => {
      projects = data;
      loading = false;
    });
  });
</script>

<p class="eyebrow">Parked projects</p>

{#if loading}
  <div class="loading">Loading...</div>
{:else if projects.length === 0}
  <div class="empty-state">No parked projects.</div>
{:else}
  <table>
    <thead>
      <tr>
        <th>ID</th>
        <th>Project</th>
        <th>Readiness</th>
        <th>Score</th>
        <th>Last activity</th>
      </tr>
    </thead>
    <tbody>
      {#each projects as p}
        <tr onclick={() => (window.location.hash = `#/project/${p.id}`)}>
          <td class="num">{p.id}</td>
          <td class="name-cell">{p.name}</td>
          <td class="num">{p.readiness}%</td>
          <td class="num">{p.priority_score}</td>
          <td style="font-size: 0.85rem; color: var(--ink-soft)">{p.last_activity}</td>
        </tr>
      {/each}
    </tbody>
  </table>
{/if}
