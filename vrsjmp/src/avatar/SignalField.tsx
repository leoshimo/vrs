"use client";
import { useCallback, useEffect, useLayoutEffect, useRef } from "react";
import { NativeShader, type NativeShaderHandle } from "./NativeShader";
import { signalFragment, type SignalConfig } from "./signal-field";
import { createSignalMotion, advanceSignalMotion } from "./signal-motion";
import { paletteAccents } from "./avatar-palettes";
import type { MotionMode as OrbState } from "./signal-motion";

export type DispatchOrigin = {
  serial: number;
  rect: number[];
  radius: number;
  context?: string;
};
const selectionDuration = 0.12;
function color(value: string, fallback: number[]) {
  const hex = value.trim().match(/^#([\da-f]{6})$/i)?.[1];
  return hex ? [0, 2, 4].map((i) => parseInt(hex.slice(i, i + 2), 16) / 255) : fallback;
}
function radius(node: Element | null | undefined, fallback = 0) {
  return node ? parseFloat(getComputedStyle(node).borderTopLeftRadius) || fallback : fallback;
}

export const initialSignalUniforms = {
  u_hit: -100,
  u_state: 0,
  u_shape: 0,
  u_breathe: 1,
  u_motion: 1,
  u_texture: 0,
  u_shadow: 1,
  u_surface: 0,
  u_selectionStyle: 0,
  u_dark: 1,
  u_selectAt: -100,
  u_appearance: 1,
  u_typingSparkAge: 10,
  u_fill: 0.5,
  u_contrast: 1,
  u_matrix: 8,
  u_releaseRow: [0, 0, 0, 0],
  u_releaseRadius: 0,
  u_selectDuration: 0,
  u_split: 0,
  u_radii: [34, 20, 19, 29],
  u_paperColor: [0.082, 0.086, 0.075],
  u_inkColor: [0.89, 0.9, 0.81],
  u_rowColor: [0.86, 0.89, 0.79],
  u_signal: [0, 0, 0, 0],
  u_panel: [0, 0, 0, 0],
  u_query: [0, 0, 0, 0],
  u_island: [0, 0, 0, 0],
  u_selection: [0, 0, 0, 0],
  u_previous: [0, 0, 0, 0],
  u_clip: [0, 0, 0, 0],
};
function paletteIndex(color: SignalConfig["color"]) {
  return [
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
  ].indexOf(color ?? "mono");
}

export function signalUniforms(
  config: SignalConfig,
  animate = true,
  dark = true,
  selected = "avatar",
  appearance = 1,
  standalone = true,
) {
  return {
    u_shape: ["pearl", "square", "charged", "duet", "ribbon", "gentle"].indexOf(config.shape),
    u_color: paletteIndex(config.color),
    u_activityColor: paletteIndex(config.activityColor),
    u_colorTiming: Number(config.colorTiming !== "always"),
    u_loadingMotion: [
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
      "eddy",
      "radial",
      "surface",
    ].indexOf(config.loadingMotion || "alternating"),
    u_typingStyle: [
      "dots",
      "both",
      "cross-dots",
      "cross-both",
      "pressure",
      "light",
      "agitation",
    ].indexOf(config.typingMotion === "nudge" ? "light" : config.typingMotion || "dots"),
    u_inputStyle: ["top", "bottom", "shadow", "flat-top", "flat-bottom"].indexOf(
      config.inputTreatment || "bottom",
    ),
    u_edgeSignal: ["off", "leading", "bottom"].indexOf(config.edgeSignal || "off"),
    u_edgeMotion: ["wave", "traveler", "alternating"].indexOf(config.edgeMotion || "wave"),
    u_breathe: Number(config.breathe),
    u_motion: Number(animate),
    u_texture: Number(config.texture === "halftone"),
    u_targets: [
      Number(config.effects.indicator),
      Number(config.effects.selection),
      Number(config.effects.input),
      Number(config.effects.container),
    ],
    u_edge: ["bottom", "leading", "wave", "all"].indexOf(config.edge),
    u_amount: config.amount,
    u_pitch: config.pitch,
    u_expression: config.expression,
    u_entranceRipple: [
      config.entranceRipple ? (config.surfaceAmplitude ?? 0.5) : 0,
      appearance * 4,
    ],
    u_orbit: [
      config.idleSwirl ?? 0,
      config.workingOrbit ?? 0,
      config.swirlRate ?? 0.8,
      config.orbitRate ?? 1,
    ],
    u_pressureProfile: [config.pressureSpread ?? 3, config.pressureOffset ?? 0.36],
    u_interior: Number(config.interior === "spheres"),
    u_idleAmount: config.idleAmount ?? 1.2,
    u_fill: config.fill ?? 0.5,
    u_saturation: config.saturation ?? 0.45,
    u_activitySaturation: config.activitySaturation ?? 0.65,
    u_gravity: config.gravity ?? 0,
    ...(config.inspectPhase === undefined ? {} : { u_phase: config.inspectPhase }),
    u_lavaMix: config.lavaMix ?? 1,
    u_mixing: Number(!!config.effectMix),
    u_mixCustom: Number(!!config.effectLayerSettings),
    u_mixWaveWeight: config.effectMix?.wave ?? 0,
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
      "eddy",
      "radial",
      "surface",
    ].indexOf(config.effectLayerSettings?.wave?.loadingMotion ?? "alternating"),
    u_mixWaveAccent: Number(config.effectLayerSettings?.wave?.rippleAccent ?? false),
    u_mixDriftAngle: ((config.effectLayerSettings?.drift?.driftAngle ?? 20) * Math.PI) / 180,
    u_effectMix: [
      config.effectMix?.drift ?? 0,
      config.effectMix?.eddies ?? 0,
      config.effectMix?.agitation ?? 0,
    ],
    u_surfaceOrigin: [config.surfaceOriginX ?? -0.3, config.surfaceOriginY ?? -0.45],
    u_surfaceProfile: [
      config.surfaceWidth ?? 0.32,
      config.surfaceWavelength ?? Math.PI / 6,
      config.surfaceDamping ?? 0,
    ],
    u_surfaceInfluence: [
      config.surfaceSilhouette ?? 1,
      config.surfaceDisplacement ?? 1,
      config.surfaceShading ?? 1,
    ],
    u_loopRipple: [
      config.loopRipple?.surfaceAmplitude ?? 0,
      config.loopRipple?.surfaceDuration ?? 4,
    ],
    u_loopOrigin: [
      config.loopRipple?.surfaceOriginX ?? -0.3,
      config.loopRipple?.surfaceOriginY ?? -0.45,
    ],
    u_loopProfile: [
      config.loopRipple?.surfaceWidth ?? 0.32,
      config.loopRipple?.surfaceWavelength ?? Math.PI / 6,
      config.loopRipple?.surfaceDamping ?? 0,
    ],
    u_loopInfluence: [
      config.loopRipple?.surfaceSilhouette ?? 1,
      config.loopRipple?.surfaceDisplacement ?? 1,
      config.loopRipple?.surfaceShading ?? 1,
    ],
    u_surfaceWave: [config.surfaceWave || 0, config.surfacePhase || 0],
    u_surfaceExpression: [config.surfaceAmplitude ?? 0.8, config.surfaceDuration ?? 3],
    u_primitives: [
      config.primitiveDrift || 0,
      config.primitiveTwist || 0,
      config.primitiveNoise || 0,
      config.primitivePigment || 0,
    ],
    u_volume: config.volume ?? 1,
    u_lightStrength: config.lightStrength ?? 1,
    u_lightAngle: ((config.lightAngle ?? 0) * Math.PI) / 180,
    u_driftAngle: ((config.driftAngle ?? 0) * Math.PI) / 180,
    u_waveOffset: config.waveOffset ?? 0,
    u_fillWake: Number(config.appearanceMode === "fill"),
    u_selectionMotion: ["glide", "crossfade", "instant"].indexOf(
      config.selectionMotion ?? "crossfade",
    ),
    u_selectionTint: config.selectionTint ?? 0.1,
    u_rowEffect: ["none", "settle", "wick", "pool", "inset", "seam", "trail", "print"].indexOf(
      config.selectionEffect || "none",
    ),
    u_rowAmount: config.selectionInk ?? 0.45,
    u_rowPitch: config.selectionGrain ?? 1.2,
    u_rowTexture: Number(config.selectionTexture === "halftone"),
    u_uiAccent: color(
      paletteAccents[config.uiAccentPalette || config.color || "mono"] || "#90958e",
      [0.56, 0.58, 0.55],
    ),
    u_coupling: config.coupling ?? 1,
    u_boundary: ["none", "inner", "outer"].indexOf(config.boundary || "none"),
    u_shapeNoise: config.shapeNoise ?? 0,
    u_toneSteps: config.toneSteps ?? 0,
    u_colorStrength: config.colorStrength ?? 0.8,
    u_rippleAccent: Number(config.rippleAccent),
    u_sparkCount: config.sparkCount ?? 10,
    u_debugStage: config.debugStage ?? 0,
    u_contrast: config.contrast ?? 1,
    u_matrix: config.matrix ?? 8,
    u_dispatchTarget: Number(!standalone && config.dispatchTarget === "selection"),
    u_releaseMode: ["gather", "bloom", "sparks", "ripple", "surface"].indexOf(
      config.avatarDispatch || "gather",
    ),
    u_releaseStyle: ["contour", "sweep", "press"].indexOf(config.selectionRelease || "contour"),
    u_language: ["porcelain", "orbital", "seal"].indexOf(config.language || "porcelain"),
    u_selectionStyle: Number(config.selection === "shadow"),
    u_dark: Number(dark),
    u_appearance: appearance,
  };
}

export function SignalField({
  config,
  state,
  working,
  pulse,
  selected,
  animate,
  dark,
  appearance = 1,
  hideSignal = false,
  standalone = false,
  active = true,
  selectionReplay = 0,
  selectionContext = "",
  instantSelection = false,
  invocation = 0,
  dispatchOrigin,
}: {
  config: SignalConfig;
  state: OrbState;
  working?: boolean;
  pulse: number;
  selected: string;
  animate: boolean;
  dark: boolean;
  appearance?: number;
  hideSignal?: boolean;
  standalone?: boolean;
  active?: boolean;
  selectionReplay?: number;
  selectionContext?: string;
  instantSelection?: boolean;
  invocation?: number;
  dispatchOrigin?: DispatchOrigin;
}) {
  const host = useRef<HTMLDivElement>(null),
    shader = useRef<NativeShaderHandle>(null);
  const redraw = useRef<() => void>(() => {});
  const selection = useRef({
    from: [0, 0, 0, 0],
    to: [0, 0, 0, 0],
    at: -100,
    duration: 0,
  });
  const resetSelection = useRef(true);
  const context = useRef(selectionContext);
  const motion = useRef(createSignalMotion());
  const mixMotions = useRef({
    drift: createSignalMotion(10),
    eddies: createSignalMotion(20),
    agitation: createSignalMotion(30),
    wave: createSignalMotion(40),
  });
  const mixSettings = useRef(config.effectLayerSettings);
  useLayoutEffect(() => {
    mixSettings.current = config.effectLayerSettings;
  }, [config.effectLayerSettings]);
  const usedOrigin = useRef(-1);
  const inputs = useRef({
    state,
    working,
    pulse,
    animate,
    breathe: config.breathe,
    inspectPhase: config.inspectPhase,
    inspectPoke: config.inspectPoke,
    dark,
    instantSelection: instantSelection || config.selectionMotion === "instant",
    invocation,
    dispatchOrigin,
    loadingMotion: config.loadingMotion,
    idleAmount: config.idleAmount,
    typingEnergy: config.typingEnergy,
    typingNudge: config.typingMotion === "nudge",
    typingSparks: config.typingSparks,
    dispatchEnergy: config.dispatchEnergy,
    settle: config.settle,
    loadingStrength: config.loadingStrength,
    loadingRate: config.loadingRate,
    selectionContext,
    selectionDuration: config.selectionDuration ?? selectionDuration,
    crossfade: config.selectionMotion === "crossfade",
  });
  useLayoutEffect(() => {
    inputs.current = {
      state,
      working,
      pulse,
      animate,
      breathe: config.breathe,
      inspectPhase: config.inspectPhase,
      inspectPoke: config.inspectPoke,
      dark,
      instantSelection: instantSelection || config.selectionMotion === "instant",
      invocation,
      dispatchOrigin,
      loadingMotion: config.loadingMotion,
      idleAmount: config.idleAmount,
      typingEnergy: config.typingEnergy,
      typingNudge: config.typingMotion === "nudge",
      typingSparks: config.typingSparks,
      dispatchEnergy: config.dispatchEnergy,
      settle: config.settle,
      loadingStrength: config.loadingStrength,
      loadingRate: config.loadingRate,
      selectionContext,
      selectionDuration: config.selectionDuration ?? selectionDuration,
      crossfade: config.selectionMotion === "crossfade",
    };
    if (context.current !== selectionContext) {
      resetSelection.current = true;
      shader.current?.setUniforms({ u_releaseRow: [0, 0, 0, 0] });
    }
    context.current = selectionContext;
    shader.current?.setUniforms({ u_enableOrb: Number(!hideSignal) });
  }, [
    state,
    working,
    pulse,
    animate,
    config.breathe,
    config.inspectPhase,
    config.inspectPoke,
    config.selectionMotion,
    config.selectionDuration,
    hideSignal,
    dark,
    instantSelection,
    selectionContext,
    invocation,
    dispatchOrigin,
    config.loadingMotion,
    config.idleAmount,
    config.typingEnergy,
    config.typingSparks,
    config.typingMotion,
    config.dispatchEnergy,
    config.settle,
    config.loadingStrength,
    config.loadingRate,
  ]);
  const onFrame = useCallback((dt: number) => {
    const d = inputs.current;
    const releaseUniforms: Record<string, number | number[]> = {};
    if (
      dt > 0 &&
      d.state === "dispatch" &&
      (motion.current.mode !== "dispatch" || motion.current.pulse !== d.pulse)
    ) {
      const origin = d.dispatchOrigin;
      if (origin && origin.serial !== usedOrigin.current) {
        usedOrigin.current = origin.serial;
        releaseUniforms.u_releaseRow =
          !origin.context || d.selectionContext.startsWith(origin.context + "/")
            ? origin.rect
            : [0, 0, 0, 0];
        releaseUniforms.u_releaseRadius = origin.radius;
      } else {
        releaseUniforms.u_releaseRow = selection.current.to;
        releaseUniforms.u_releaseRadius = Math.min(19, selection.current.to[3] / 2);
      }
    }
    const m = advanceSignalMotion(motion.current, d.state, d.pulse, d.breathe, dt, d.animate, {
      invocation: d.invocation,
      working: d.working,
      typingEnergy: d.typingEnergy,
      typingNudge: d.typingNudge,
      typingSparks: d.typingSparks,
      dispatchEnergy: d.dispatchEnergy,
      settle: d.settle,
      idleAmount: d.idleAmount,
      loadingRate: d.loadingRate,
      loading:
        d.loadingMotion === "quicker"
          ? "quicker"
          : d.loadingMotion === "agitated"
            ? "agitated"
            : d.loadingMotion === "drift"
              ? "drift"
              : "normal",
    });
    const mixUniforms: Record<string, number | number[]> = {};
    if (mixSettings.current) {
      for (const [key, uniform] of [
        ["drift", "u_mixDrift"],
        ["eddies", "u_mixEddies"],
        ["agitation", "u_mixAgitation"],
        ["wave", "u_mixWave"],
      ] as const) {
        const c = mixSettings.current[key] ?? {};
        const layer = advanceSignalMotion(
          mixMotions.current[key],
          (d.working ?? d.state === "loading") ? "loading" : "idle",
          0,
          d.breathe,
          dt,
          d.animate,
          {
            idleAmount: c.idleAmount,
            loadingRate: c.loadingRate,
            loading: key === "drift" ? "drift" : key === "agitation" ? "agitated" : "normal",
          },
        );
        mixUniforms[uniform] = [
          layer.loading.value * (c.loadingStrength ?? 1),
          c.coupling ?? 1,
          layer.drift,
          layer.transport,
        ];
      }
    }
    return {
      ...mixUniforms,
      ...releaseUniforms,
      u_phase: d.inspectPhase ?? m.drift,
      u_wake: m.wake.value,
      u_momentum: m.momentum.value,
      u_seed: m.variation,
      u_transport: m.transport,
      u_poke: d.inspectPoke ?? [m.pokeX.value, m.pokeY.value],
      u_poke2: [m.poke2X.value, m.poke2Y.value],
      u_releaseAge: m.releaseAge,
      u_releasePhase: m.releasePhase,
      u_priorReleaseAge: m.priorReleaseAge,
      u_priorReleasePhase: m.priorReleasePhase,
      u_attack: Math.max(0, m.typing.value),
      u_loading: Math.max(0, m.loading.value) * (d.loadingStrength ?? 1),
      u_dispatch: Math.max(0, m.dispatch.value),
      u_tap: Math.max(0, m.tap.value),
      u_colorActivity: m.fieldImpulseVelocity * 0.6,
      u_typingSparkAge: m.typingSparkAge,
      u_burst: Math.max(0, m.burst.value),
      u_echo: Math.max(0, m.echo.value),
    };
  }, []);
  useLayoutEffect(() => {
    shader.current?.setUniforms(
      signalUniforms(config, animate, dark, selected, appearance, standalone),
    );
    redraw.current();
  }, [config, animate, dark, selected, appearance, standalone]);
  useLayoutEffect(() => {
    if (!selectionReplay) return;
    const mount = shader.current;
    if (!mount) return;
    const at = mount.getCurrentFrame() / 1000;
    selection.current = {
      ...selection.current,
      from: selection.current.to,
      at,
      duration: inputs.current.animate ? inputs.current.selectionDuration : 0,
    };
    mount.setUniforms({
      u_previous: selection.current.to,
      u_selectAt: at,
      u_selectDuration: selection.current.duration,
    });
  }, [selectionReplay]);
  useEffect(() => {
    const el = host.current,
      root = el?.parentElement;
    if (!el || !root) return;
    let frame = 0,
      retries = 0;
    let measureScaleX = 1,
      measureScaleY = 1;
    const rect = (node: Element | null | undefined, base: DOMRect) => {
      if (!node) return [0, 0, 0, 0];
      const r = node.getBoundingClientRect();
      return [
        (r.left - base.left) * measureScaleX,
        (r.top - base.top) * measureScaleY,
        r.width * measureScaleX,
        r.height * measureScaleY,
      ];
    };
    function measure() {
      frame = 0;
      const mount = shader.current;
      if (!mount) {
        if (retries++ < 30) frame = requestAnimationFrame(measure);
        return;
      }
      const base = el!.getBoundingClientRect();
      // CSS zoom/presentation scale must not change the shader's design coordinates.
      measureScaleX = base.width > 0 ? el!.clientWidth / base.width : 1;
      measureScaleY = base.height > 0 ? el!.clientHeight / base.height : 1;
      const anchor = rect(root!.querySelector(".signal-anchor,.kernel-anchor"), base);
      const rowNode = root!.querySelector('.activity-row[data-selected="true"]');
      const next = rect(rowNode, base);
      const viewport = rect(root!.querySelector(".activity-list"), base);
      const detached = !!root!.closest('[data-detached="true"]');
      const split = detached || !!root!.closest(".timeline-palette");
      const queryNode = root!.querySelector(".activity-query");
      const bodyNode = split ? root!.querySelector(".activity-body") : root!;
      const panel = standalone ? [0, 0, 0, 0] : rect(bodyNode, base);
      const now = mount.getCurrentFrame() / 1000;
      const old = selection.current;
      const snap =
        resetSelection.current || inputs.current.instantSelection || !inputs.current.animate;
      resetSelection.current = false;
      if (snap || next.some((n, i) => Math.abs(n - old.to[i]) > 0.5)) {
        let p = old.duration ? Math.min(1, Math.max(0, (now - old.at) / old.duration)) : 1;
        p = 1 - Math.pow(1 - p, 3);
        const from =
          !snap && old.to[2] && next[2]
            ? inputs.current.crossfade
              ? old.to
              : old.from.map((n, i) => n + (old.to[i] - n) * p)
            : next;
        selection.current = {
          from,
          to: next,
          at: now,
          duration: snap ? 0 : inputs.current.selectionDuration,
        };
      }
      const style = getComputedStyle(root!);
      mount.setUniforms({
        u_split: Number(split),
        u_radii: [
          radius(detached ? bodyNode : split ? bodyNode : root),
          radius(queryNode),
          radius(rowNode, 19),
          anchor[3] / 2,
        ],
        u_paperColor: color(
          style.getPropertyValue("--sheet"),
          inputs.current.dark ? [0.082, 0.086, 0.075] : [0.941, 0.945, 0.921],
        ),
        u_inkColor: color(
          style.getPropertyValue("--ink"),
          inputs.current.dark ? [0.89, 0.9, 0.81] : [0.22, 0.29, 0.24],
        ),
        u_selectionInkColor: color(
          style.getPropertyValue("--selection-ink"),
          inputs.current.dark ? [0.13, 0.14, 0.12] : [0.14, 0.25, 0.26],
        ),
        u_rowColor: color(
          style.getPropertyValue("--selection"),
          inputs.current.dark ? [0.86, 0.89, 0.79] : [0.78, 0.84, 0.74],
        ),
        u_signal: [anchor[0] + anchor[2] / 2, anchor[1] + anchor[3] / 2, anchor[2] * 0.39, 1],
        u_panel: panel,
        u_query: rect(queryNode, base),
        u_island: detached ? anchor : [0, 0, 0, 0],
        u_selection: selection.current.to,
        u_previous: selection.current.from,
        u_selectAt: selection.current.at,
        u_selectDuration: selection.current.duration,
        u_clip: viewport,
      });
    }
    const schedule = () => {
      if (!frame) frame = requestAnimationFrame(measure);
    };
    const followLayout = () => {
      resetSelection.current = true;
      schedule();
    };
    const observer = new ResizeObserver(followLayout);
    observer.observe(root);
    root
      .querySelectorAll(".activity-query,.activity-body,.activity-list")
      .forEach((n) => observer.observe(n));
    root.addEventListener("scroll", followLayout, true);
    redraw.current = schedule;
    schedule();
    return () => {
      cancelAnimationFrame(frame);
      observer.disconnect();
      root.removeEventListener("scroll", followLayout, true);
      redraw.current = () => {};
    };
  }, [standalone]);
  return (
    <div ref={host} className="signal-field" aria-hidden="true">
      <NativeShader
        ref={shader}
        fragment={signalFragment}
        uniforms={initialSignalUniforms}
        speed={animate ? 1 : 0}
        active={active}
        onFrame={onFrame}
      />
    </div>
  );
}
