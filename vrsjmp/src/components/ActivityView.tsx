import { Component, createRef, type ReactNode } from "react";

type Props = {
  children: ReactNode;
  pageKey: string;
  active: boolean;
  reduced: boolean;
};
type BeforeUpdate = { height: number; outgoing: HTMLElement | null; scrollTops: number[] };

export class ActivityView extends Component<Props> {
  private surface = createRef<HTMLElement>();
  private page = createRef<HTMLDivElement>();
  private outgoing = createRef<HTMLDivElement>();
  private resize?: ResizeObserver;
  private heightAnimation?: Animation;
  private fades: Animation[] = [];
  private targetHeight = 0;

  componentDidMount() {
    this.targetHeight = this.page.current!.offsetHeight;
    this.updateOverflow();
    this.resize = new ResizeObserver(() => {
      this.resizeToPage();
      this.updateOverflow();
    });
    this.resize.observe(this.page.current!);
  }

  getSnapshotBeforeUpdate(previous: Props): BeforeUpdate {
    let outgoing: HTMLElement | null = null;
    let scrollTops: number[] = [];
    if (previous.pageKey !== this.props.pageKey && previous.active && this.props.active && !this.props.reduced) {
      outgoing = this.page.current!.cloneNode(true) as HTMLElement;
      outgoing.style.opacity = getComputedStyle(this.page.current!).opacity;
      // The outgoing page is a visual snapshot. Only the new page is interactive
      // or exposed to assistive technology.
      outgoing.removeAttribute("id");
      outgoing.querySelectorAll("[id]").forEach((node) => node.removeAttribute("id"));
      scrollTops = Array.from(this.page.current!.querySelectorAll(".activity-list"), (node) => node.scrollTop);
    }
    return { height: parseFloat(getComputedStyle(this.surface.current!).height), outgoing, scrollTops };
  }

  componentDidUpdate(_previous: Props, _state: unknown, before: BeforeUpdate) {
    if (this.props.reduced) this.clearFades();
    if (before.outgoing) {
      this.clearFades();
      this.outgoing.current!.append(before.outgoing);
      before.outgoing.querySelectorAll(".activity-list").forEach((node, index) => {
        node.scrollTop = before.scrollTops[index];
      });
      const options = { duration: 180, fill: "both" as const };
      const exit = before.outgoing.animate([
        { opacity: before.outgoing.style.opacity }, { opacity: 0, offset: .35 }, { opacity: 0 },
      ], options);
      const enter = this.page.current!.animate([
        { opacity: 0 }, { opacity: 0, offset: .35 }, { opacity: 1 },
      ], options);
      this.fades = [exit, enter];
      void enter.finished.then(() => {
        if (this.fades.includes(enter)) this.clearFades();
      }, () => {});
    }
    this.resizeToPage(before.height);
    this.updateOverflow();
  }

  private updateOverflow = () => {
    const list = this.page.current?.querySelector<HTMLElement>(".activity-list");
    if (!list) return;
    const remaining = list.scrollHeight - list.clientHeight - list.scrollTop;
    const bottomInset = parseFloat(getComputedStyle(list).paddingBottom);
    list.dataset.moreBelow = String(remaining > bottomInset + 1);
  };

  private resizeToPage(from?: number) {
    const surface = this.surface.current!;
    const height = this.page.current!.offsetHeight;
    if (this.props.reduced || !this.props.active) {
      this.targetHeight = height;
      // On dismiss, let an in-progress resize continue through the window fade.
      if (this.props.reduced) this.heightAnimation?.cancel();
      return;
    }
    if (height === this.targetHeight) return;
    from ??= parseFloat(getComputedStyle(surface).height);
    this.targetHeight = height;
    this.heightAnimation?.cancel();
    if (Math.abs(from - height) < 1) return;
    const animation = surface.animate([{ height: `${from}px` }, { height: `${height}px` }], {
      duration: 180, easing: "cubic-bezier(.2,0,0,1)",
    });
    this.heightAnimation = animation;
    void animation.finished.then(() => {
      if (this.heightAnimation === animation) this.heightAnimation = undefined;
      animation.cancel();
    }, () => {});
  }

  private clearFades() {
    this.fades.forEach((animation) => animation.cancel());
    this.fades = [];
    this.outgoing.current?.replaceChildren();
  }

  componentWillUnmount() {
    this.resize?.disconnect();
    this.heightAnimation?.cancel();
    this.clearFades();
  }

  render() {
    return (
      <section ref={this.surface} className="activity-view surface" aria-label="Activity view" onScrollCapture={this.updateOverflow}>
        <div ref={this.page} className="activity-page" data-page={this.props.pageKey}>
          {this.props.children}
        </div>
        <div ref={this.outgoing} className="activity-outgoing" aria-hidden="true" inert />
      </section>
    );
  }
}
