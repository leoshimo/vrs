type FrameJob = { active: boolean; tick: (delta: number) => boolean };
export class FrameClock {
  private jobs = new Set<FrameJob>();
  private frame = 0;
  private last = 0;
  private ticking = false;
  private visible = true;
  private request: (callback: (time: number) => void) => number;
  private cancel: (id: number) => void;
  constructor(request: (callback: (time: number) => void) => number, cancel: (id: number) => void) {
    this.request = request;
    this.cancel = cancel;
  }
  private schedule() {
    if (!this.frame && !this.ticking && this.visible && [...this.jobs].some((job) => job.active))
      this.frame = this.request((time) => this.tick(time));
  }
  private tick(time: number) {
    this.frame = 0;
    this.ticking = true;
    const delta = this.last ? Math.min(64, time - this.last) : 0;
    this.last = time;
    for (const job of this.jobs) if (job.active) job.active = job.tick(delta);
    this.ticking = false;
    this.schedule();
    if (!this.frame) this.last = 0;
  }
  setVisible(visible: boolean) {
    this.visible = visible;
    this.cancel(this.frame);
    this.frame = 0;
    this.last = 0;
    this.schedule();
  }
  task(tick: FrameJob["tick"]) {
    const job = { tick, active: false };
    this.jobs.add(job);
    return {
      wake: () => {
        job.active = true;
        this.schedule();
      },
      sleep: () => {
        job.active = false;
        if (![...this.jobs].some((j) => j.active)) {
          this.cancel(this.frame);
          this.frame = 0;
          this.last = 0;
        }
      },
      dispose: () => {
        this.jobs.delete(job);
        if (![...this.jobs].some((j) => j.active)) {
          this.cancel(this.frame);
          this.frame = 0;
          this.last = 0;
        }
      },
    };
  }
}
let browserClock: FrameClock | undefined;
export function frameTask(tick: FrameJob["tick"]) {
  if (!browserClock) {
    browserClock = new FrameClock(
      (callback) => requestAnimationFrame(callback),
      (id) => cancelAnimationFrame(id),
    );
    const visibility = () => browserClock!.setVisible(!document.hidden);
    document.addEventListener("visibilitychange", visibility);
    visibility();
  }
  return browserClock.task(tick);
}
