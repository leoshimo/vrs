import { useLayoutEffect, useRef, type RefObject } from "react";
import { registerPreview } from "../avatar/preview-renderer";
import { applyPlaceholderAccent } from "../avatar/preview-colors";
import type { ExpressionPlayer } from "../avatar/expression-player";

export function Avatar({
  player,
  revision,
  active,
  dark,
  reduced,
  inputRef,
}: {
  player: ExpressionPlayer;
  revision: number;
  active: boolean;
  dark: boolean;
  reduced: boolean;
  inputRef: RefObject<HTMLInputElement | null>;
}) {
  const canvas = useRef<HTMLCanvasElement>(null);
  const renderer = useRef<ReturnType<typeof registerPreview> | null>(null);
  useLayoutEffect(() => {
    renderer.current = registerPreview(canvas.current!, player);
    return () => renderer.current?.dispose();
  }, [player]);
  useLayoutEffect(() => {
    renderer.current?.update({
      dark,
      live: active && !reduced,
      onFrame: (p, paint) => {
        if (paint && inputRef.current) applyPlaceholderAccent(inputRef.current, p);
      },
    });
    renderer.current?.paint();
  }, [player, revision, active, dark, reduced, inputRef]);
  return (
    <span className="avatar" aria-hidden="true">
      <span className="avatar-object">
        <canvas ref={canvas} className="native-shader" />
      </span>
    </span>
  );
}
