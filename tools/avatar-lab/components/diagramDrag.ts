import type { PointerEvent } from 'react';
export function dragDiagram(onPoint?: (x: number, y: number) => void) {
  function pick(event: PointerEvent<SVGSVGElement>) {
    const matrix = event.currentTarget.getScreenCTM();
    if (!matrix || !onPoint) return;
    const p = new DOMPoint(event.clientX, event.clientY).matrixTransform(
      matrix.inverse(),
    );
    onPoint(p.x, p.y);
  }
  return {
    'data-draggable': !!onPoint,
    onPointerDown: (e: PointerEvent<SVGSVGElement>) => {
      e.currentTarget.setPointerCapture(e.pointerId);
      pick(e);
    },
    onPointerMove: (e: PointerEvent<SVGSVGElement>) => {
      if (e.buttons) pick(e);
    },
  };
}
export const clamp = (v: number, a = 0, b = 1) => Math.max(a, Math.min(b, v));
export function radialPoint(x: number, y: number, r: number): [number, number] {
  const old = Math.hypot(x, y);
  return old < 0.00001 ? [r, 0] : [(x * r) / old, (y * r) / old];
}
