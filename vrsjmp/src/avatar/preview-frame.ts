import type { ExpressionPlayer } from "./expression-player";
export type PreviewState = {
  live: boolean;
  paused: boolean;
  visible: boolean;
  dirty: boolean;
  trackTime: boolean;
};
export function advancePreview(player: ExpressionPlayer, state: PreviewState, delta: number) {
  const running = state.live && !state.paused;
  const animating = running && player.needsAnimation();
  if (running) player.advance(delta);
  return {
    draw: state.visible && (state.dirty || animating),
    keepAlive: running && (player.needsAnimation() || (state.visible && state.trackTime)),
  };
}
