import { useEffect, useState } from "react";
export function useAppearance() {
  const [dark, setDark] = useState(() => matchMedia("(prefers-color-scheme: dark)").matches);
  const [reduced, setReduced] = useState(
    () => matchMedia("(prefers-reduced-motion: reduce)").matches,
  );
  useEffect(() => {
    const appearance = matchMedia("(prefers-color-scheme: dark)");
    const motion = matchMedia("(prefers-reduced-motion: reduce)");
    const updateAppearance = () => setDark(appearance.matches);
    const updateMotion = () => setReduced(motion.matches);
    appearance.addEventListener("change", updateAppearance);
    motion.addEventListener("change", updateMotion);
    return () => {
      appearance.removeEventListener("change", updateAppearance);
      motion.removeEventListener("change", updateMotion);
    };
  }, []);
  return { dark, reduced };
}
