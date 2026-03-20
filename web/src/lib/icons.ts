// Deterministic project icon system.
// Each project gets a stable colour derived from its name and a two-letter monogram.

const PALETTE = [
  "#1e3a5f", // navy
  "#2c8a57", // forest
  "#8b5cf6", // violet
  "#b45309", // amber
  "#0f766e", // teal
  "#be185d", // rose
  "#4338ca", // indigo
  "#0369a1", // sky
  "#7c3aed", // purple
  "#c2410c", // burnt orange
  "#047857", // emerald
  "#a21caf", // fuchsia
  "#1d4ed8", // blue
  "#b91c1c", // red
];

function hashName(name: string): number {
  let h = 0;
  for (let i = 0; i < name.length; i++) {
    h = ((h << 5) - h + name.charCodeAt(i)) | 0;
  }
  return Math.abs(h);
}

export function projectColour(name: string): string {
  return PALETTE[hashName(name) % PALETTE.length];
}

export function projectMonogram(name: string): string {
  const parts = name.split(/[-_ ]+/).filter(Boolean);
  if (parts.length >= 2) {
    return (parts[0][0] + parts[1][0]).toUpperCase();
  }
  return name.slice(0, 2).toUpperCase();
}

export function projectIconSvg(name: string, size = 28): string {
  const colour = projectColour(name);
  const mono = projectMonogram(name);
  const fontSize = Math.round(size * 0.42);
  const textY = Math.round(size * 0.62);
  const r = Math.round(size / 2);
  return [
    `<svg xmlns="http://www.w3.org/2000/svg" width="${size}" height="${size}" viewBox="0 0 ${size} ${size}">`,
    `<rect width="${size}" height="${size}" rx="${Math.round(size * 0.22)}" fill="${colour}"/>`,
    `<text x="${r}" y="${textY}" font-family="system-ui,-apple-system,sans-serif" font-size="${fontSize}" font-weight="600" fill="#fff" text-anchor="middle">${mono}</text>`,
    `</svg>`,
  ].join("");
}

// Pre-built data URI for use in <img> tags
export function projectIconDataUri(name: string, size = 28): string {
  return `data:image/svg+xml,${encodeURIComponent(projectIconSvg(name, size))}`;
}
