import { assignedSetup } from '../../../../vrsjmp/src/avatar/assigned-setup';
import { makeLabSetup, readLabDraft } from './lab-setup';
import { expressions as presets } from './avatar-expressions';
import { effects, expressionEffect, presetName } from './avatar-effects';
import type { Setup } from './setup';
import {
  defaultColor,
  assignmentNames,
  type AvatarExpression,
  type AvatarSetup,
  type MotionKind,
  type Pattern,
} from '../../../../vrsjmp/src/avatar/expression-model';
export * from '../../../../vrsjmp/src/avatar/expression-model';

const kinds: Record<string, MotionKind> = {
  agitation: 'stir',
  'agitation-kick': 'ruffle',
  light: 'tilt',
  eddy: 'eddies',
  wave: 'sweep',
  surface: 'ripple',
  ripple: 'echo',
  print: 'printed',
};
function convert(old: Setup): AvatarSetup {
  const entries: AvatarExpression[] = old.expressions
    .filter((e) => e.motions.length)
    .map((e) => {
      const patterns: Pattern[] = e.motions.map((m) => {
        const effect = expressionEffect(m);
        const kind =
          effect === 'entrance'
            ? m.settings.appearanceMode === 'fill'
              ? 'fill'
              : 'printed'
            : (kinds[effect] ?? (effect as MotionKind));
        return { kind, settings: { ...m.settings } };
      });
      for (const p of patterns.slice()) {
        if (p.settings.entranceRipple)
          patterns.push({
            kind: 'ripple',
            settings: { ...p.settings, surfaceDuration: 0.3 },
          });
        if (p.settings.typingSparks)
          patterns.push({
            kind: 'sparks',
            settings: { sparkCount: 4, dispatchEnergy: 0.45 },
          });
      }
      const first = e.motions[0],
        effect = expressionEffect(first);
      const preset = presets.find((p) => p.id === e.id);
      const group =
        effect === 'entrance'
          ? patterns[0].kind === 'fill'
            ? 'Fill'
            : 'Printed'
          : effects[effect].name;
      let name = e.name;
      if (
        preset &&
        e.name === preset.name &&
        e.motions.length === 1 &&
        effect !== 'entrance'
      ) {
        const variant = presetName(preset);
        name =
          group === variant || group === e.name
            ? group
            : `${group} — ${variant}`;
      }
      if (e.id.startsWith('pressure-')) name = `Pressure — ${e.name}`;
      if (e.id === 'nudge-sparks') name = 'Nudge + Sparks';
      if (e.id === 'opal-nudge') name = 'Nudge — Opal';
      if (e.id === 'ember-sparks') name = 'Sparks — Ember';
      if (e.id === 'print-wake') name = 'Printed — Slow';
      if (e.id === 'fill-wake') name = 'Fill — Original';
      const c = first.settings;
      for (const p of patterns)
        for (const key of [
          'activityColor',
          'color',
          'colorTiming',
          'colorStrength',
          'saturation',
          'activitySaturation',
        ] as const)
          delete p.settings[key];
      return {
        id: e.id,
        name,
        description:
          e.motions.length > 1
            ? e.motions
                .map((m) => effects[expressionEffect(m)].name)
                .join(' + ')
            : (preset?.note ?? effects[effect].note),
        patterns,
        duration:
          first.action === 'wake'
            ? first.duration === 0.42 || first.duration === 0.62
              ? 0.3
              : first.duration
            : first.action === 'typing'
              ? 0.24
              : 0.3,
        color: {
          ...defaultColor,
          mode:
            c.activityColor && c.activityColor !== 'mono'
              ? 'custom'
              : patterns[0].kind === 'lava' || patterns[0].kind === 'swirl'
                ? 'none'
                : 'accent',
          palette: c.activityColor ?? 'opal',
          strength: c.activityColor ? 0.4 : 0.3,
        },
        custom: e.id.startsWith('custom-'),
      };
    });
  const alias: Record<string, string | null> = { 'quick-open': null };
  // Identical old reveal presets collapse; preserve independently tuned versions.
  const slow = entries.find((e) => e.id === 'print-wake'),
    printed = entries.find((e) => e.id === 'entrance-print');
  if (slow && printed && slow.duration === printed.duration) {
    entries.splice(entries.indexOf(slow), 1);
    alias['print-wake'] = printed.id;
  }
  const byId = (id: string) => (id in alias ? alias[id] : id);
  const result: AvatarSetup = {
    version: 2,
    appearance: old.appearance,
    expressions: entries,
    assignments: {
      idle: byId(old.events.idle),
      typing: byId(old.events.typing),
      working: byId(old.events.working),
      open: byId(old.events.open),
      openAfterIdle: byId(old.events.flourish),
      submit: byId(old.events.submit),
    },
  };
  const addVariant = (
    source: string,
    id: string,
    name: string,
    patch: Partial<AvatarExpression>,
  ) => {
    const e = entries.find((e) => e.id === source);
    if (e && !entries.some((e) => e.id === id))
      entries.push({ ...structuredClone(e), id, name, ...patch });
  };
  addVariant('field-nudge', 'nudge-momentum', 'Nudge — Momentum', {
    momentum: true,
    description: 'Taps build speed; the field coasts between them.',
  });
  addVariant('swirl', 'swirl-flow', 'Swirl — Flow', {
    distortionOnly: true,
    description: 'A loose current carries the existing field around the rim.',
  });
  addVariant('orbit', 'orbit-flow', 'Orbit — Flow', {
    distortionOnly: true,
    description:
      'A regular traveling distortion carries the field around the rim.',
  });
  return result;
}
const collection: readonly (readonly [string, string])[] = [
  ['rest', 'Lava'],
  ['field-nudge', 'Nudge'],
  ['nudge-momentum', 'Momentum'],
  ['pressure-dimple', 'Dimple'],
  ['light', 'Tilt'],
  ['agitation-kick', 'Ruffle'],
  ['drift', 'Drift'],
  ['eddies', 'Eddies'],
  ['restless', 'Stir'],
  ['swirl', 'Swirl'],
  ['orbit', 'Orbit'],
  ['alternating', 'Sweep'],
  ['surface-react', 'Ripple'],
  ['gather', 'Gather'],
  ['bloom', 'Bloom'],
  ['sparks', 'Sparks'],
  ['echo', 'Echo'],
  ['fill-wake', 'Fill'],
  ['entrance-print', 'Printed'],
  ['printed-open', 'Printed — Subtle'],
  ['working', 'Stir + Ripple'],
  ['entrance-print-ripple', 'Printed + Ripple'],
];
export const proposedAssignments: AvatarSetup['assignments'] = {
  idle: 'rest',
  typing: 'field-nudge',
  working: 'working',
  open: 'printed-open',
  openAfterIdle: 'entrance-print-ripple',
  submit: 'sparks',
};
export function simplifyWorkspace(setup: AvatarSetup): AvatarSetup {
  const defaults = [...assignedSetup().expressions, ...convert(makeLabSetup()).expressions];
  const migrateOpen = !setup.expressions.some((e) => e.id === 'printed-open');
  const expressions = collection.map(([id, name]) => {
    const source =
      setup.expressions.find((e) => e.id === id) ??
      defaults.find((e) => e.id === id)!;
    return {
      ...source,
      name,
      custom: false,
      ...(id === 'working'
        ? { description: 'Irregular flow with a repeating surface wave.' }
        : {}),
      ...(id === 'field-nudge'
        ? { momentum: false }
        : id === 'nudge-momentum'
          ? { momentum: true }
          : {}),
    };
  });
  return {
    ...setup,
    expressions,
    assignments: Object.fromEntries(
      assignmentNames.map((a) => {
        const id = migrateOpen && a === 'open' && setup.assignments[a] === 'bloom'
          ? 'printed-open' : setup.assignments[a];
        return [
          a,
          id === null || expressions.some((e) => e.id === id)
            ? id
            : proposedAssignments[a],
        ];
      }),
    ) as AvatarSetup['assignments'],
  };
}
export function loadProposal(setup: AvatarSetup): AvatarSetup {
  return {
    ...simplifyWorkspace(setup),
    assignments: { ...proposedAssignments },
  };
}
export function makeWorkspace() {
  const library = workingAccent(loadProposal(convert(makeLabSetup())));
  const native = assignedSetup();
  return {
    ...library,
    appearance: native.appearance,
    assignments: native.assignments,
    expressions: library.expressions.map((expression) => {
      const chosen = native.expressions.find((e) => e.id === expression.id);
      return chosen
        ? { ...expression, ...chosen, description: expression.description }
        : expression;
    }),
  };
}
function workingAccent(setup: AvatarSetup): AvatarSetup {
  return {
    ...setup,
    expressions: setup.expressions.map((e) => ({
      ...e,
      color: {
        ...e.color,
        mode: e.id === setup.assignments.working ? 'accent' : 'none',
      },
    })),
  };
}
function parseWorkspace(raw: string | null): AvatarSetup | undefined {
  if (!raw) return;
  try {
    const s = JSON.parse(raw) as AvatarSetup;
    if (
      s.version === 2 &&
      s.appearance &&
      Array.isArray(s.expressions) &&
      s.assignments &&
      Object.values(s.assignments).every(
        (id) => id === null || s.expressions.some((e) => e.id === id),
      ) &&
      s.expressions.every(
        (e) => e.patterns?.length && e.color && Number.isFinite(e.duration),
      )
    )
      return s;
  } catch {
    /* Use the previous draft or code defaults. */
  }
}
export function readWorkspace(
  raw: string | null,
  legacy: string | null = null,
  previous: string | null = null,
  priorProposal: string | null = null,
): AvatarSetup {
  const saved = parseWorkspace(raw);
  if (saved) return simplifyWorkspace(saved);
  const proposal = parseWorkspace(priorProposal);
  if (proposal) return workingAccent(simplifyWorkspace(proposal));
  const prior = parseWorkspace(previous);
  if (prior) return workingAccent(loadProposal(prior));
  return legacy
    ? workingAccent(simplifyWorkspace(convert(readLabDraft(legacy))))
    : makeWorkspace();
}
export const workspaceKey = 'avatar-lab-workspace-v4';
