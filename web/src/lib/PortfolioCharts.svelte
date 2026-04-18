<script lang="ts">
  import {
    Chart,
    BarController,
    CategoryScale,
    LinearScale,
    BarElement,
    DoughnutController,
    ArcElement,
    Tooltip,
  } from "chart.js";
  import type { PortfolioStats } from "./types";

  Chart.register(BarController, CategoryScale, LinearScale, BarElement, DoughnutController, ArcElement, Tooltip);

  let { stats }: { stats: PortfolioStats } = $props();

  let stageCanvas: HTMLCanvasElement;
  let actionCanvas: HTMLCanvasElement;

  const actionColourMap: Record<string, string> = {
    PUSH: "#1c7a3f",
    GROOM: "#1e3a5f",
    OBSERVE: "#999",
    KILL: "#b14532",
    PIVOT: "#b8860b",
    SUSTAIN: "#5b9bd5",
    INTEGRATE: "#9b59b6",
    REPURPOSE: "#d4881c",
  };

  $effect(() => {
    const stageLabels = stats.by_stage.map((s) => s.label);
    const stageCounts = stats.by_stage.map((s) => s.count);

    const stageChart = new Chart(stageCanvas, {
      type: "bar",
      data: {
        labels: stageLabels,
        datasets: [
          {
            data: stageCounts,
            backgroundColor: "#1e3a5f",
            borderWidth: 0,
            borderRadius: 2,
            barPercentage: 0.7,
          },
        ],
      },
      options: {
        indexAxis: "y",
        responsive: true,
        maintainAspectRatio: false,
        animation: false,
        scales: {
          x: {
            beginAtZero: true,
            ticks: {
              stepSize: 1,
              font: { size: 10, family: "'Iosevka Term', 'Cascadia Code', 'SF Mono', monospace" },
              color: "#999",
            },
            grid: { color: "#e8e8e8" },
          },
          y: {
            ticks: {
              font: { size: 11, family: "system-ui, -apple-system, sans-serif" },
              color: "#555",
            },
            grid: { display: false },
          },
        },
        plugins: {
          legend: { display: false },
          tooltip: {
            backgroundColor: "#1a1a1a",
            titleFont: { size: 11 },
            bodyFont: { size: 11 },
          },
        },
      },
    });

    const actionLabels = stats.by_action.map((a) => a.action);
    const actionCounts = stats.by_action.map((a) => a.count);
    const actionColours = actionLabels.map((a) => actionColourMap[a] ?? "#999");

    const actionChart = new Chart(actionCanvas, {
      type: "doughnut",
      data: {
        labels: actionLabels,
        datasets: [
          {
            data: actionCounts,
            backgroundColor: actionColours,
            borderWidth: 1,
            borderColor: "#fff",
          },
        ],
      },
      options: {
        responsive: true,
        maintainAspectRatio: false,
        animation: false,
        cutout: "55%",
        plugins: {
          legend: {
            position: "bottom",
            labels: {
              boxWidth: 10,
              padding: 12,
              font: { size: 11, family: "system-ui, -apple-system, sans-serif" },
              color: "#555",
            },
          },
          tooltip: {
            backgroundColor: "#1a1a1a",
            titleFont: { size: 11 },
            bodyFont: { size: 11 },
          },
        },
      },
    });

    return () => {
      stageChart.destroy();
      actionChart.destroy();
    };
  });
</script>

<div class="charts-row">
  <div class="chart-card">
    <h3>Stage Funnel</h3>
    <div style="height: 260px">
      <canvas bind:this={stageCanvas}></canvas>
    </div>
  </div>
  <div class="chart-card">
    <h3>Action Distribution</h3>
    <div style="height: 260px">
      <canvas bind:this={actionCanvas}></canvas>
    </div>
  </div>
</div>
