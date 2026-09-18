'use client';
/* oxlint-disable jsx-a11y/prefer-tag-over-role -- Canvas is the rendered image. */
import {
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
  type ReactNode,
} from 'react';
import { useMediaQuery } from './useMediaQuery';
import { registerPreview } from '@/lib/avatar/preview-renderer';
import {
  ExpressionPlayer,
  type PlaybackAction,
} from '../../../vrsjmp/src/avatar/expression-player';
import {
  type Assignment,
  type AvatarExpression,
  type AvatarSetup,
} from '@/lib/avatar/workspace';
import { applyPlaceholderAccent } from '@/lib/avatar/preview-colors';
import { comparisonSetup, comparisonAction } from '@/lib/avatar/comparison';
import { InputCurves } from './InputCurves';
import { SignalTimeline } from '@/lib/avatar/signal-timeline';

export type PreviewCommand = {
  serial: number;
  action: PlaybackAction | 'reset' | 'resetHidden';
};
export function ExpressionPreview({
  setup,
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
  status,
  onSample,
  onInspect,
  label = 'Avatar preview',
}: {
  setup: AvatarSetup;
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
  status?: string;
  onSample?: (player: ExpressionPlayer, paint: boolean) => void;
  onInspect?: () => void;
  label?: string;
}) {
  const canvas = useRef<HTMLCanvasElement>(null);
  const candidateId = candidate?.id;
  const adjusted = useMemo(
    () =>
      assignment
        ? comparisonSetup(setup, { id: candidateId ?? null, input: assignment })
        : setup,
    [setup, candidateId, assignment],
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
  useLayoutEffect(() => {
    renderer.current = registerPreview(canvas.current!, player.current!);
    return () => renderer.current?.dispose();
  }, []);
  useLayoutEffect(() => {
    if (engine && player.current !== engine) {
      player.current = engine;
      renderer.current?.update({ player: engine });
    }
    player.current!.setup = adjusted;
    if (working !== undefined)
      player.current!.working =
        assignment && assignment !== 'working' ? false : working;
    if (shown !== undefined)
      player.current!.visible = assignment ? true : shown;
    renderer.current?.update({
      dark,
      resolution: size,
      live: live && !reduce,
      paused,
      onFrame,
      trackTime: curves || Boolean(onSample),
    });
  });
  useLayoutEffect(() => {
    if (!command || commandSerial.current === command.serial) return;
    commandSerial.current = command.serial;
    if (assignment) {
      const action = comparisonAction(assignment, command.action);
      if (action) player.current!.trigger(action);
    } else if (command.action === 'reset' || command.action === 'resetHidden') {
      player.current = new ExpressionPlayer(adjusted);
      if (command.action === 'resetHidden') player.current.trigger('hide');
      renderer.current?.update({ player: player.current });
    } else player.current!.trigger(command.action);
    if (reduce) for (let i = 0; i < 16; i++) player.current!.frame(0.064);
    renderer.current?.update({});
    // Commands are delivered once; changing an expression does not repeat the gesture.
  }, [command, adjusted, reduce, assignment]);
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
          style={{
            visibility: !assignment && shown === false ? 'hidden' : undefined,
          }}
        >
          <div
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
