'use client';
import { useState } from 'react';
import { SignalTimeline } from '@/lib/avatar/signal-timeline';
import { InputCurves } from './InputCurves';
import { Pause, Play } from 'lucide-react';
import {
  assigned,
  assignmentLabels,
  assignmentNames,
  type AvatarSetup,
} from '@/lib/avatar/workspace';
import { ExpressionPreview } from './ExpressionPreview';
import { usePlayback } from './usePlayback';
import { Button } from './ui/button';

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
        <div className="response-actions">
          <Button onClick={() => playback.act('typing')}>Type</Button>
          <Button onClick={() => playback.act('submit')}>Submit</Button>
          <Button onClick={() => playback.act('complete')}>Complete</Button>
          <Button
            aria-pressed={playback.working}
            onClick={() => playback.act(playback.working ? 'idle' : 'working')}
          >
            {playback.working ? 'Idle' : 'Work'}
          </Button>
          <Button onClick={playback.togglePlay}>
            {playback.paused ? <Play size={13} /> : <Pause size={13} />}
            {playback.paused ? 'Play' : 'Pause'}
          </Button>
        </div>
      </aside>
      <div className="recipe-calculation composition-notes">
        <div className="expression-render" data-dark={dark}>
          <InputCurves source={timeline} />
        </div>
        <div>
          <p>
            Each assignment drives a motion curve and a color curve. Idle holds
            its signal; Working holds while work is active. A keypress or
            submission adds a Typing pulse. Complete fires when an action succeeds.
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
            An expression interprets its motion signal. Nudge changes the
            field’s speed. Stir distorts its coordinates. Ripple samples earlier
            values of its signal at increasing distances, so several taps can
            travel across the sphere together.
          </p>
          <p>
            The renderer combines those changes before shading and printing one
            sphere. The plots show the separate inputs: two curves at the same
            height can produce different motions.
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
