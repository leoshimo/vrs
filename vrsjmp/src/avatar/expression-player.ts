import { parameters } from "./effect-settings";
import { ResponseCurve, advanceMomentum, type Momentum } from "./response-curve";
import {
  assigned,
  assignmentNames,
  type Assignment,
  type AvatarExpression,
  type AvatarSetup,
} from "./expression-model";
export type Uniforms = Record<string, number | number[]>;
export const historyLength = 128;
export const paletteNames = [
  "mono",
  "aurora",
  "ember",
  "lagoon",
  "oxide",
  "risograph",
  "field",
  "opal",
  "spectrum",
  "slate",
  "moss",
  "plum",
  "ochre",
  "mist",
  "dusk",
];
const waveKinds = ["ripple", "sparks", "echo", "bloom"];
export type PlaybackAction = Assignment | "hide";
type Drive = {
  motion: ResponseCurve;
  color: ResponseCurve;
  held: number;
  colorHeld: number;
  next: number;
};
type WaveSignal = { curve: ResponseCurve; profile: number[]; influence: number[]; last: number };
export class ExpressionPlayer {
  time = 0;
  get phase() {
    return this.flowPhase;
  }
  private lightPhase = 0;
  private flowPhase = 0;
  visible = true;
  working = false;
  momentum: Momentum = { speed: 0, phase: 0 };
  private opened = -10;
  private settleUntil = 0;
  private entrance?: AvatarExpression;
  private drives = Object.fromEntries(
    assignmentNames.map((a) => [
      a,
      { motion: new ResponseCurve(), color: new ResponseCurve(), held: 0, colorHeld: 0, next: 0 },
    ]),
  ) as Record<Assignment, Drive>;
  private history = Array.from({ length: 4 }, () => new ResponseCurve());
  private randomHistory = Array.from({ length: 4 }, () => new ResponseCurve());
  private seed = 12345;
  private waves = new Map<string, WaveSignal>();
  private phases = new Map<string, number>();
  private printOffset = [0, 0];
  private inputs: { at: number; action: PlaybackAction; working: boolean; visible: boolean }[] = [];
  setup: AvatarSetup;
  constructor(setup: AvatarSetup) {
    this.setup = setup;
  }
  private random() {
    this.seed = (Math.imul(this.seed, 1664525) + 1013904223) >>> 0;
    return this.seed / 4294967296;
  }
  private emit(e: AvatarExpression, scale = 1) {
    for (const p of e.patterns) {
      const channel = waveKinds.indexOf(p.kind);
      if (channel < 0) continue;
      const s = parameters(p);
      const amount = s.strength;
      // The source signal is sampled at a spatial delay in the shader. New taps
      // add to this curve; they never replace the wave already traveling outward.
      const duration =
        p.kind === "ripple"
          ? Math.max(0.1, ((s.width ?? 0.32) * (parameters(p).travel ?? 0.65)) / 3.14)
          : e.duration;
      this.history[channel].add(this.time, duration, amount * scale, 0.35);
      if (p.kind === "ripple") {
        const travel = Math.min(15, parameters(p).travel ?? 0.65);
        const profile = [s.originX ?? -0.7, s.originY ?? -0.6, travel, travel + duration + 0.1];
        const influence = [s.silhouette ?? 1, s.displacement ?? 1, s.shading ?? 1, 1];
        const key = JSON.stringify([profile, influence]);
        let wave = this.waves.get(key);
        if (!wave) {
          // Channels belong to spatial profiles, not individual taps. Repeated
          // taps always accumulate in the same curve, including while it travels.
          if (this.waves.size >= 8) {
            const oldest = [...this.waves].sort((a, b) => a[1].last - b[1].last)[0];
            this.waves.delete(oldest[0]);
          }
          wave = { curve: new ResponseCurve(), profile, influence, last: this.time };
          this.waves.set(key, wave);
        }
        wave.last = this.time;
        wave.curve.add(this.time, duration, amount * scale, 0.35);
      }
      if (p.kind === "sparks")
        for (const curve of this.randomHistory)
          curve.add(this.time, duration, (this.random() * 2 - 1) * amount * scale, 0.35);
    }
  }
  private settle(expression?: AvatarExpression) {
    if (!expression) return;
    this.settleUntil = Math.max(
      this.settleUntil,
      this.time +
        Math.max(
          4,
          expression.duration,
          expression.color.duration * 4,
          ...expression.patterns.map((p) => (parameters(p).travel ?? 0) + 1),
        ),
    );
  }
  trigger(action: PlaybackAction) {
    this.inputs.push({ at: this.time, action, working: this.working, visible: this.visible });
    if (action === "hide") {
      this.visible = false;
      return;
    }
    if (action === "complete") {
      if (this.working) this.settle(assigned(this.setup, "working"));
      this.working = false;
    }
    if (action === "idle" || action === "working") {
      if (this.working) this.settle(assigned(this.setup, "working"));
      this.working = action === "working";
      const e = assigned(this.setup, action);
      if (e?.patterns.some((p) => p.kind === "printed" || p.kind === "fill")) {
        this.opened = this.time;
        this.entrance = e;
      }
      return;
    }
    this.visible = true;
    const e = assigned(this.setup, action);
    if (
      action === "open" ||
      action === "openAfterIdle" ||
      e?.patterns.some((p) => p.kind === "printed" || p.kind === "fill")
    ) {
      this.opened = this.time;
      this.entrance = e;
    }
    if (!e) return;
    this.settle(e);
    const drive = this.drives[action];
    drive.motion.add(this.time, e.duration, 1, e.attack === undefined ? (e.momentum ? 0.3 : 0.1) : Math.min(0.9, e.attack / e.duration));
    drive.color.add(
      this.time,
      e.color.duration,
      e.color.strength,
      Math.min(0.9, e.color.attack / e.color.duration),
    );
    this.emit(e);
  }
  fork(setup: AvatarSetup) {
    const copy = new ExpressionPlayer(setup);
    const start = Math.max(0, this.time - 20);
    const events = this.inputs.filter((event) => event.at >= start);
    copy.time = start;
    copy.working = events[0]?.working ?? this.working;
    copy.visible = events[0]?.visible ?? this.visible;
    const advanceTo = (time: number) => {
      while (copy.time < time - 1e-8) {
        if (!copy.visible) {
          copy.time = time;
          break;
        }
        copy.advance(Math.min(1 / 120, time - copy.time));
      }
    };
    for (const event of events) {
      advanceTo(event.at);
      copy.trigger(event.action);
    }
    advanceTo(this.time);
    copy.time = this.time;
    // Start comparisons at the current field pose; their rates can then diverge.
    copy.setPose(this.lightPhase, this.flowPhase);
    copy.printOffset = [...this.printOffset];
    copy.seed = this.seed;
    for (const [key, phase] of this.phases) if (copy.phases.has(key)) copy.phases.set(key, phase);
    return copy;
  }
  inspect(assignment: Assignment, time = this.time) {
    const drive = this.drives[assignment];
    return {
      input: drive.motion.sample(time),
      color: drive.color.sample(time),
      ripple: this.history[0].sample(time),
      speed: this.momentum.speed,
      phase: this.momentum.phase,
    };
  }
  signals(assignment?: Assignment) {
    let motion = 0;
    let color = 0;
    for (const a of assignment ? [assignment] : assignmentNames) {
      const e = assigned(this.setup, a);
      if (!e) continue;
      const d = this.drives[a];
      motion += d.held + d.motion.sample(this.time);
      if (e.color.mode !== "none")
        color += d.color.sample(this.time) + d.colorHeld * e.color.strength;
    }
    return { motion, color, speed: this.momentum.speed };
  }
  colorSignals() {
    return assignmentNames.flatMap((a) => {
      const e = assigned(this.setup, a);
      if (!e || e.color.mode === "none") return [];
      const palette =
        e.color.mode === "custom" ? e.color.palette : this.setup.appearance.activityColor;
      if (!palette || palette === "mono") return [];
      const d = this.drives[a];
      return [{ palette, strength: d.color.sample(this.time) + d.colorHeld * e.color.strength }];
    });
  }
  needsAnimation() {
    if (!this.visible) return false;
    if (this.time < this.settleUntil || this.momentum.speed > 0.0001) return true;
    return assignmentNames.some((assignment) => {
      const expression = assigned(this.setup, assignment);
      if (!expression) return false;
      const held = assignment === "idle" || (assignment === "working" && this.working) ? 1 : 0;
      const drive = this.drives[assignment];
      if (
        Math.abs(drive.held - held) > 0.0001 ||
        (expression.color.mode !== "none" && Math.abs(drive.colorHeld - held) > 0.0001)
      )
        return true;
      return Boolean(
        held &&
        expression.patterns.some((pattern) => {
          const settings = parameters(pattern);
          if (pattern.kind === "lava") return settings.flowSpeed > 0 || settings.lightRotation > 0;
          if (settings.strength === 0) return false;
          return (
            waveKinds.includes(pattern.kind) ||
            pattern.kind === "nudge" ||
            (["stir", "eddies", "drift", "sweep", "swirl", "orbit"].includes(pattern.kind) &&
              settings.speed > 0)
          );
        }),
      );
    });
  }
  advance(delta: number) {
    if (!this.visible) return;
    const dt = Math.max(0, Math.min(0.064, delta));
    this.time += dt;
    while (this.inputs.length > 1 && this.inputs[1].at < this.time - 20) this.inputs.shift();
    for (const assignment of assignmentNames) {
      const e = assigned(this.setup, assignment);
      if (!e) continue;
      const d = this.drives[assignment];
      const held = assignment === "idle" || (assignment === "working" && this.working) ? 1 : 0;
      d.held += (held - d.held) * (1 - Math.exp(-dt * (held ? 10 : 6)));
      d.colorHeld +=
        (held - d.colorHeld) *
        (1 - Math.exp((-dt * 3) / Math.max(0.05, held ? e.color.attack : e.color.duration)));
      if (held && d.held > 0.01 && this.time >= d.next) {
        this.emit(e, d.held);
        d.next = this.time + Math.max(0.35, ...e.patterns.map((p) => parameters(p).travel ?? 1.5));
      }
    }
    const contributions = this.contributions();
    const priorMomentum = this.momentum.phase;
    advanceMomentum(this.momentum, contributions.push, dt, 7, 2.2);
    const nudge =
      dt * 2.4 * (1 - Math.exp(-contributions.nudge / 2.4)) + this.momentum.phase - priorMomentum;
    this.lightPhase += dt * contributions.lightSpeed + nudge;
    this.flowPhase += dt * contributions.flowSpeed + nudge;
    for (const c of contributions.effects) {
      this.phases.set(c.key, (this.phases.get(c.key) ?? 0) + dt * c.speed);
      if (c.kind === 6) {
        this.printOffset[0] += dt * c.amount * c.speed * Math.cos(c.options[2]);
        this.printOffset[1] += dt * c.amount * c.speed * Math.sin(c.options[2]);
      }
    }
    for (const [key, wave] of this.waves)
      if (this.time - wave.last > wave.profile[3]) this.waves.delete(key);
  }
  setPose(light: number, flow = light) {
    this.lightPhase = light;
    this.flowPhase = flow;
  }
  private contributions() {
    const effects: {
      key: string;
      kind: number;
      amount: number;
      speed: number;
      coupling: number;
      phase: number;
      options: number[];
    }[] = [];
    const paletteWeights = new Map<number, number>();
    let push = 0,
      nudge = 0,
      lightSpeed = 0,
      flowSpeed = 0,
      flowStrength = 0,
      lava = false,
      sparkCount = 0;
    for (const assignment of assignmentNames) {
      const expression = assigned(this.setup, assignment);
      if (!expression) continue;
      const d = this.drives[assignment];
      const level = d.held + d.motion.sample(this.time);
      if (expression.color.mode !== "none") {
        const palette = paletteNames.indexOf(
          expression.color.mode === "custom"
            ? expression.color.palette
            : (this.setup.appearance.activityColor ?? "opal"),
        );
        const color = d.color.sample(this.time) + d.colorHeld * expression.color.strength;
        if (palette > 0) paletteWeights.set(palette, (paletteWeights.get(palette) ?? 0) + color);
      }
      expression.patterns.forEach((p, index) => {
        const s = parameters(p);
        if (p.kind === "lava") {
          lava = true;
          flowStrength += level * s.flowStrength;
          lightSpeed += level * 0.38 * s.lightRotation;
          flowSpeed += level * 0.38 * s.flowSpeed;
        }
        if (p.kind === "nudge") {
          if (expression.momentum) push += level * s.strength * 34;
          else nudge += level * s.strength * 2;
        }
        if (p.kind === "sparks") sparkCount = Math.max(sparkCount, s.count);
        const kind = [
          "",
          "pressure",
          "tilt",
          "ruffle",
          "stir",
          "eddies",
          "drift",
          "sweep",
          "swirl",
          "orbit",
          "gather",
        ].indexOf(p.kind);
        if (kind < 1 || level < 0.00001) return;
        const key = `${assignment}:${expression.id}:${index}:${p.kind}`;
        const strength = s.strength;
        const speed = s.speed ?? 0;
        effects.push({
          key,
          kind,
          amount: level * strength,
          speed,
          coupling: s.distortion ?? 1,
          phase: this.phases.get(key) ?? 0,
          options: [
            s.spread ?? 3,
            s.offset ?? 0.36,
            p.kind === "sweep" ? (s.direction ?? 0) : ((s.angle ?? 20) * Math.PI) / 180,
            Number(Boolean(expression.distortionOnly)),
          ],
        });
      });
    }
    return {
      effects,
      paletteWeights,
      push,
      nudge,
      lightSpeed,
      flowSpeed,
      flowStrength: lava ? flowStrength : (this.setup.appearance.idleAmount ?? 1.6),
      sparkCount,
    };
  }
  // Sampling never advances time, emits a pulse, or consumes history.
  frame(): Uniforms {
    const { effects, paletteWeights, flowStrength, sparkCount } = this.contributions();
    const rippleTravel = Math.max(
      0.3,
      ...Object.values(this.setup.assignments)
        .filter((e): e is AvatarExpression => e !== null)
        .flatMap((e) =>
          e.patterns.filter((p) => p.kind === "ripple").map((p) => parameters(p).travel ?? 0.65),
        ),
    );
    const spans = [Math.min(16, rippleTravel + 0.8), 0.6, 0.8, 0.8];
    const packet: number[] = [],
      random: number[] = [];
    for (let i = 0; i < historyLength; i++) {
      for (let channel = 0; channel < 4; channel++)
        packet.push(
          this.history[channel].sample(this.time - (i * spans[channel]) / (historyLength - 1)),
        );
      for (const c of this.randomHistory)
        random.push(c.sample(this.time - (i * spans[1]) / (historyLength - 1)));
    }
    const waveTexture = Array.from({ length: 128 * 8 }, () => 0),
      waveProfiles = Array.from({ length: 32 }, () => 0),
      waveInfluences = Array.from({ length: 32 }, () => 0);
    [...this.waves.values()].forEach((wave, row) => {
      waveProfiles.splice(row * 4, 4, ...wave.profile);
      waveInfluences.splice(row * 4, 4, ...wave.influence);
      for (let i = 0; i < 128; i++)
        waveTexture[row * 128 + i] = wave.curve.sample(this.time - (i * wave.profile[3]) / 127);
    });
    const accents = [...paletteWeights]
      .filter(([, weight]) => weight > 0.0001)
      .sort((a, b) => a[0] - b[0])
      .slice(0, 4)
      .flatMap(([id, weight]) => [id, weight, 0, 0]);
    while (accents.length < 16) accents.push(0);
    const age = this.time - this.opened;
    const reveal = this.entrance?.patterns.find((p) => p.kind === "printed" || p.kind === "fill");
    const revealStart = Math.max(0, Math.min(1, this.entrance?.revealStart ?? 0));
    const appearance = reveal
      ? revealStart + (1 - revealStart) * Math.min(1, age / Math.max(0.1, this.entrance!.duration))
      : 1;
    const channels = effects.slice(0, 32);
    const effectData = channels.flatMap((c) => [c.kind, c.amount, c.phase, c.coupling]);
    const effectOptions = channels.flatMap((c) => c.options);
    while (effectData.length < 128) effectData.push(0);
    while (effectOptions.length < 128) effectOptions.push(0);
    return {
      waveTexture,
      "u_waveProfiles[0]": waveProfiles,
      "u_waveInfluences[0]": waveInfluences,
      u_historySpans: spans,
      "u_history[0]": packet,
      "u_randomHistory[0]": random,
      "u_accents[0]": accents,
      "u_effects[0]": effectData,
      "u_effectOptions[0]": effectOptions,
      u_effectCount: channels.length,
      u_phase: this.flowPhase,
      u_lavaPhase: [this.lightPhase, this.flowPhase],
      u_printOffset: [...this.printOffset],
      u_idleAmount: flowStrength,
      u_sparkCount: sparkCount,
      u_appearance: appearance,
      u_fillWake: Number(reveal?.kind === "fill"),
      u_enableOrb: Number(this.visible),
    };
  }
}
