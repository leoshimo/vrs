import { useCallback, useLayoutEffect, useState } from "react";
import { AvatarSession } from "../avatar/avatar-session";

export function useAvatarEvents(reduced: boolean, visible: boolean, working: boolean) {
  const [session] = useState(() => new AvatarSession());
  const [revision, setRevision] = useState(0);
  useLayoutEffect(() => {
    session.reduced = reduced;
    const changed = session.player.working !== working;
    session.working(working);
    if (!visible) session.hide(performance.now());
    if (changed) setRevision((n) => n + 1);
  }, [session, reduced, visible, working]);
  const open = useCallback(() => {
    session.open(performance.now());
    setRevision((n) => n + 1);
  }, [session]);
  return {
    player: session.player,
    revision,
    open,
    type: () => {
      session.type();
      setRevision((n) => n + 1);
    },
    submit: () => {
      session.submit();
      setRevision((n) => n + 1);
    },
  };
}
