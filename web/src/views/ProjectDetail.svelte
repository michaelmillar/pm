<script lang="ts">
  import { fetchProject } from "../lib/api";
  import type { ProjectDetail as PD } from "../lib/types";
  import ProjectIcon from "../lib/ProjectIcon.svelte";

  let { id }: { id: number } = $props();
  let detail: PD | null = $state(null);
  let loading = $state(true);

  $effect(() => {
    loading = true;
    fetchProject(id).then((data) => {
      detail = data;
      loading = false;
    });
  });

  function statusClass(s: string): string {
    if (s === "pass") return "status-pass";
    if (s === "fail") return "status-fail";
    return "status-pending";
  }
</script>

<a href="#/" class="back-link">&larr; Dashboard</a>

{#if loading}
  <div class="loading">Loading...</div>
{:else if detail}
  <div class="detail-header">
    <ProjectIcon name={detail.name} size={32} />
    <h2>{detail.name}</h2>
    <span class="state-badge">{detail.state}</span>
    {#if detail.soft_deadline}
      <span class="eyebrow">deadline {detail.soft_deadline}</span>
    {/if}
  </div>

  <div class="axes-row">
    <div class="axis">
      <span class="axis-value">{detail.priority_score}</span>
      <span class="axis-label">Score</span>
    </div>
    <div class="axis">
      <span class="axis-value">{detail.readiness}%</span>
      <span class="axis-label">Readiness</span>
    </div>
    <div class="axis">
      <span class="axis-value">{detail.impact}</span>
      <span class="axis-label">Impact</span>
    </div>
    <div class="axis">
      <span class="axis-value">{detail.monetization}</span>
      <span class="axis-label">Monetisation</span>
    </div>
    <div class="axis">
      <span class="axis-value">{detail.defensibility}</span>
      <span class="axis-label">Defensibility</span>
    </div>
    <div class="axis">
      <span class="axis-value" style="color: {detail.days_stale > 30 ? 'var(--danger)' : detail.days_stale > 14 ? 'var(--warning)' : 'var(--accent)'}">{detail.days_stale}d</span>
      <span class="axis-label">Stale</span>
    </div>
  </div>

  <div class="card-grid">
    {#if detail.roadmap}
      <div class="card">
        <p class="eyebrow">Roadmap</p>
        <h3>{detail.roadmap.readiness}% ready</h3>
        {#if !detail.roadmap.weight_valid}
          <p class="flag-warning">Phase weights do not sum to 1.0.</p>
        {/if}
        {#each detail.roadmap.phases as phase}
          <div class="phase">
            <div class="phase-head">
              <span class="phase-label">{phase.label}</span>
              <span class="phase-meta">{Math.round(phase.progress * 100)}% &middot; w{phase.weight}</span>
            </div>
            <div class="phase-bar">
              <div class="phase-bar-fill" style="width: {phase.progress * 100}%"></div>
            </div>
            <ul class="task-list">
              {#each phase.tasks as task}
                <li class:done={task.done}>
                  <span class="task-check">{task.done ? "\u2713" : "\u2717"}</span>
                  {task.label}
                </li>
              {/each}
            </ul>
          </div>
        {/each}
      </div>
    {/if}

    {#if detail.dod}
      <div class="card">
        <p class="eyebrow">Definition of Done</p>
        <h3>{detail.dod.complete}/{detail.dod.total} criteria met</h3>
        <p style="font-size: 0.88rem; color: var(--ink-soft); margin: 0 0 0.5rem">USP: {detail.dod.usp}</p>
        {#each detail.dod.criteria as c}
          <div class="criterion">
            <div class="criterion-head">{c.description}</div>
            <div class="criterion-scenario">{c.scenario}</div>
            <div class="criterion-statuses">
              <span class={statusClass(c.automated)}>auto: {c.automated}</span>
              <span class={statusClass(c.human)}>human: {c.human}</span>
            </div>
          </div>
        {/each}
      </div>
    {/if}

    {#if detail.research}
      <div class="card">
        <p class="eyebrow">Research</p>
        {#if detail.research.researched_at}
          <p style="font-size: 0.8rem; color: var(--ink-faint); margin: 0 0 0.5rem">
            Last researched {detail.research.researched_at.slice(0, 10)}
          </p>
        {/if}
        {#if detail.research.consecutive_flags >= 2}
          <div class="flag-warning">Research recommends re-evaluating this project.</div>
        {/if}
        <div class="research-summary">{detail.research.summary}</div>
        {#if detail.research.previous}
          <details style="margin-top: 0.75rem">
            <summary style="font-size: 0.85rem; color: var(--ink-soft); cursor: pointer">Previous research</summary>
            <div class="research-summary" style="margin-top: 0.5rem; opacity: 0.7">{detail.research.previous}</div>
          </details>
        {/if}
      </div>
    {/if}

    {#if detail.tasks.length > 0}
      <div class="card">
        <p class="eyebrow">Tasks</p>
        <h3>{detail.tasks.filter((t) => t.source === "pending").length} pending</h3>
        <ul class="task-list">
          {#each detail.tasks as task}
            <li class:done={task.source !== "pending"}>
              <span class="task-check">{task.source === "pending" ? "\u2717" : "\u2713"}</span>
              <span style="font-family: monospace; font-size: 0.78rem; color: var(--ink-faint)">{task.plan_file}#{task.task_number}</span>
              {task.description}
            </li>
          {/each}
        </ul>
      </div>
    {/if}

    {#if detail.roadmap?.assessment}
      <div class="card">
        <p class="eyebrow">Assessment</p>
        {#if detail.roadmap.assessment.stale}
          <div class="flag-warning">Assessment is older than 90 days. Consider re-researching.</div>
        {/if}
        {#if detail.roadmap.assessment.reasoning}
          <div class="research-summary">{detail.roadmap.assessment.reasoning}</div>
        {/if}
        {#if detail.roadmap.assessment.signals?.length}
          <p class="eyebrow" style="margin-top: 0.75rem">Signals</p>
          <ul class="task-list">
            {#each detail.roadmap.assessment.signals as signal}
              <li>{signal}</li>
            {/each}
          </ul>
        {/if}
      </div>
    {/if}
  </div>

  {#if detail.inbox_note}
    <div class="card">
      <p class="eyebrow">Inbox note</p>
      <div class="research-summary">{detail.inbox_note}</div>
    </div>
  {/if}

  <p style="font-size: 0.8rem; color: var(--ink-faint); margin-top: 1rem">
    Created {detail.created_at} &middot; Last activity {detail.last_activity}
    {#if detail.path}
      &middot; <code style="font-size: 0.78rem">{detail.path}</code>
    {/if}
  </p>
{:else}
  <div class="empty-state">Project not found.</div>
{/if}
