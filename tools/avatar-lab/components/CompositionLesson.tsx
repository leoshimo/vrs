'use client';
import { useState } from 'react';
import { SignalTimeline } from '@/lib/avatar/signal-timeline';
import { InputCurves } from './InputCurves';
import {
  assigned,
  assignmentLabels,
  assignmentNames,
  type AvatarSetup,
} from '@/lib/avatar/workspace';
import { ExpressionPreview } from './ExpressionPreview';
import { usePlayback } from './usePlayback';
import { PlaybackControls } from './PlaybackControls';

export function CompositionLesson({
  setup,
  dark,
}: {
  setup: AvatarSetup;
  dark: boolean;
}) {
  const playback = usePlayback(setup);
  const [timeline] = useState(() => new SignalTimeline());
  return (
    <div className="recipe-diagrams recipe-workbench composition-lesson">
      <aside className="recipe-local-preview">
        <figure className="recipe-sphere">
          <figcaption>Assigned avatar</figcaption>
          <div
            className="response-sphere"
            data-appearance={dark ? 'dark' : 'light'}
          >
            <ExpressionPreview
              setup={setup}
              dark={dark}
              command={playback.command}
              working={playback.working}
              shown={playback.shown}
              paused={playback.paused}
              zoom={3}
              live
              onSample={(player, paint) => timeline.record(player, paint)}
            />
          </div>
          <small>96 px · 3×</small>
        </figure>
      </aside>
      <div className="recipe-calculation composition-notes">
        <PlaybackControls playback={playback} />
        <div className="expression-render" data-dark={dark}>
          <InputCurves source={timeline} />
        </div>
        <div>
          <p>
            Each assignment drives a motion curve and a color curve. Idle holds
            its signal whenever the avatar is visible; Working adds a second
            held signal while work is active. Typing, Submit, Open, and Complete
            add pulses. Each assignment owns its effect and tweaks; assigning
            the same effect twice does not link their settings.
          </p>
          <table>
            <thead>
              <tr>
                <th>Input</th>
                <th>Expression</th>
                <th>Color</th>
              </tr>
            </thead>
            <tbody>
              {assignmentNames.map((a) => {
                const expression = assigned(setup, a);
                return (
                  <tr key={a}>
                    <td>{assignmentLabels[a]}</td>
                    <td>{expression?.name ?? 'No effect'}</td>
                    <td>
                      {expression?.color.mode === 'accent'
                        ? 'Accent'
                        : expression?.color.mode === 'custom'
                          ? 'Custom'
                          : 'Off'}
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        </div>
        <div>
          <p>
            The sequence supplies inputs to every preview. Window intervals open
            and hide it; their opening markers choose the entrance. Working is
            held over an interval. Typing cells send a few taps, while Submit
            and Complete each send one. Editing an assignment never changes
            playback. The buttons at the left switch to Manual for individual
            input.
          </p>
          <p>
            An expression interprets its motion signal. Nudge changes the
            field’s speed. Stir distorts its coordinates. Ripple samples earlier
            values of its signal at increasing distances, so several taps can
            travel across the sphere together.
          </p>
          <p>
            The renderer combines those changes before shading and printing one
            sphere. The plots show response strength. Nudge integrates that
            strength into phase; Ripple reads its recent history at a
            distance-dependent delay.
          </p>
          <p>
            Color has its own timing and strength. An expression with Color off
            contributes no accent, even while another expression is coloring the
            shared sphere.
          </p>
        </div>
      </div>
    </div>
  );
}
