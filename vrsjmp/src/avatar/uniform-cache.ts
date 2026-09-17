export type UniformValue = number | number[];
/** Copies arrays so callers can reuse and mutate their upload buffer safely. */
export function changedUniform(cache: Map<string, UniformValue>, key: string, value: UniformValue) {
  const old = cache.get(key);
  if (
    typeof value === "number"
      ? old === value
      : Array.isArray(old) && old.length === value.length && value.every((n, i) => n === old[i])
  )
    return false;
  cache.set(key, typeof value === "number" ? value : [...value]);
  return true;
}
