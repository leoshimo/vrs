'use client';
import { useLayoutEffect, useRef, type CSSProperties } from 'react';
import { SignalField } from './SignalField';
import type { SignalConfig } from '@/lib/avatar/signal-field';
import type { MotionMode } from '@/lib/avatar/signal-motion';
import type { StudyTheme } from './PreviewAppearance';
export function AvatarViewer({
  config,
  state = 'idle',
  pulse = 0,
  invocation = 0,
  appearance = 1,
  animate = true,
  active = true,
  theme = 'dark',
  size = 96,
  displaySize,
  zoom = 1,
  bar = false,
  working,
  windowOpacity = 1,
  query,
  onQueryChange,
  onType,
  onSubmit,
}: {
  config: SignalConfig;
  state?: MotionMode;
  pulse?: number;
  invocation?: number;
  appearance?: number;
  animate?: boolean;
  active?: boolean;
  theme?: StudyTheme;
  size?: number;
  displaySize?: number;
  zoom?: number;
  bar?: boolean;
  working?: boolean;
  windowOpacity?: number;
  query?: string;
  onQueryChange?: (value: string) => void;
  onType?: () => void;
  onSubmit?: () => void;
}) {
  const viewer = useRef<HTMLDivElement>(null);
  useLayoutEffect(() => {
    const el = viewer.current;
    const canvas = el?.parentElement;
    if (!el || !canvas || !bar) return;
    const fit = () => {
      el.style.scale = String(
        Math.min(1, Math.max(0.1, (canvas.clientWidth - 32) / 560)),
      );
    };
    const observer = new ResizeObserver(fit);
    observer.observe(canvas);
    fit();
    return () => {
      observer.disconnect();
      el.style.removeProperty('scale');
    };
  }, [bar]);
  const presented = bar ? 42 : (displaySize ?? size);
  const magnification = bar ? 1 : zoom;
  const avatar = (
    <div className="avatar-render-slot">
      <div className="avatar-object">
        <span className="signal-anchor" />
        <SignalField
          config={{
            ...config,
            boundary: 'none',
            effects: {
              ...config.effects,
              input: false,
              selection: false,
              container: false,
            },
            dispatchTarget: 'avatar',
          }}
          state={state}
          working={working}
          pulse={pulse}
          selected="avatar"
          standalone
          animate={animate}
          active={active}
          dark={theme === 'dark'}
          invocation={invocation}
          appearance={appearance}
        />
      </div>
    </div>
  );
  return (
    <div
      className="avatar-viewer"
      ref={viewer}
      data-bar={bar}
      data-zoomed={magnification > 1}
      data-appearance={theme}
      data-surface={config.uiSurface || 'porcelain'}
      data-font={config.uiFont || 'system'}
      style={
        {
          opacity: windowOpacity,
          '--avatar-size': `${size}px`,
          '--avatar-presented': `${presented * magnification}px`,
          '--avatar-scale': (presented / size) * magnification,
        } as CSSProperties
      }
    >
      {bar ? (
        <div className="avatar-viewer-input">
          <div className="avatar-bar-surface" aria-hidden="true" />
          {avatar}
          {onQueryChange ? (
            <input
              className="avatar-bar-input"
              aria-label="Test command"
              placeholder="Search…"
              value={query ?? ''}
              onChange={(e) => onQueryChange(e.target.value)}
              onKeyDown={(e) => {
                if (e.nativeEvent.isComposing) return;
                if (e.key === 'Enter') {
                  e.preventDefault();
                  onSubmit?.();
                } else if (
                  !e.metaKey &&
                  !e.ctrlKey &&
                  !e.altKey &&
                  (e.key.length === 1 ||
                    e.key === 'Backspace' ||
                    e.key === 'Delete')
                )
                  onType?.();
              }}
            />
          ) : (
            <span className="avatar-bar-placeholder">Search…</span>
          )}
        </div>
      ) : (
        avatar
      )}
    </div>
  );
}
