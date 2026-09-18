import { ExpressionPlayer, type PlaybackAction } from "./expression-player";
import { assignedSetup } from "./assigned-setup";

export class AvatarSession {
  readonly player = new ExpressionPlayer(assignedSetup());
  private lastHidden = -Infinity;
  private visible = false;
  reduced = false;
  constructor() {
    this.player.visible = false;
  }
  private trigger(action: PlaybackAction) {
    this.player.trigger(action);
    if (this.reduced && action !== "hide")
      for (let i = 0; i < 120; i++) this.player.advance(1 / 120);
  }
  open(now: number) {
    if (this.visible) return;
    this.visible = true;
    this.trigger(now - this.lastHidden >= 30_000 ? "openAfterIdle" : "open");
  }
  hide(now: number) {
    if (!this.visible) return;
    this.visible = false;
    this.lastHidden = now;
    this.trigger("hide");
  }
  working(value: boolean) {
    if (this.player.working !== value) this.trigger(value ? "working" : "idle");
  }
  type() {
    if (this.visible) this.trigger("typing");
  }
  submit() {
    if (this.visible) this.trigger("submit");
  }
}
