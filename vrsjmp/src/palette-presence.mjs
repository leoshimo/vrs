// Serialize native visibility changes; an interrupted exit must never hide a
// window that has since reopened.
export class PalettePresence {
  constructor({ show, hide, prepare, animate, onHidden }) {
    this.showNative = show;
    this.hideNative = hide;
    this.prepare = prepare;
    this.animate = animate;
    this.onHidden = onHidden;
    this.visible = false;
    this.revision = 0;
    this.native = Promise.resolve();
    this.animation = null;
  }
  enqueue(action, revision) {
    const operation = this.native.then(() => {
      if (revision === this.revision) return action();
    });
    this.native = operation.catch(() => {});
    return operation;
  }
  async show() {
    if (this.visible) {
      await this.enqueue(this.showNative, this.revision);
      return;
    }
    const revision = ++this.revision;
    this.visible = true;
    this.animation?.cancel();
    this.prepare();
    await this.enqueue(this.showNative, revision);
    if (revision === this.revision) this.animation = this.animate(true);
  }
  async hide(reason = "dismiss") {
    if (!this.visible) return;
    const revision = ++this.revision;
    this.visible = false;
    this.animation?.cancel();
    const animation = this.animate(false, reason);
    this.animation = animation;
    await animation.finished;
    await this.enqueue(this.hideNative, revision);
    if (revision === this.revision) this.onHidden?.();
  }
  dispose() {
    this.revision++;
    this.visible = false;
    this.animation?.cancel();
  }
}

export function presentedSnapshot(current, previous) {
  if (!previous) return current;
  // Keep the last rendered contents until the exit animation is over.
  if (!current.visible) return { ...previous, visible: false };
  // A new page may not have rows yet. Retain the old page, inactive, until the
  // response arrives instead of shrinking to a loading label and growing again.
  if (current.loading && !current.items.length && previous.visible && previous.items.length) {
    return { ...previous, query: current.query, loading: true, visible: true };
  }
  return current;
}
