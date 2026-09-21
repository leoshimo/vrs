// N16's selected Scan drift: local displacement, retained ink, and Bayer grain.
// The static image stays visible until the first frame has been painted.
export function inkEffect(host) {
  const image = host.querySelector('img');
  const bayer2 = (x, y) => 2 * (x % 2) + 3 * (y % 2) - 4 * (x % 2) * (y % 2);
  const rank = (x, y) => (16 * bayer2(x, y) + 4 * bayer2(x >> 1, y >> 1) + bayer2(x >> 2, y >> 2) + .5) / 64;
  let item, frame = 0, time = 0, last = 0, paused = true, visible = true, loopDuration = 0;
  let canvas;

  function phase(speed) {
    if (!loopDuration) return time * speed;
    const cycles = Math.max(1, Math.round(speed * loopDuration / (2 * Math.PI)));
    return time * cycles * 2 * Math.PI / loopDuration;
  }

  function prepare() {
    const box = image.getBoundingClientRect(), parent = host.getBoundingClientRect();
    const scale = Math.min(2, devicePixelRatio || 1), pad = 8;
    const width = Math.round((box.width + pad * 2) * scale);
    const height = Math.round((box.height + 12) * scale);
    const source = document.createElement('canvas');
    source.width = width; source.height = height;
    const context = source.getContext('2d', {willReadFrequently: true});
    context.drawImage(image, pad * scale, 6 * scale, box.width * scale, box.height * scale);
    const raw = context.getImageData(0, 0, width, height).data;
    const alpha = new Float32Array(width * height), grain = new Float32Array(width * height);
    for (let y = 0; y < height; y++) for (let x = 0; x < width; x++) {
      const n = y * width + x, i = n * 4;
      alpha[n] = raw[i + 3] / 255 * (1 - (raw[i] * .2126 + raw[i + 1] * .7152 + raw[i + 2] * .0722) / 255);
      grain[n] = Math.max(0, (rank(Math.floor(x / scale), Math.floor(y / scale)) - .76) / .24);
    }
    if (!canvas) {
      canvas = document.createElement('canvas'); canvas.className = 'ink-motion';
      canvas.setAttribute('aria-hidden', 'true'); host.append(canvas);
    }
    canvas.width = width; canvas.height = height;
    Object.assign(canvas.style, {width: (box.width + pad * 2) + 'px', height: (box.height + 12) + 'px',
      left: (box.left - parent.left - pad) + 'px', top: (box.top - parent.top - 6) + 'px'});
    const output = canvas.getContext('2d');
    item = {width, height, scale, alpha, grain, output, pixels: output.createImageData(width, height), strength: box.width / 290};
    paint();
  }

  function paint() {
    if (!item) return;
    const {width, height, scale, alpha, grain, output, pixels, strength} = item;
    const center = .5 + .5 * Math.sin(phase(.45));
    for (let y = 0; y < height; y++) {
      const scan = Math.exp(-Math.pow((y / height - center) / .085, 2));
      for (let x = 0; x < width; x++) {
        const n = y * width + x;
        const wave = Math.pow((1 + Math.sin(x / width * Math.PI * 2 - phase(.95))) / 2, 4);
        const offset = wave * .65 * Math.sin(phase(.7)) + scan * 1.65 * Math.sin(phase(1.2));
        const erase = wave * .28 + scan * .24;
        const ghostOpacity = wave * .04 + scan * .12;
        const sx = x - offset * scale * strength, x0 = Math.floor(sx), x1 = x0 + 1, mix = sx - x0;
        const a = (x0 >= 0 && x0 < width ? alpha[y * width + x0] : 0) * (1 - mix)
          + (x1 >= 0 && x1 < width ? alpha[y * width + x1] : 0) * mix;
        const ink = a * (1 - grain[n] * erase), ghostX = x - Math.round(2.2 * scale * strength);
        const ghost = ghostX >= 0 && ghostX < width ? alpha[y * width + ghostX] * ghostOpacity : 0;
        pixels.data[n * 4 + 3] = Math.round(Math.min(1, ink + ghost * (1 - ink)) * 255);
      }
    }
    output.putImageData(pixels, 0, 0);
    host.dataset.inkReady = 'true'; canvas.dataset.phase = time.toFixed(3);
  }

  function tick(now) {
    frame = 0;
    if (now - last >= 50) { time += Math.min(.1, (now - last) / 1000 || .05); paint(); last = now; }
    if (!paused && visible && !document.hidden) frame = requestAnimationFrame(tick);
  }
  function wake() {
    cancelAnimationFrame(frame); frame = 0; last = performance.now();
    if (!paused && visible && !document.hidden && item) frame = requestAnimationFrame(tick);
  }
  const ready = image.decode().then(() => {
    prepare(); new ResizeObserver(prepare).observe(image);
    new IntersectionObserver(([entry]) => { visible = entry.isIntersecting; wake(); }).observe(host);
    wake();
  }).catch(() => { host.removeAttribute('data-ink-ready'); });
  document.addEventListener('visibilitychange', wake);
  return {
    ready,
    setPaused(value) { paused = value; wake(); },
    renderAt(seconds, duration) {
      paused = true; cancelAnimationFrame(frame); frame = 0;
      loopDuration = duration;
      time = seconds % duration;
      paint();
    }
  };
}
