const EMOJI_MAP: Record<string, string> = {};

const PALETTE = [
  "#1e3a5f", "#2c8a57", "#8b5cf6", "#b45309", "#0f766e",
  "#be185d", "#4338ca", "#0369a1", "#7c3aed", "#c2410c",
  "#047857", "#a21caf", "#1d4ed8", "#b91c1c",
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

export function projectEmoji(name: string): string | null {
  const key = name.toLowerCase().replace(/\s+/g, "-");
  return EMOJI_MAP[key] ?? null;
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
  const emoji = projectEmoji(name);
  const fontSize = Math.round(size * 0.42);
  const textY = Math.round(size * 0.62);
  const r = Math.round(size / 2);
  const content = emoji
    ? `<text x="${r}" y="${Math.round(size * 0.72)}" font-size="${Math.round(size * 0.55)}" text-anchor="middle">${emoji}</text>`
    : `<text x="${r}" y="${textY}" font-family="system-ui,-apple-system,sans-serif" font-size="${fontSize}" font-weight="600" fill="#fff" text-anchor="middle">${projectMonogram(name)}</text>`;
  return [
    `<svg xmlns="http://www.w3.org/2000/svg" width="${size}" height="${size}" viewBox="0 0 ${size} ${size}">`,
    emoji ? "" : `<rect width="${size}" height="${size}" rx="${Math.round(size * 0.22)}" fill="${colour}"/>`,
    content,
    `</svg>`,
  ].join("");
}

export function projectIconDataUri(name: string, size = 28): string {
  return `data:image/svg+xml,${encodeURIComponent(projectIconSvg(name, size))}`;
}
