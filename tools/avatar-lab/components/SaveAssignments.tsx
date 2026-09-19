'use client';
import { useId, useRef, useState } from 'react';

export function SaveAssignments({
  modified,
  ready,
  saving,
  error,
  onSave,
}: {
  modified: boolean;
  ready: boolean;
  saving: boolean;
  error: string;
  onSave: () => Promise<boolean>;
}) {
  const dialog = useRef<HTMLDialogElement>(null);
  const id = useId();
  const [saved, setSaved] = useState(false);
  return (
    <div className="assignment-save">
      <button
        className="save-defaults"
        disabled={!modified || !ready || saving}
        onClick={() => dialog.current?.showModal()}
      >
        {saving
          ? 'Saving…'
          : saved && !modified
            ? 'Defaults saved'
            : 'Save as defaults'}
      </button>
      <output className="sr-only">
        {saved && !modified ? 'Assignment defaults saved.' : ''}
      </output>
      <dialog
        ref={dialog}
        className="save-dialog"
        aria-labelledby={`${id}-title`}
        aria-describedby={`${id}-description`}
        onCancel={(event) => {
          if (saving) event.preventDefault();
        }}
      >
        <form
          onSubmit={async (event) => {
            event.preventDefault();
            if (await onSave()) {
              setSaved(true);
              dialog.current?.close();
            }
          }}
        >
          <h2 id={`${id}-title`}>Save as defaults?</h2>
          <p id={`${id}-description`}>
            Replace VRSJMP’s saved assignments with the applied effects and
            their settings. Takes effect on the next build.
          </p>
          {error && (
            <p role="alert" className="configuration-error">
              {error}
            </p>
          )}
          <div className="save-dialog-actions">
            <button
              type="button"
              disabled={saving}
              onClick={() => dialog.current?.close()}
            >
              Cancel
            </button>
            <button className="save-defaults" type="submit" disabled={saving}>
              {saving ? 'Saving…' : 'Save defaults'}
            </button>
          </div>
        </form>
      </dialog>
    </div>
  );
}
