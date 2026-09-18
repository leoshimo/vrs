import type { Assignment, AvatarExpression } from './workspace';
const order = {
  idle: ['Flow', 'Shape', 'Light', 'Marks', 'Reveal'],
  working: ['Flow', 'Shape', 'Light', 'Marks', 'Reveal'],
  typing: ['Flow', 'Shape', 'Light', 'Marks', 'Reveal'],
  submit: ['Marks', 'Shape', 'Light', 'Flow', 'Reveal'],
  open: ['Reveal', 'Light', 'Shape', 'Marks', 'Flow'],
  openAfterIdle: ['Reveal', 'Light', 'Shape', 'Marks', 'Flow'],
};
function group(expression: AvatarExpression) {
  const kinds = expression.patterns.map((p) => p.kind);
  if (kinds.some((k) => k === 'printed' || k === 'fill')) return 'Reveal';
  if (
    kinds.some((k) =>
      [
        'nudge',
        'lava',
        'drift',
        'eddies',
        'stir',
        'swirl',
        'orbit',
        'sweep',
        'ruffle',
      ].includes(k),
    )
  )
    return 'Flow';
  if (kinds.some((k) => ['pressure', 'ripple', 'gather'].includes(k)))
    return 'Shape';
  if (kinds.some((k) => k === 'tilt' || k === 'bloom')) return 'Light';
  return 'Marks';
}
export function expressionGroups(
  expressions: AvatarExpression[],
  assignment: Assignment,
) {
  return order[assignment]
    .map((name) => ({
      name,
      expressions: expressions.filter((e) => group(e) === name),
    }))
    .filter((g) => g.expressions.length);
}
