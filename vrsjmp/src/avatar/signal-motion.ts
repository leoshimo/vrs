export type MotionMode = "idle" | "typing" | "loading" | "dispatch";
type Spring = { value: number; velocity: number };
export type SignalMotion = {
  momentum: Spring;
  typing: Spring;
  loading: Spring;
  dispatch: Spring;
  speed: Spring;
  tap: Spring;
  burst: Spring;
  echo: Spring;
  pokeX: Spring;
  pokeY: Spring;
  poke2X: Spring;
  poke2Y: Spring;
  seed: number;
  variation: number;
  wake: Spring;
  invocation: number;
  transport: number;
  fieldSpeed: Spring;
  drift: number;
  releaseAge: number;
  releasePhase: number;
  priorReleaseAge: number;
  priorReleasePhase: number;
  mode: MotionMode;
  phase: number;
  flickVelocity: number;
  fieldImpulseVelocity: number;
  pulse: number;
};
export function createSignalMotion(seed = Math.random() * 0x100000000): SignalMotion {
  return {
    momentum: { value: 0, velocity: 0 },
    typing: { value: 0, velocity: 0 },
    loading: { value: 0, velocity: 0 },
    dispatch: { value: 0, velocity: 0 },
    speed: { value: 0, velocity: 0 },
    tap: { value: 0, velocity: 0 },
    burst: { value: 0, velocity: 0 },
    echo: { value: 0, velocity: 0 },
    pokeX: { value: 0, velocity: 0 },
    pokeY: { value: 0, velocity: 0 },
    poke2X: { value: 0, velocity: 0 },
    poke2Y: { value: 0, velocity: 0 },
    seed: seed >>> 0,
    variation: (seed >>> 0) / 0x100000000,
    wake: { value: 0, velocity: 0 },
    invocation: 0,
    transport: 0,
    fieldSpeed: { value: 0.38, velocity: 0 },
    drift: 0,
    releaseAge: 10,
    releasePhase: 0,
    priorReleaseAge: 10,
    priorReleasePhase: 0,
    mode: "idle",
    phase: 0,
    flickVelocity: 0,
    fieldImpulseVelocity: 0,
    pulse: 0,
  };
}
// Sample once per gesture, never per frame: randomness changes intent, not stability.
function random(m: SignalMotion) {
  m.seed = (Math.imul(m.seed, 1664525) + 1013904223) >>> 0;
  return m.seed / 0x100000000;
}
// Exact critically damped step: changing a target preserves position AND velocity.
function spring(s: Spring, target: number, dt: number, frequency: number) {
  const error = s.value - target,
    c = s.velocity + frequency * error,
    decay = Math.exp(-frequency * dt);
  s.value = target + (error + c * dt) * decay;
  s.velocity = (s.velocity - frequency * c * dt) * decay;
}
export function advanceSignalMotion(
  m: SignalMotion,
  mode: MotionMode,
  pulse: number,
  idle: boolean,
  dt: number,
  animate = true,
  activity: {
    invocation?: number;
    working?: boolean;
    loading?: "normal" | "quicker" | "agitated" | "drift";
    idleAmount?: number;
    loadingRate?: number;
    typingEnergy?: number;
    typingNudge?: boolean;
    dispatchEnergy?: number;
    settle?: number;
  } = {},
) {
  // Existing refs can survive a development hot update.
  m.fieldImpulseVelocity ??= 0;
  m.momentum ??= { value: 0, velocity: 0 };
  const targets = {
    // Typing is an event, not a sustained animation mode.
    typing: 0,
    loading: (activity.working ?? mode === "loading") ? 1 : 0,
    dispatch: mode === "dispatch" ? 1 : 0,
  };
  if (!animate) {
    for (const k of ["typing", "loading", "dispatch"] as const) {
      m[k].value = targets[k];
      m[k].velocity = 0;
    }
    m.phase = 0;
    m.drift = 0;
    m.releaseAge = 10;
    m.priorReleaseAge = 10;
    m.flickVelocity = 0;
    m.fieldImpulseVelocity = 0;
    m.speed.value = 0;
    m.speed.velocity = 0;
    for (const k of [
      "momentum",
      "tap",
      "burst",
      "echo",
      "pokeX",
      "pokeY",
      "poke2X",
      "poke2Y",
      "wake",
    ] as const) {
      m[k].value = 0;
      m[k].velocity = 0;
    }
    m.fieldSpeed.value = 0;
    m.fieldSpeed.velocity = 0;
    m.transport = 0;
    m.invocation = activity.invocation || 0;
    m.mode = mode;
    m.pulse = pulse;
    return m;
  }
  const step = Math.max(0, Math.min(0.064, dt));
  // A zero-time sample cannot consume an event or change the current pose.
  if (!step) return m;
  const typingEnergy = Math.max(0.1, Math.min(2.5, activity.typingEnergy ?? 1));
  const dispatchEnergy = Math.max(0.3, Math.min(2.5, activity.dispatchEnergy ?? 1));
  const settle = Math.max(0.5, Math.min(3, activity.settle ?? 1.5));
  if ((activity.invocation || 0) !== m.invocation) {
    m.invocation = activity.invocation || 0;
    m.wake.velocity = Math.min(5, m.wake.velocity + 3.6);
    m.momentum.velocity = Math.min(7, m.momentum.velocity + 3.2);
  }
  spring(m.wake, 0, step, 2.4 / Math.sqrt(settle));
  m.releaseAge += step;
  m.priorReleaseAge += step;
  if (pulse !== m.pulse || mode !== m.mode) {
    if (mode === "typing" && activity.typingNudge) {
      m.fieldImpulseVelocity = Math.min(3, m.fieldImpulseVelocity + 2.2 * typingEnergy);
    }
    if (mode === "typing" && !activity.typingNudge) {
      const angle = random(m) * Math.PI * 2;
      const strength = (0.85 + random(m) * 0.3) * typingEnergy;
      const compliance = 1 / (1 + Math.pow(Math.hypot(m.pokeX.value, m.pokeY.value) / 0.45, 4));
      m.pokeX.velocity += Math.cos(angle) * 16 * strength * compliance;
      m.pokeY.velocity += Math.sin(angle) * 16 * strength * compliance;
      const second = angle + 1.1 + random(m) * 3.8;
      const resistance = 1 / (1 + Math.pow(Math.hypot(m.poke2X.value, m.poke2Y.value) / 0.45, 4));
      m.poke2X.velocity = Math.max(
        -22,
        Math.min(22, m.poke2X.velocity + Math.cos(second) * 13 * resistance),
      );
      m.poke2Y.velocity = Math.max(
        -22,
        Math.min(22, m.poke2Y.velocity + Math.sin(second) * 13 * resistance),
      );
      const velocity = Math.hypot(m.pokeX.velocity, m.pokeY.velocity);
      if (velocity > 24) {
        m.pokeX.velocity *= 24 / velocity;
        m.pokeY.velocity *= 24 / velocity;
      }
      m.tap.velocity = Math.min(26, m.tap.velocity + 22 * typingEnergy);
      m.typing.velocity = Math.min(18, m.typing.velocity + 12);
      // A physical flick changes angular momentum immediately, never the pose.
      m.flickVelocity = Math.min(9, m.flickVelocity + 4.8 * strength);
    }
    if (mode === "dispatch") {
      m.burst.velocity = Math.min(60, m.burst.velocity + 29 * dispatchEnergy);
      m.momentum.velocity = Math.min(9, m.momentum.velocity + 5 * dispatchEnergy);
      m.priorReleaseAge = m.releaseAge;
      m.priorReleasePhase = m.releasePhase;
      m.releaseAge = 0;
      m.releasePhase = m.drift;
    }
  }
  m.pulse = pulse;
  m.mode = mode;
  spring(m.momentum, 0, step, 3 / settle);
  spring(m.tap, 0, step, 17);
  spring(m.burst, 0, step, 10);
  spring(m.echo, m.burst.value, step, 9);
  spring(m.pokeX, 0, step, 13);
  spring(m.pokeY, 0, step, 13);
  spring(m.poke2X, 0, step, 11);
  spring(m.poke2Y, 0, step, 11);
  spring(m.typing, targets.typing, step, 13);
  spring(m.loading, targets.loading, step, 5);
  spring(m.dispatch, targets.dispatch, step, 9);
  const speed = (idle ? 0.18 : 0) + m.loading.value * 0.12 + m.burst.value * 5;
  spring(m.speed, speed * (1 - 0.35 * m.dispatch.value), step, 9);
  // Exact drag integral: sharp attack, a smooth coast, then ordinary idle.
  const drag = Math.exp(-7 * step);
  m.phase += m.speed.value * step + (m.flickVelocity * (1 - drag)) / 7;
  m.flickVelocity *= drag;
  // Illumination drifts independently; impulses displace its field without spinning it.
  const idleSpeed = idle ? 0.38 * (0.8 + (activity.idleAmount ?? 1) * 0.2) : 0;
  const loadSpeed =
    activity.loading === "quicker" ? 1.7 : activity.loading === "agitated" ? 0.46 : 0.28;
  spring(
    m.fieldSpeed,
    idleSpeed +
      m.loading.value * loadSpeed * (activity.loadingRate ?? 1) +
      m.wake.value * 1.15 +
      m.momentum.value * 0.65,
    step,
    4,
  );
  const fieldDrag = Math.exp(-12 * step);
  m.drift += m.fieldSpeed.value * step + (m.fieldImpulseVelocity * (1 - fieldDrag)) / 12;
  m.fieldImpulseVelocity *= fieldDrag;
  if (activity.loading === "drift")
    m.transport += m.loading.value * step * 0.9 * (activity.loadingRate ?? 1);
  return m;
}
