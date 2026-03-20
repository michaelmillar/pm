<script lang="ts">
  import { fetchInbox } from "../lib/api";
  import type { Project } from "../lib/types";

  let projects: Project[] = $state([]);
  let loading = $state(true);

  $effect(() => {
    fetchInbox().then((data) => {
      projects = data;
      loading = false;
    });
  });
</script>

<p class="eyebrow">Inbox</p>

{#if loading}
  <div class="loading">Loading...</div>
{:else if projects.length === 0}
  <div class="empty-state">Inbox is empty. Discover projects with <code>pm scan</code>.</div>
{:else}
  <table>
    <thead>
      <tr>
        <th>ID</th>
        <th>Project</th>
        <th>Created</th>
        <th>Impact</th>
        <th>Monet</th>
        <th>Path</th>
      </tr>
    </thead>
    <tbody>
      {#each projects as p}
        <tr onclick={() => (window.location.hash = `#/project/${p.id}`)}>
          <td class="num">{p.id}</td>
          <td class="name-cell">{p.name}</td>
          <td style="font-size: 0.85rem; color: var(--ink-soft)">{p.created_at}</td>
          <td class="num">{p.impact}</td>
          <td class="num">{p.monetization}</td>
          <td style="font-size: 0.8rem; color: var(--ink-faint)">{p.path ?? "\u2014"}</td>
        </tr>
      {/each}
    </tbody>
  </table>
{/if}
