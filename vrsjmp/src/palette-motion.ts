export function animatePalette(node: HTMLElement | null, appearing: boolean, reduced: boolean) {
  if (!node) return { finished: Promise.resolve(), cancel() {} };
  const style = getComputedStyle(node);
  const from = { opacity: style.opacity, transform: style.transform };
  const to = {
    opacity: appearing ? "1" : "0",
    transform: reduced || appearing ? "scale(1)" : "scale(.995)",
  };
  node.dataset.presence = appearing ? "opening" : "closing";
  const animation = node.animate([from, to], {
    duration: reduced ? 0 : appearing ? 140 : 90,
    easing: appearing ? "cubic-bezier(.16,1,.3,1)" : "cubic-bezier(.4,0,1,1)",
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
    cancel() {
      if (animation.playState === "idle") return;
      const current = getComputedStyle(node);
      node.style.opacity = current.opacity;
      node.style.transform = current.transform;
      animation.cancel();
    },
  };
}
