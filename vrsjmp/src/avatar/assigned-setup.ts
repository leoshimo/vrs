import saved from "./assigned-setup.json" with { type: "json" };
import type { AvatarSetup } from "./expression-model";

// Vite includes this data in the application bundle.
export function assignedSetup(): AvatarSetup {
  return structuredClone(saved) as AvatarSetup;
}
