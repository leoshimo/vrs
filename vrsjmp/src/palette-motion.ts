const completionHoldMs = 150;
const completionFadeMs = 300;
export const completionExitMs = completionHoldMs + completionFadeMs;

export function animatePalette(
  node: HTMLElement | null,
  appearing: boolean,
  reduced: boolean,
  reason: "dismiss" | "complete" = "dismiss",
) {
  if (!node) return { finished: Promise.resolve(), cancel() {}, pause() {}, play() {} };
  const style = getComputedStyle(node);
  const from = { opacity: style.opacity, transform: style.transform };
  const complete = !appearing && reason === "complete";
  const to = {
    opacity: appearing ? "1" : "0",
    transform: complete ? from.transform : reduced || appearing ? "scale(1)" : "scale(.995)",
  };
  node.dataset.presence = appearing ? "opening" : "closing";
  const frames = complete
    ? [from, { ...from, offset: completionHoldMs / completionExitMs, easing: "cubic-bezier(.3,0,.4,1)" }, to]
    : [from, to];
  const animation = node.animate(frames, {
    duration: reduced ? 0 : complete ? completionExitMs : appearing ? 140 : 90,
    easing: complete ? "linear" : appearing ? "cubic-bezier(.16,1,.3,1)" : "cubic-bezier(.4,0,1,1)",
    fill: "forwards",
  });
  const finished = animation.finished.then(() => {
    Object.assign(node.style, to);
    node.style.visibility = appearing ? "visible" : "hidden";
    node.dataset.presence = appearing ? "shown" : "hidden";
    animation.cancel();
  }, () => {});
  return {
    finished,
    pause() { animation.pause(); },
    play() { if (animation.playState === "paused") animation.play(); },
    cancel() {
      if (animation.playState === "idle") return;
      const current = getComputedStyle(node);
      node.style.opacity = current.opacity;
      node.style.transform = current.transform;
      animation.cancel();
    },
  };
}
