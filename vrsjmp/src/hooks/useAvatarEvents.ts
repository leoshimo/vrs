import { useCallback, useEffect, useRef, useState } from "react";
import type { MotionMode } from "../avatar/signal-motion";
import { eventConfig, expression, type EventName, type Setup } from "../avatar/setup";
import preset from "../avatar/preset.json";

const setup = preset as unknown as Setup;
export function useAvatarEvents(reduced: boolean) {
  const [motion, setMotion] = useState<{ mode: MotionMode; pulse: number }>({
    mode: "idle",
    pulse: 0,
  });
  const [transient, setTransient] = useState<EventName | null>(null);
  const [entrance, setEntrance] = useState({ serial: 0, flourish: false });
  const [progress, setProgress] = useState(1);
  const timer = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);
  const lastOpen = useRef(-Infinity);
  const trigger = useCallback((event: EventName) => {
    clearTimeout(timer.current);
    const profile = expression(setup, event);
    const mode = profile.motions[0]?.action ?? "idle";
    setTransient(event);
    setMotion((old) => ({ mode: mode === "wake" ? "idle" : mode, pulse: old.pulse + 1 }));
    timer.current = setTimeout(
      () => {
        setMotion((old) => ({ ...old, mode: "idle" }));
        setTransient(null);
      },
      event === "typing"
        ? 80
        : Math.max(
            1200,
            ...profile.motions.map((m) => (m.settings.surfaceDuration ?? m.duration) * 1000 + 800),
          ),
    );
  }, []);
  const open = useCallback(() => {
    const now = performance.now();
    const flourish = now - lastOpen.current >= 30_000;
    lastOpen.current = now;
    setProgress(reduced ? 1 : 0);
    setEntrance((old) => ({ serial: old.serial + 1, flourish }));
    trigger(flourish ? "flourish" : "open");
  }, [trigger, reduced]);
  useEffect(() => {
    if (!entrance.serial || reduced) {
      setProgress(1);
      return;
    }
    let frame = 0;
    const start = performance.now();
    const duration = entrance.flourish ? 680 : 120;
    setProgress(0);
    const tick = (now: number) => {
      const t = Math.min(1, (now - start) / duration);
      setProgress(t);
      if (t < 1) frame = requestAnimationFrame(tick);
    };
    frame = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(frame);
  }, [entrance, reduced]);
  useEffect(() => () => clearTimeout(timer.current), []);
  return {
    ...motion,
    config: eventConfig(setup, transient),
    progress,
    flourish: entrance.flourish,
    open,
    type: () => trigger("typing"),
    submit: () => trigger("submit"),
  };
}
