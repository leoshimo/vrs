'use client';
/* oxlint-disable jsx-a11y/prefer-tag-over-role -- Canvas is the rendered image. */
import {
  useLayoutEffect,
  useEffectEvent,
  useMemo,
  useRef,
  useState,
  type ReactNode,
  type RefObject,
} from 'react';
import { useMediaQuery } from './useMediaQuery';
import { registerPreview } from '@/lib/avatar/preview-renderer';
import {
  ExpressionPlayer,
  type PlaybackAction,
} from '../../../vrsjmp/src/avatar/expression-player';
import { type Assignment, type AvatarExpression } from '@/lib/avatar/workspace';
import type { AvatarSetup } from '../../../vrsjmp/src/avatar/expression-model';
import { applyPlaceholderAccent } from '@/lib/avatar/preview-colors';
import { comparisonSetup } from '@/lib/avatar/comparison';
import { InputCurves } from './InputCurves';
import { SignalTimeline } from '@/lib/avatar/signal-timeline';
import { animatePalette } from '../../../vrsjmp/src/palette-motion';

export type PreviewCommand = {
  serial: number;
  events: {
    serial: number;
    action: PlaybackAction | 'reset' | 'resetHidden';
  }[];
};
export function ExpressionPreview({
  setup,
  reference,
  candidate,
  assignment,
  command,
  dark,
  size = 96,
  zoom = 1,
  live = false,
  bar = false,
  query = '',
  onQueryChange,
  onAction,
  paused = false,
  curves = false,
  working,
  shown,
  engine,
  caption,
  overlay,
  status,
  onSample,
  onInspect,
  label = 'Avatar preview',
}: {
  setup: AvatarSetup;
  reference?: RefObject<ExpressionPlayer | null>;
  candidate?: AvatarExpression;
  assignment?: Assignment;
  command?: PreviewCommand;
  dark: boolean;
  size?: number;
  zoom?: number;
  live?: boolean;
  bar?: boolean;
  query?: string;
  onQueryChange?: (s: string) => void;
  onAction?: (a: PlaybackAction) => void;
  paused?: boolean;
  curves?: boolean;
  working?: boolean;
  shown?: boolean;
  engine?: ExpressionPlayer;
  caption?: ReactNode;
  overlay?: ReactNode;
  status?: string;
  onSample?: (player: ExpressionPlayer, paint: boolean) => void;
  onInspect?: () => void;
  label?: string;
}) {
  const canvas = useRef<HTMLCanvasElement>(null);
  const stage = useRef<HTMLDivElement>(null);
  const transition = useRef<ReturnType<typeof animatePalette> | null>(null);
  const adjusted = useMemo(
    () => (assignment ? comparisonSetup(setup, assignment, candidate) : setup),
    [setup, candidate, assignment],
  );
  const player = useRef<ExpressionPlayer | null>(null);
  player.current ??= engine ?? new ExpressionPlayer(adjusted);
  const renderer = useRef<ReturnType<typeof registerPreview> | null>(null);
  const reduce = useMediaQuery('(prefers-reduced-motion: reduce)');
  const input = useRef<HTMLInputElement>(null);
  const [timeline] = useState(() => new SignalTimeline());
  const onFrame = (p: ExpressionPlayer, paint: boolean) => {
    if (curves) timeline.record(p, paint);
    onSample?.(p, paint);
    if (bar && paint && input.current) applyPlaceholderAccent(input.current, p);
  };
  const commandSerial = useRef<number | undefined>(command?.serial);
  const notifySample = useEffectEvent(() => onSample?.(player.current!, false));
  const initialize = useEffectEvent(() => {
    notifySample();
    if (shown === false && stage.current) {
      stage.current.style.opacity = '0';
      stage.current.style.visibility = 'hidden';
      stage.current.dataset.presence = 'hidden';
    }
  });
  useLayoutEffect(() => {
    initialize();
    renderer.current = registerPreview(canvas.current!, player.current!);
    return () => {
      transition.current?.cancel();
      renderer.current?.dispose();
    };
  }, []);
  const synchronize = useEffectEvent(() => {
    if (!engine && reference?.current) {
      player.current = reference.current.fork(adjusted);
      renderer.current?.update({ player: player.current });
    }
  });
  // Keep comparisons at the same pose when an assignment is applied or selected.
  useLayoutEffect(() => {
    synchronize();
  }, [setup.assignments, assignment, candidate?.id]);
  useLayoutEffect(() => {
    if (engine && player.current !== engine) {
      player.current = engine;
      renderer.current?.update({ player: engine });
    }
    player.current!.setup = adjusted;
    if (working !== undefined) player.current!.working = working;
    if (shown !== undefined) player.current!.visible = shown;
    renderer.current?.update({
      dark,
      resolution: size,
      live: live && !reduce && shown !== false,
      paused,
      onFrame,
      trackTime: curves || Boolean(onSample),
    });
  });
  useLayoutEffect(() => {
    if (!command || commandSerial.current === command.serial) return;
    for (const event of command.events.filter(
      (e) => e.serial > (commandSerial.current ?? -1),
    )) {
      if (event.action === 'reset' || event.action === 'resetHidden') {
        player.current = new ExpressionPlayer(adjusted);
        if (event.action === 'resetHidden') player.current.trigger('hide');
        renderer.current?.update({ player: player.current });
      } else player.current!.trigger(event.action);
      if (stage.current) {
        if (event.action === 'resetHidden') {
          transition.current?.cancel();
          stage.current.style.opacity = '0';
          stage.current.style.visibility = 'hidden';
          stage.current.dataset.presence = 'hidden';
        } else if (event.action === 'hide') {
          transition.current?.cancel();
          transition.current = animatePalette(stage.current, false, reduce);
        } else if (
          player.current!.visible &&
          (stage.current.dataset.presence === 'hidden' ||
            stage.current.dataset.presence === 'closing')
        ) {
          transition.current?.cancel();
          stage.current.style.visibility = 'visible';
          transition.current = animatePalette(stage.current, true, reduce);
        }
      }
    }
    commandSerial.current = command.serial;
    notifySample();
    if (reduce) for (let i = 0; i < 16; i++) player.current!.advance(0.064);
    renderer.current?.update({});
    // Commands are delivered once; changing an expression does not repeat the gesture.
  }, [command, adjusted, reduce, assignment]);
  useLayoutEffect(() => {
    if (paused) transition.current?.pause();
    else transition.current?.play();
  }, [paused, command]);
  const style = {
    '--preview-size': `${bar ? 42 : 42 * zoom}px`,
    '--preview-ratio': (size + 80) / size,
  } as React.CSSProperties;
  const avatar = (
    <span className="shared-avatar" style={style}>
      <canvas ref={canvas} role="img" aria-label={label} />
    </span>
  );
  const Visual = onInspect ? 'button' : 'div';
  return (
    <div className="expression-render" data-bar={bar} data-dark={dark}>
      <div className="expression-stage">
        <Visual
          className="preview-visual"
          onClick={onInspect}
          aria-label={
            onInspect ? `Edit ${candidate?.name ?? 'No effect'}` : undefined
          }
        >
          <div
            ref={stage}
            className={bar ? 'shared-bar' : 'shared-avatar-preview'}
            data-surface={setup.appearance.uiSurface ?? 'porcelain'}
            data-dark={dark}
            style={style}
          >
            {avatar}
            {bar && (
              <input
                ref={input}
                aria-label="Test command"
                placeholder="Search…"
                value={query}
                onChange={(e) => onQueryChange?.(e.target.value)}
                onKeyDown={(e) => {
                  if (e.nativeEvent.isComposing) return;
                  if (e.key === 'Enter') {
                    e.preventDefault();
                    onAction?.('submit');
                  } else if (
                    !e.metaKey &&
                    !e.ctrlKey &&
                    !e.altKey &&
                    (e.key.length === 1 ||
                      e.key === 'Backspace' ||
                      e.key === 'Delete')
                  )
                    onAction?.('typing');
                }}
              />
            )}
          </div>
          {overlay && <span className="preview-overlay">{overlay}</span>}
        </Visual>
        {caption && <div className="preview-caption">{caption}</div>}
      </div>
      {curves && (
        <InputCurves
          source={timeline}
          assignment={assignment}
          status={status ?? (assignment ? '' : undefined)}
        />
      )}
    </div>
  );
}
