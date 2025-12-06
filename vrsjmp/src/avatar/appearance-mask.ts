// Cached SVG masks for DOM surfaces. The avatar keeps its own GLSL reveal.
const cache = new Map<string, string>();
const bit = (x: number, y: number) => 2 * (x % 2) + 3 * (y % 2) - 4 * (x % 2) * (y % 2);
function bayer(x: number, y: number) {
  return (
    (16 * bit(x, y) +
      4 * bit(Math.floor(x / 2), Math.floor(y / 2)) +
      bit(Math.floor(x / 4), Math.floor(y / 4)) +
      0.5) /
    64
  );
}
export function appearanceMask(progress: number, mode: "fill" | "print") {
  if (progress >= 1) return "none";
  const frame = Math.max(0, Math.min(31, Math.floor(progress * 32)));
  const key = `${mode}:${frame}`;
  if (cache.has(key)) return cache.get(key)!;
  const width = mode === "fill" ? 8 : 256,
    height = mode === "fill" ? 8 : 24;
  const p = frame / 31;
  let path = "";
  for (let y = 0; y < height; y++) {
    let start = -1;
    for (let x = 0; x <= width; x++) {
      const coverage = mode === "fill" ? p : Math.max(0, Math.min(1, p * 2 - x / width));
      const filled = x < width && frame > 0 && bayer(x % 8, y % 8) < coverage;
      if (filled && start < 0) start = x;
      if (!filled && start >= 0) {
        path += `M${start} ${y}h${x - start}v1h${start - x}z`;
        start = -1;
      }
    }
  }
  const svg = `<svg xmlns="http://www.w3.org/2000/svg" width="${width}" height="${height}" viewBox="0 0 ${width} ${height}" preserveAspectRatio="none" shape-rendering="crispEdges"><path fill="white" d="${path}"/></svg>`;
  const mask = `url("data:image/svg+xml,${encodeURIComponent(svg)}")`;
  cache.set(key, mask);
  return mask;
}
