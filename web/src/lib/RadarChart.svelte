<script lang="ts">
  import {
    Chart,
    RadarController,
    RadialLinearScale,
    PointElement,
    LineElement,
    Filler,
    Tooltip,
  } from "chart.js";

  Chart.register(RadarController, RadialLinearScale, PointElement, LineElement, Filler, Tooltip);

  let {
    labels,
    values,
    max = 10,
  }: { labels: string[]; values: number[]; max?: number } = $props();

  let canvas: HTMLCanvasElement;

  $effect(() => {
    const data = values;
    const lbls = labels;
    const mx = max;

    const chart = new Chart(canvas, {
      type: "radar",
      data: {
        labels: lbls,
        datasets: [
          {
            data,
            borderColor: "#1e3a5f",
            backgroundColor: "rgba(30, 58, 95, 0.08)",
            borderWidth: 1.5,
            pointBackgroundColor: "#1e3a5f",
            pointBorderColor: "#1e3a5f",
            pointRadius: 3,
            pointHoverRadius: 5,
            fill: true,
          },
        ],
      },
      options: {
        responsive: true,
        maintainAspectRatio: true,
        animation: false,
        scales: {
          r: {
            min: 0,
            max: mx,
            ticks: {
              stepSize: 2,
              backdropColor: "transparent",
              font: { size: 9 },
              color: "#999",
            },
            grid: { color: "#d0d0d0" },
            angleLines: { color: "#e0e0e0" },
            pointLabels: {
              font: { size: 11, family: "'STIX Two Text', Georgia, serif" },
              color: "#333",
            },
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

    return () => chart.destroy();
  });
</script>

<canvas bind:this={canvas}></canvas>
