// A sampled signal holds the sum of all interactions. No per-interaction
// animation objects survive after their contribution has been written.
export const sampleRate = 120;
const capacity = 4096;
export class ResponseCurve {
  private values = new Float64Array(capacity);
  private stamps = new Float64Array(capacity).fill(-1);
  add(time: number, duration: number, amplitude = 1, attack = 0.15) {
    const first = Math.ceil(time * sampleRate);
    const count = Math.max(2, Math.ceil(duration * sampleRate));
    for (let offset = 0; offset <= count; offset++) {
      const tick = first + offset;
      const index = tick % capacity;
      if (this.stamps[index] !== tick) {
        this.stamps[index] = tick;
        this.values[index] = 0;
      }
      const p = offset / count;
      const rise = Math.min(1, p / Math.max(0.01, attack));
      const fall = Math.max(0, (1 - p) / Math.max(0.01, 1 - attack));
      this.values[index] +=
        amplitude * (rise * rise * (3 - 2 * rise)) * (fall * fall * (3 - 2 * fall));
    }
  }
  sample(time: number) {
    if (time < 0) return 0;
    const tick = time * sampleRate,
      first = Math.floor(tick),
      fraction = tick - first;
    const read = (n: number) => (this.stamps[n % capacity] === n ? this.values[n % capacity] : 0);
    return read(first) * (1 - fraction) + read(first + 1) * fraction;
  }
}

export type Momentum = { speed: number; phase: number };
export function advanceMomentum(value: Momentum, push: number, dt: number, drag = 8, limit = 2.4) {
  // dv/dt = push * (1 - v/limit) - drag*v. Its exact step is stable at any frame rate.
  const rate = drag + Math.max(0, push) / limit;
  const target = Math.max(0, push) / rate;
  const decay = Math.exp(-rate * dt);
  value.phase += target * dt + ((value.speed - target) * (1 - decay)) / rate;
  value.speed = target + (value.speed - target) * decay;
}
