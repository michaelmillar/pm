<script lang="ts">
  import Dashboard from "./views/Dashboard.svelte";
  import ProjectDetail from "./views/ProjectDetail.svelte";
  import Inbox from "./views/Inbox.svelte";
  import Next from "./views/Next.svelte";
  import Parked from "./views/Parked.svelte";

  let hash = $state(window.location.hash || "#/");

  $effect(() => {
    const handler = () => {
      hash = window.location.hash || "#/";
    };
    window.addEventListener("hashchange", handler);
    return () => window.removeEventListener("hashchange", handler);
  });

  let route = $derived(hash.slice(1));

  function navClass(path: string): string {
    if (path === "/" && (route === "/" || route === "")) return "active";
    if (path !== "/" && route.startsWith(path)) return "active";
    return "";
  }
</script>

<div class="page-shell">
  <header class="masthead">
    <h1>pm</h1>
    <nav>
      <a href="#/" class={navClass("/")}>Dashboard</a>
      <a href="#/inbox" class={navClass("/inbox")}>Inbox</a>
      <a href="#/parked" class={navClass("/parked")}>Parked</a>
      <a href="#/next" class={navClass("/next")}>Next</a>
    </nav>
  </header>

  {#if route === "/" || route === ""}
    <Dashboard />
  {:else if route.startsWith("/project/")}
    <ProjectDetail id={parseInt(route.slice(9))} />
  {:else if route === "/inbox"}
    <Inbox />
  {:else if route === "/parked"}
    <Parked />
  {:else if route === "/next"}
    <Next />
  {:else}
    <div class="empty-state">Not found.</div>
  {/if}
</div>
