import { ResponseCurve, advanceMomentum, type Momentum } from "./response-curve";
import {
  assigned,
  assignmentNames,
  type Assignment,
  type AvatarExpression,
  type AvatarSetup,
} from "./expression-model";
import type { SignalConfig } from "./signal-field";
export type Uniforms = Record<string, number | number[]>;
export const historyLength = 128;
const palettes = [
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
  phase = 0;
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
  private profiles: Record<string, Partial<SignalConfig>> = {};
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
      const amount =
        p.kind === "ripple"
          ? (p.settings.surfaceAmplitude ?? 0.45)
          : (p.settings.dispatchEnergy ?? 0.8);
      // The source signal is sampled at a spatial delay in the shader. New taps
      // add to this curve; they never replace the wave already traveling outward.
      const duration =
        p.kind === "ripple"
          ? Math.max(
              0.1,
              ((p.settings.surfaceWidth ?? 0.32) * (p.settings.surfaceDuration ?? 0.65)) / 3.14,
            )
          : e.duration;
      this.history[channel].add(this.time, duration, amount * scale, 0.35);
      if (p.kind === "ripple") {
        const travel = Math.min(15, p.settings.surfaceDuration ?? 0.65);
        const profile = [
          p.settings.surfaceOriginX ?? -0.7,
          p.settings.surfaceOriginY ?? -0.6,
          travel,
          travel + duration + 0.1,
        ];
        const influence = [
          p.settings.surfaceSilhouette ?? 1,
          p.settings.surfaceDisplacement ?? 1,
          p.settings.surfaceShading ?? 1,
          1,
        ];
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
          ...expression.patterns.map((p) => (p.settings.surfaceDuration ?? 0) + 1),
        ),
    );
  }
  trigger(action: PlaybackAction) {
    if (action === "hide") {
      this.visible = false;
      return;
    }
    if (action === "idle" || action === "working") {
      this.settle(assigned(this.setup, "working"));
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
    drive.motion.add(this.time, e.duration, 1, e.momentum ? 0.3 : 0.1);
    drive.color.add(
      this.time,
      e.color.duration,
      e.color.strength,
      Math.min(0.9, e.color.attack / e.color.duration),
    );
    this.emit(e);
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
    return (
      this.visible &&
      Boolean(
        assigned(this.setup, "idle") ||
        (this.working && assigned(this.setup, "working")) ||
        this.time < this.settleUntil ||
        this.momentum.speed > 0.0001,
      )
    );
  }
  advance(delta: number) {
    this.evaluate(delta);
  }
  config(): SignalConfig {
    // Static appearance is shared. Every motion supplies its own dynamic values.
    return {
      ...this.setup.appearance,
      effects: {
        ...this.setup.appearance.effects,
        input: false,
        selection: false,
        container: false,
        placeholder: false,
      },
      boundary: "none",
      shape: "pearl",
      colorTiming: "always",
    };
  }
  private evaluate(delta: number) {
    const dt = Math.max(0, Math.min(0.064, delta));
    if (this.visible) this.time += dt;
    const amounts: Record<string, number> = {};
    const settings: Record<string, Partial<SignalConfig>> = {};
    const paletteWeights = new Map<number, number>();
    let push = 0,
      directNudge = 0,
      flowOnly = 0,
      flowTotal = 0;
    for (const assignment of assignmentNames) {
      const e = assigned(this.setup, assignment);
      if (!e) continue;
      const d = this.drives[assignment];
      const held = assignment === "idle" ? 1 : assignment === "working" && this.working ? 1 : 0;
      d.held += (held - d.held) * (1 - Math.exp(-dt * 10));
      const level = d.held + d.motion.sample(this.time);
      if (held && d.held > 0.01 && this.time >= d.next) {
        this.emit(e, d.held);
        d.next =
          this.time + Math.max(0.35, ...e.patterns.map((p) => p.settings.surfaceDuration ?? 1.5));
      }
      d.colorHeld +=
        (held - d.colorHeld) *
        (1 - Math.exp((-dt * 3) / Math.max(0.05, held ? e.color.attack : e.color.duration)));
      let color = d.color.sample(this.time) + d.colorHeld * e.color.strength;
      if (e.color.mode === "none") color = 0;
      const palette = palettes.indexOf(
        e.color.mode === "custom"
          ? e.color.palette
          : (this.setup.appearance.activityColor ?? "opal"),
      );
      if (palette > 0) paletteWeights.set(palette, (paletteWeights.get(palette) ?? 0) + color);
      for (const p of e.patterns) {
        const key = p.kind;
        amounts[key] = (amounts[key] ?? 0) + level;
        if (level > 0.0001) this.profiles[key] = p.settings;
        settings[key] = this.profiles[key] ?? p.settings;
        if (key === "nudge") {
          const energy = p.settings.typingEnergy ?? 0.6;
          if (e.momentum) push += level * energy * 34;
          else directNudge += level * energy * 2;
        }
        if (key === "swirl" || key === "orbit") {
          flowTotal += level;
          if (e.distortionOnly) flowOnly += level;
        }
      }
    }
    const priorMomentumPhase = this.momentum.phase;
    advanceMomentum(this.momentum, push, dt, 7, 2.2);
    const lavaSpeed = (amounts.lava ?? 0) * 0.38 * Number(settings.lava?.loadingRate ?? 1);
    this.phase +=
      dt * (lavaSpeed + 2.4 * (1 - Math.exp(-directNudge / 2.4))) +
      this.momentum.phase -
      priorMomentumPhase;
    return { amounts, settings, paletteWeights, flowOnly, flowTotal };
  }
  frame(delta: number): Uniforms {
    const { amounts, settings, paletteWeights, flowOnly, flowTotal } = this.evaluate(delta);
    const amount = (key: string, setting: keyof SignalConfig, fallback: number) =>
      (amounts[key] ?? 0) * Number(settings[key]?.[setting] ?? fallback);
    const rippleTravel = Math.max(
      0.3,
      ...this.setup.expressions
        .filter((e) => Object.values(this.setup.assignments).includes(e.id))
        .flatMap((e) =>
          e.patterns
            .filter((p) => p.kind === "ripple")
            .map((p) => p.settings.surfaceDuration ?? 0.65),
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
    for (const [key, wave] of this.waves)
      if (this.time - wave.last > wave.profile[3]) this.waves.delete(key);
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
    const pressure = amount("pressure", "typingEnergy", 0.28);
    const tilt = amount("tilt", "typingEnergy", 1) * 0.25;
    const ripple =
      settings.ripple ?? this.entrance?.patterns.find((p) => p.kind === "ripple")?.settings ?? {};
    const age = this.time - this.opened;
    const reveal = this.entrance?.patterns.find((p) => p.kind === "printed" || p.kind === "fill");
    const revealStart = Math.max(0, Math.min(1, this.entrance?.revealStart ?? 0));
    const appearance = reveal
      ? revealStart + (1 - revealStart) * Math.min(1, age / Math.max(0.1, this.entrance!.duration))
      : 1;
    const loading = Math.min(1, amounts.orbit ?? 0);
    return {
      u_driven: 1,
      waveTexture,
      "u_waveProfiles[0]": waveProfiles,
      "u_waveInfluences[0]": waveInfluences,
      u_historySpans: spans,
      "u_history[0]": packet,
      "u_randomHistory[0]": random,
      "u_accents[0]": accents,
      u_phase: this.phase,
      u_seed: 0.37,
      u_time: this.time,
      u_wake: 0,
      u_momentum: 0,
      u_releaseAge: 10,
      u_priorReleaseAge: 10,
      u_typingSparkAge: 10,
      u_releasePhase: 0,
      u_priorReleasePhase: 0,
      u_burst: amount("gather", "dispatchEnergy", 1),
      u_echo: 0,
      u_releaseMode: 0,
      u_dispatchTarget: 0,
      u_tap: amount("ruffle", "typingEnergy", 0.5),
      u_attack: 0,
      u_dispatch: 0,
      u_colorActivity: Math.min(
        1,
        [...paletteWeights.values()].reduce((a, b) => a + b, 0),
      ),
      u_poke: [
        pressure * Math.cos(this.phase * 1.3) + tilt,
        pressure * Math.sin(this.phase * 1.3) + tilt * 0.4,
      ],
      u_poke2: [0, 0],
      u_typingStyle: pressure > 0.001 ? 4 : (amounts.ruffle ?? 0) > 0.001 ? 6 : 5,
      u_pressureProfile: [
        settings.pressure?.pressureSpread ?? 3,
        settings.pressure?.pressureOffset ?? 0.36,
      ],
      u_mixing: 1,
      u_mixCustom: 1,
      u_lavaMix: 1,
      u_loading: 0,
      u_loadingMotion: 1,
      u_effectMix: [1, 1, 1],
      u_mixWaveWeight: 1,
      u_mixDrift: [
        amount("drift", "loadingStrength", 1),
        settings.drift?.coupling ?? 1,
        this.time,
        this.time * (settings.drift?.loadingRate ?? 1),
      ],
      u_mixEddies: [
        amount("eddies", "loadingStrength", 1),
        settings.eddies?.coupling ?? 1,
        this.time * (settings.eddies?.loadingRate ?? 1),
        0,
      ],
      u_mixAgitation: [
        amount("stir", "loadingStrength", 1),
        settings.stir?.coupling ?? 1,
        this.time * (settings.stir?.loadingRate ?? 1),
        0,
      ],
      u_mixWave: [
        amount("sweep", "loadingStrength", 1),
        settings.sweep?.coupling ?? 1,
        this.time * (settings.sweep?.loadingRate ?? 1),
        0,
      ],
      u_mixWaveMode: [
        "orbit",
        "alternating",
        "bands",
        "strong",
        "right-left",
        "drift",
        "quicker",
        "agitated",
        "left-right",
        "top-bottom",
        "bottom-top",
        "random",
      ].indexOf(settings.sweep?.loadingMotion ?? "alternating"),
      u_mixDriftAngle: ((settings.drift?.driftAngle ?? 20) * Math.PI) / 180,
      u_orbit: [
        amount("swirl", "idleSwirl", 0.8),
        amount("orbit", "workingOrbit", 1),
        settings.swirl?.swirlRate ?? 0.8,
        settings.orbit?.orbitRate ?? 1.8,
      ],
      u_flowOnly: flowTotal ? flowOnly / flowTotal : 0,
      u_loopRipple: [0, 4],
      u_entranceRipple: [0, 0],
      u_surfaceWave: [0, 0],
      u_surfaceExpression: [
        ripple.surfaceAmplitude ?? 0.45,
        Math.min(15, ripple.surfaceDuration ?? 0.65),
      ],
      u_surfaceOrigin: [ripple.surfaceOriginX ?? -0.7, ripple.surfaceOriginY ?? -0.6],
      u_surfaceInfluence: [
        ripple.surfaceSilhouette ?? 1,
        ripple.surfaceDisplacement ?? 1,
        ripple.surfaceShading ?? 1,
      ],
      u_sparkCount: settings.sparks?.sparkCount ?? 11,
      u_appearance: appearance,
      u_fillWake: Number(reveal?.kind === "fill"),
      u_idleAmount: settings.lava?.idleAmount ?? this.setup.appearance.idleAmount ?? 1.6,
      u_enableOrb: Number(this.visible),
      u_drivenOrbit: loading,
    };
  }
}
