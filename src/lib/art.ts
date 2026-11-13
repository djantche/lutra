// Procedural key art: a seeded landscape (sky, star field, sun or planet, ridges)
// used as placeholder artwork until SteamGridDB images are wired in.

export const ART_WIDTH = 1600;
export const ART_HEIGHT = 1000;

export interface ArtScene {
  sky: [string, string, string];
  sun: { cx: number; cy: number; r: number; color: string; colorLow: string };
  ring: boolean;
  stars: { x: number; y: number; r: number; opacity: number }[];
  ridges: { d: string; fill: string; base: number }[];
}

function hash(text: string): number {
  let h = 2166136261;
  for (let i = 0; i < text.length; i++) {
    h ^= text.charCodeAt(i);
    h = Math.imul(h, 16777619);
  }
  return h >>> 0;
}

function mulberry32(seed: number): () => number {
  let state = seed;
  return () => {
    state = (state + 0x6d2b79f5) | 0;
    let t = Math.imul(state ^ (state >>> 15), 1 | state);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

function ridgePath(rand: () => number, base: number, amplitude: number, peaks: number): string {
  const step = ART_WIDTH / peaks;
  let d = `M0 ${ART_HEIGHT} L0 ${Math.round(base - rand() * amplitude * 0.5)}`;
  for (let i = 0; i < peaks; i++) {
    const peakX = step * i + step * (0.3 + rand() * 0.4);
    const peakY = base - amplitude * (0.45 + rand() * 0.55);
    const valleyY = base - amplitude * rand() * 0.35;
    d += ` L${Math.round(peakX)} ${Math.round(peakY)} L${Math.round(step * (i + 1))} ${Math.round(valleyY)}`;
  }
  return `${d} L${ART_WIDTH} ${ART_HEIGHT} Z`;
}

export function createScene(seed: string, hue: number): ArtScene {
  const rand = mulberry32(hash(seed));
  const skyHue = (hue + 20 + rand() * 40) % 360;
  const glowHue = (hue + 25) % 360;
  const sunHue = (hue + 35 + rand() * 30) % 360;

  return {
    sky: [`hsl(${skyHue} 55% 8%)`, `hsl(${hue} 60% 26%)`, `hsl(${glowHue} 85% 60%)`],
    sun: {
      cx: 560 + rand() * 480,
      cy: 260 + rand() * 180,
      r: 110 + rand() * 90,
      color: `hsl(${sunHue} 95% 74%)`,
      colorLow: `hsl(${(sunHue + 25) % 360} 90% 55%)`,
    },
    ring: rand() > 0.6,
    stars: Array.from({ length: 60 }, () => ({
      x: rand() * ART_WIDTH,
      y: rand() * ART_HEIGHT * 0.55,
      r: 0.8 + rand() * 2.2,
      opacity: 0.2 + rand() * 0.7,
    })),
    ridges: [
      { base: 640, amplitude: 260, peaks: 6, light: 24 },
      { base: 750, amplitude: 180, peaks: 9, light: 14 },
      { base: 870, amplitude: 120, peaks: 12, light: 6 },
    ].map((layer) => ({
      d: ridgePath(rand, layer.base, layer.amplitude, layer.peaks),
      fill: `hsl(${hue} 45% ${layer.light}%)`,
      base: layer.base,
    })),
  };
}
