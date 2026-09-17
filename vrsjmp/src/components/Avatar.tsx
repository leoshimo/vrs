import { SignalField } from "../avatar/SignalField";
import type { SignalConfig } from "../avatar/signal-field";
import type { MotionMode } from "../avatar/signal-motion";

export function Avatar({
  config,
  mode,
  pulse,
  working,
  active,
  dark,
  reduced,
}: {
  config: SignalConfig;
  mode: MotionMode;
  pulse: number;
  working: boolean;
  active: boolean;
  dark: boolean;
  reduced: boolean;
}) {
  return (
    <span className="avatar" aria-hidden="true">
      <span className="avatar-object">
        <span className="signal-anchor" />
        <SignalField
          config={{
            ...config,
            boundary: "none",
            dispatchTarget: "avatar",
            effects: { ...config.effects, input: false, selection: false, container: false },
          }}
          state={mode}
          pulse={pulse}
          working={working}
          selected="avatar"
          active={active}
          animate={!reduced}
          dark={dark}
          standalone
        />
      </span>
    </span>
  );
}
