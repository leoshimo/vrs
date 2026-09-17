import { defaultSignal, type SignalConfig } from './signal-field';
import { expressionSettings, type Expression } from './avatar-expressions';

export type ExpressionOverride = {
  settings?: Partial<SignalConfig>;
  duration?: number;
};
export type ExpressionOverrides = Record<string, ExpressionOverride>;

export function resolveExpression(
  base: SignalConfig,
  expression: Expression,
  override?: ExpressionOverride,
) {
  const defaults = expressionSettings(expression.action, {
    ...defaultSignal,
    ...expression.settings,
  });
  // Only the selected action's keys can override appearance or another action.
  const settings = expressionSettings(expression.action, {
    ...defaultSignal,
    ...defaults,
    ...override?.settings,
  });
  return {
    config: { ...base, ...settings },
    duration: override?.duration ?? expression.duration ?? 1.4,
  };
}

export function editExpression(
  expression: Expression,
  prior: ExpressionOverride | undefined,
  patch: ExpressionOverride,
): ExpressionOverride {
  const defaults = expressionSettings(expression.action, {
    ...defaultSignal,
    ...expression.settings,
  });
  const values = expressionSettings(expression.action, {
    ...defaultSignal,
    ...defaults,
    ...prior?.settings,
    ...patch.settings,
  });
  const settings = Object.fromEntries(
    Object.entries(values).filter(
      ([key, value]) => value !== defaults[key as keyof SignalConfig],
    ),
  );
  const duration = patch.duration ?? prior?.duration;
  return {
    settings,
    ...(duration !== undefined && duration !== (expression.duration ?? 1.4)
      ? { duration }
      : {}),
  };
}
export function isModified(override?: ExpressionOverride) {
  return (
    !!override &&
    (Object.keys(override.settings ?? {}).length > 0 ||
      override.duration !== undefined)
  );
}
