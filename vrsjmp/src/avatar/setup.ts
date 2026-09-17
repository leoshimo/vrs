import { defaultSignal, type SignalConfig } from "./signal-field.ts";
import { expressions, expressionSettings, type AvatarAction } from "./avatar-expressions.ts";
export type EventName = "idle" | "typing" | "working" | "open" | "flourish" | "submit";
export const eventLabels: Record<EventName, string> = {
  idle: "Idle",
  typing: "Typing",
  working: "Working",
  open: "Open",
  flourish: "Open · Flourish",
  submit: "Submit",
};
export type Motion = {
  preset: string;
  action: AvatarAction;
  settings: Partial<SignalConfig>;
  duration: number;
};
export type SavedExpression = { id: string; name: string; motions: Motion[] };
export type Setup = {
  version: 1;
  initialized: boolean;
  appearance: SignalConfig;
  expressions: SavedExpression[];
  events: Record<EventName, string>;
};
export function motionFromPreset(id: string): Motion {
  const p = expressions.find((e) => e.id === id);
  if (!p) throw new Error("Unknown motion");
  return {
    preset: id,
    action: p.action,
    settings: expressionSettings(p.action, { ...defaultSignal, ...p.settings }),
    duration: p.duration ?? 1.4,
  };
}
export function family(m: Motion) {
  if (m.action === "loading")
    return m.settings.loadingMotion === "surface"
      ? "ripple"
      : m.settings.loadingMotion === "agitated"
        ? "stir"
        : m.settings.loadingMotion === "eddy"
          ? "eddies"
          : m.settings.loadingMotion === "drift"
            ? "drift"
            : m.settings.loadingMotion === "quicker"
              ? "lava"
              : "sweep";
  return m.action;
}
export function makeDefaultSetup(): Setup {
  const library = expressions
    .filter((e) => e.settings.loadingMotion !== "radial")
    .map((e) => ({
      id: e.id,
      name: e.name,
      motions: [motionFromPreset(e.id)],
    }));
  const ripple = motionFromPreset("surface-working");
  ripple.settings = {
    ...ripple.settings,
    surfaceAmplitude: 0.28,
    surfaceDuration: 5,
  };
  library.push({
    id: "working",
    name: "Restless ripple",
    motions: [motionFromPreset("restless"), ripple],
  });
  library.push({ id: "quick-open", name: "Quick entrance", motions: [] });
  return {
    version: 1,
    initialized: false,
    appearance: {
      ...defaultSignal,
      pitch: 1.4,
      idleAmount: 1.6,
      shape: "pearl",
    },
    expressions: library,
    events: {
      idle: "rest",
      typing: "field-nudge",
      working: "working",
      open: "quick-open",
      flourish: "surface-submit",
      submit: "sparks",
    },
  };
}
export function compatible(event: EventName, e: SavedExpression) {
  const actions: Record<EventName, AvatarAction[]> = {
    idle: ["idle"],
    typing: ["typing"],
    working: ["loading"],
    open: ["wake", "dispatch"],
    flourish: ["wake", "dispatch"],
    submit: ["dispatch"],
  };
  return (
    e.motions.every((m) => actions[event].includes(m.action)) &&
    (e.motions.length > 0 || event === "open" || event === "flourish")
  );
}
export function expression(setup: Setup, event: EventName) {
  return setup.expressions.find((e) => e.id === setup.events[event])!;
}
// Event activation changes envelopes, never the underlying shader profile.
// Keeping Working configured lets its existing field fade to rest without
// exposing a different loading effect during the release.
export function eventConfig(setup: Setup, transient?: EventName | null) {
  return compose(setup.appearance, [
    ...expression(setup, "idle").motions,
    ...expression(setup, "typing").motions,
    ...expression(setup, "submit").motions,
    ...expression(setup, "working").motions,
    ...(transient ? expression(setup, transient).motions : []),
  ]);
}
// Compose each field contribution explicitly. Loading motions retain independent parameters.
export function compose(base: SignalConfig, motions: Motion[]): SignalConfig {
  const config: SignalConfig = {
    ...base,
    shape: "pearl",
    effects: { ...base.effects, placeholder: false },
    loopRipple: undefined,
    effectMix: undefined,
    effectLayerSettings: undefined,
  };
  const applyLoading = (settings: Partial<SignalConfig>) =>
    Object.assign(
      config,
      Object.fromEntries(Object.entries(settings).filter(([key]) => !key.startsWith("surface"))),
    );
  const loading = motions.filter((m) => m.action === "loading");
  for (const m of motions) if (m.action !== "loading") Object.assign(config, m.settings);
  if (loading.length === 1 && loading[0].settings.loadingMotion !== "surface")
    applyLoading(loading[0].settings);
  else if (loading.length) {
    config.effectMix = { drift: 0, eddies: 0, agitation: 0, wave: 0 };
    config.effectLayerSettings = {};
    config.loadingMotion = "alternating";
    config.loadingStrength = 1;
    for (const m of loading) {
      const f = family(m);
      if (f === "ripple") config.loopRipple = { ...m.settings };
      else if (f === "lava") applyLoading(m.settings);
      else {
        const key = f === "stir" ? "agitation" : f === "sweep" ? "wave" : (f as "drift" | "eddies");
        config.effectMix[key] = 1;
        config.effectLayerSettings[key] = { ...m.settings };
      }
    }
  }
  return config;
}
export function validateSetup(value: unknown): asserts value is Setup {
  if (!value || typeof value !== "object") throw new Error("Invalid setup");
  const v = value as Setup;
  if (
    v.version !== 1 ||
    !Array.isArray(v.expressions) ||
    v.expressions.length > 150 ||
    !v.appearance ||
    !v.events
  )
    throw new Error("Invalid setup format");
  if (
    v.appearance.shape !== "pearl" ||
    !v.appearance.effects ||
    !Number.isFinite(v.appearance.pitch)
  )
    throw new Error("Invalid appearance");
  const ids = new Set<string>();
  for (const e of v.expressions) {
    if (
      typeof e.id !== "string" ||
      !/^[a-z0-9-]{1,100}$/.test(e.id) ||
      ids.has(e.id) ||
      typeof e.name !== "string" ||
      !e.name.trim() ||
      e.name.length > 100 ||
      !Array.isArray(e.motions) ||
      e.motions.length > 6
    )
      throw new Error("Invalid expression");
    ids.add(e.id);
    const families = new Set<string>();
    for (const m of e.motions) {
      const original = expressions.find((p) => p.id === m.preset);
      if (
        !original ||
        m.action !== original.action ||
        !m.settings ||
        typeof m.settings !== "object" ||
        !Number.isFinite(m.duration) ||
        m.duration < 0.1 ||
        m.duration > 30
      )
        throw new Error("Invalid motion");
      const f = family(m);
      if (families.has(f)) throw new Error("Use one motion from each family");
      families.add(f);
      for (const [key, n] of Object.entries(m.settings)) {
        if (["__proto__", "constructor", "prototype"].includes(key))
          throw new Error("Invalid setting");
        if (typeof n === "number" && (!Number.isFinite(n) || Math.abs(n) > 10000))
          throw new Error("Invalid setting value");
        if (!["number", "string", "boolean"].includes(typeof n))
          throw new Error("Invalid setting value");
      }
    }
    if (e.motions.length > 1 && !e.motions.every((m) => m.action === "loading"))
      throw new Error("Combine motions within Working");
  }
  for (const event of Object.keys(eventLabels) as EventName[]) {
    const target = v.expressions.find((e) => e.id === v.events[event]);
    if (!target || !compatible(event, target))
      throw new Error(`Invalid ${eventLabels[event]} assignment`);
  }
}
