import type { Assignment, AvatarExpression } from './workspace';
export type Comparison = {
  key: string;
  input: Assignment;
  expression: AvatarExpression | null;
};
export function comparisonSetup<
  T extends { assignments: Record<Assignment, AvatarExpression | null> },
>(setup: T, input: Assignment, expression?: AvatarExpression): T {
  return {
    ...setup,
    assignments: { ...setup.assignments, [input]: expression ?? null },
  };
}
