import { useLayoutEffect, useRef, useState, type ReactNode } from "react";

// One measured height owns the animation. The native window and Search Bar stay
// fixed, so content changes cannot recenter the window or scale its typography.
export function ActivityView({
  title,
  back,
  showHeading,
  children,
  pageKey,
  reduced,
}: {
  title: string;
  back?: () => void;
  showHeading: boolean;
  children: ReactNode;
  pageKey: string;
  reduced: boolean;
}) {
  const content = useRef<HTMLDivElement>(null);
  const [height, setHeight] = useState<number>();
  useLayoutEffect(() => {
    const node = content.current;
    if (!node) return;
    const measure = () => setHeight(Math.ceil(node.getBoundingClientRect().height));
    measure();
    const observer = new ResizeObserver(measure);
    observer.observe(node);
    return () => observer.disconnect();
  }, []);
  return (
    <section
      className="activity-view surface overflow-hidden rounded-[18px]"
      aria-label="Activity view"
      style={{ height, transition: reduced ? "none" : "height 160ms cubic-bezier(.2,.8,.2,1)" }}
    >
      <div ref={content} className={showHeading ? "" : "pt-[6px]"}>
        {showHeading && (
          <header className="flex h-9 items-center gap-2 px-[18px] text-xs" data-tauri-drag-region>
            {back && (
              <button
                type="button"
                className="back-button -ml-1 grid size-6 shrink-0 place-items-center rounded-md"
                onClick={back}
                aria-label="Back (Escape)"
              >
                ←
              </button>
            )}
            <h1 className="min-w-0 truncate font-medium">{title}</h1>
          </header>
        )}
        <div key={pageKey} className="activity-page" data-page={pageKey}>
          {children}
        </div>
      </div>
    </section>
  );
}
