# Avatar Lab

```sh
cd tools/avatar-lab
pnpm install
pnpm dev
```

Open <http://127.0.0.1:5173>.

Expressions edits the avatar's appearance and input assignments. Each assignment
owns its tweaks. Select an effect to edit the Candidate preview. Assigned and the main
bar keep showing the working assignment until you choose Apply.
Reset returns the candidate to its starting values. Pin snapshots to compare more
alternatives under the same inputs; candidate edits stay in this browser.
The preview row stays visible as the workspace scrolls. On narrow windows, selecting
an effect opens its settings over the list. Close the pane or press Escape to return
to the list; the candidate's edits remain.
The rail labels send manual inputs and switch to Manual, which leaves the tracks
empty. Choose a sequence to run it; Play/Pause controls its clock. Edit opens
quarter-second cells, and the end time sets the loop length.

Save as defaults confirms and writes the working assignment set to
`vrsjmp/src/avatar/assigned-setup.json`. Each changed assignment has a Revert control
that restores its saved effect and parameters. Appearance saves separately.
Saving keeps the current previews and playback in place. The local dev server
handles saving; an exported site can only preview changes. Saving does not commit.
VRSJMP imports the file at build time, so the packaged app needs no external config
file. Rebuild/restart VRSJMP to pick up saved changes outside hot-reload development.

Recipe walks through the field, printing, response curves, and composition using
the shared renderer in `vrsjmp/src/avatar`. The lab's catalog and sequencer live in
`lib/avatar`; the native app contains only the assigned effects.
