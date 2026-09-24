# Building the documentation

Edit the Markdown pages in `docs/`. The sphere landing page combines
[`index.md`](index.md) with the template in
[`tools/docs/landing.html`](../tools/docs/landing.html); its styles and behavior
live in `docs/landing.css` and `docs/landing.js`. The selected visual assets and
regeneration instructions are in [`tools/visuals`](../tools/visuals/README.md).

The exporter lives in [`tools/docs/build.py`](../tools/docs/build.py). It adds
the shared page layout, navigation, repository link, and code highlighting.
[`site.css`](site.css) styles the pages; [`site.js`](site.js) handles the contents
and copy buttons. All generated pages and copied assets go into `_site/`,
which Git ignores. Source files stay in place; source links open GitHub.
Examples are displayed, never evaluated during export.

Install [uv](https://docs.astral.sh/uv/getting-started/installation/), then run:

```sh
uv run --locked tools/docs/build.py              # Build _site and check local links
uv run --locked tools/docs/build.py --serve 8769 # Preview and rebuild on edits
```

The builder declares its dependencies inline. uv uses `tools/docs/build.py.lock`
and manages Python and its environment; no virtualenv setup is needed.

The preview serves `_site/`. Open <http://127.0.0.1:8769/> and refresh the browser
after an edit. A failed build
leaves the last good preview in place and reports the error in the terminal.
Use another port if 8769 is already occupied.

Use `--watch` to rebuild without a server, or `--output PATH` for a separate
build directory. Edit the Markdown sources; the next build replaces generated HTML.

Link between pages using relative paths, such as `[Services](manual.md#services)`.
Markdown headings receive stable slugs. An explicit `<a id="services"></a>`
before a heading preserves its anchor when the heading is renamed, both in
the exported site and on GitHub.
Export converts page links to HTML, copies linked images from `assets/`, and points code links at GitHub.
Relative site links work both locally and under GitHub Pages's `/vrs/` path.

## Checks and publishing

```sh
uv run --locked tools/docs/build.py --test
```

Tests live in [`tools/docs/test_build.py`](../tools/docs/test_build.py) and run
with the builder's locked dependencies. To update dependencies, edit the inline
metadata and run `uv lock --script tools/docs/build.py`, then rerun the checks.

The [documentation workflow](../.github/workflows/docs.yml) installs uv,
runs these same commands, and builds `_site`. Pull requests build an artifact for review;
changes on `main` also deploy it to GitHub Pages. Generated HTML is ignored by
Git; the workflow builds it from the Markdown sources for publishing.

Before the first deployment, select **GitHub Actions** under the repository's
**Settings → Pages → Build and deployment → Source**. This setting is separate
from adding the workflow file.
