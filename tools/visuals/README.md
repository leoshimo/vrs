# VRS visuals

The website uses the approved N16 mark, Charter, Scan drift lettering, and
Bayer sphere. It ships static HTML, CSS, images, and browser JavaScript.
The site builder copies `sphere.js` and `ink.js` into `assets/visuals/` in the
published site; there is no React runtime or frontend build step.

## Export

Install Node 22+, uv, and ffmpeg, then prepare the exporter once:

```sh
cd tools/visuals
npm ci
npx playwright install chromium
```

From the repository root:

```sh
tools/visuals/export-visuals
```

This crops the preserved source artwork, builds the production site in a
temporary directory, captures the actual landing renderer with a fixed browser
clock, and writes light/dark PNGs, eight-second GIFs, and sphere fallbacks into
`assets/visuals/`. The export recipe lives in `export.mjs`; there are no command
configuration flags. Commit regenerated assets with changes to the effects.

The Playwright version and browser revision are pinned by `package-lock.json`.
Frame timing and artwork are fixed. GPU rendering and ffmpeg versions can still
produce small pixel/encoding differences across operating systems; exported
files are checked in so normal docs builds do not depend on either tool.

## Update

- `ink.js`: the selected lettering effect. The static N16 image is its input.
- `sphere.js`: the frozen standalone sphere renderer selected in Avatar Lab.
  It is unchanged from the approved study. To adopt a later Avatar Lab effect,
  replace this reviewed standalone bundle and run the export and browser checks.
- `docs/landing.css` and `docs/landing.js`: layout and progressive enhancement.
- `tools/docs/landing.html`: landing markup; the two lines of copy are authored
  in `docs/index.md`.
- `assets/visuals/source/`: original artwork and generation prompts. Keep these
  originals; the exporter generates the cropped distribution assets.

The browser follows system light/dark appearance. Animation follows the system’s
reduced-motion preference, including changes made while the page is open. A static sphere remains available without
JavaScript or WebGL. Navigation marks stay static.

## Review

```sh
uv run --locked tools/docs/build.py --test
uv run --locked tools/docs/build.py --serve 8769
cd tools/visuals
npx playwright install chromium webkit
npm test
```

The browser checks build the production site beneath `/vrs/`, inspect all five
pages in light/dark at six widths in Chromium and WebKit, and exercise navigation,
copying, reduced motion, no JavaScript, missing WebGL, and README banner proportions.
Screenshots and the report go into ignored `tools/visuals/artifacts/`.
Check the branch README on GitHub as well: its image selection is controlled by
GitHub's Markdown renderer, separately from the documentation site.
