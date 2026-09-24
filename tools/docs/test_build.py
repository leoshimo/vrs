"""Export contracts: intact source, no execution, portable links, repeatable builds."""
from contextlib import redirect_stdout
from html.parser import HTMLParser
import importlib.util
import io
from pathlib import Path
import re
import shutil
import tempfile
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location('export_docs', Path(__file__).with_name('build.py'))
docs = importlib.util.module_from_spec(spec)
spec.loader.exec_module(docs)


class Text(HTMLParser):
    def __init__(self, markup):
        super().__init__()
        self.parts = []
        self.feed(markup)

    def handle_data(self, value):
        self.parts.append(value)


class ExportTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name).resolve()
        self.site = self.root / 'public'
        for name in ('docs', 'assets', 'scripts'):
            (self.root / name).mkdir()
        self.sources = {'docs/index.md': 'docs/index.html', 'docs/other.md': 'docs/other.html'}
        self.assets = {name: name for name in ('docs/site.css', 'docs/site.js', 'docs/identity.css', 'assets/visuals/logomark.png')}
        (self.root / 'docs/site.css').write_text('body { color: black; }')
        (self.root / 'docs/site.js').write_text('// fixture')
        (self.root / 'docs/identity.css').write_text('/* identity fixture */')
        (self.root / 'assets/visuals').mkdir()
        (self.root / 'assets/visuals/logomark.png').write_bytes(b'fixture')
        (self.root / 'README.md').write_text('# Source repository')
        (self.root / 'scripts/tool.ll').write_text('(+ 1 2)')
        (self.root / 'assets/a picture.svg').write_text('<svg xmlns="http://www.w3.org/2000/svg"/>')
        self.marker = self.root / 'must-not-exist'
        (self.root / 'TODO.org').write_text('* TODO Source-only task\n')
        (self.root / 'docs/index.md').write_text(f'''# Fixture

A paragraph with & and &lt;literal&gt; text.

[Other page](other.md#other)
[README](../README.md)
[Program](../scripts/tool.ll)
[Tasks](../TODO.org)
![Picture](<../assets/a picture.svg>)

## A heading

```vrs
(print "<&>") # source characters, not HTML
```

```python
from pathlib import Path
Path({str(self.marker)!r}).write_text("export executed code")
```

```

(add_todo "Recorded action")
```

## Another heading
''')
        (self.root / 'docs/other.md').write_text('''# Other

<a id="other"></a>

## Other

[Back](index.md)
''')
        for name, value in [('ROOT', self.root), ('SOURCES', self.sources), ('ASSETS', self.assets), ('LANDING', None)]:
            p = patch.object(docs, name, value)
            p.start()
            self.addCleanup(p.stop)

    def build(self):
        with redirect_stdout(io.StringIO()):
            docs.build(self.site)

    def snapshot(self):
        return {str(p.relative_to(self.site)): p.read_bytes() for p in self.site.rglob('*') if p.is_file()}

    def test_export_is_repeatable_preserves_source_and_never_evaluates(self):
        before = {name: (self.root / name).read_bytes() for name in self.sources}
        self.build()
        first = self.snapshot()
        self.build()
        self.assertEqual(first, self.snapshot())
        self.assertFalse(self.marker.exists())
        self.assertEqual(before, {name: (self.root / name).read_bytes() for name in self.sources})
        for name in self.sources:
            self.assertFalse((self.site / name).exists())
        markup = (self.site / 'docs/index.html').read_text()
        block = re.search(r'<pre[^>]*class="src src-vrs"[^>]*>(.*?)</pre>', markup, re.S)[1]
        self.assertEqual('(print "<&>") # source characters, not HTML\n', ''.join(Text(block).parts))
        self.assertIn('id="a-heading"', markup)
        self.assertEqual(3, markup.count('class="copy-code"'))
        self.assertIn('aria-label="Copy code"', markup)
        self.assertEqual(3, markup.count('class="code-block"'))
        block = re.search(r'<pre class="example">(.*?)</pre>', markup, re.S)[1]
        # Browsers discard one opening newline in <pre>, leaving the source's own.
        self.assertTrue(block.startswith('\n\n'))
        self.assertEqual('\n(add_todo "Recorded action")\n', ''.join(Text(block[1:]).parts))

    def test_project_subpath_links_and_assets(self):
        self.build()
        markup = (self.site / 'docs/index.html').read_text()
        self.assertIn('href="other.html#other"', markup)
        self.assertIn('src="../assets/a%20picture.svg"', markup)
        self.assertIn('https://github.com/leoshimo/vrs/blob/main/scripts/tool.ll', markup)
        self.assertIn('https://github.com/leoshimo/vrs/blob/main/README.md', markup)
        self.assertIn('https://github.com/leoshimo/vrs/blob/main/TODO.org', markup)
        self.assertFalse((self.site / 'TODO.html').exists())
        # The same relative links validate when the entire site sits below /vrs/.
        nested = self.root / 'host/vrs'
        shutil.copytree(self.site, nested)
        docs.check_site(nested)
        self.assertTrue((nested / 'assets/a picture.svg').exists())

    def test_book_layout_links_and_intact_examples(self):
        source = self.root / 'docs/tour.md'
        code = '(print "<&>") # source, not HTML\n'
        source.write_text('# Markdown tour\n\n[Other](other.md#other)\n\n'
                          '[Rendered link](other.html#other)\n\n'
                          '## Try it\n\n```vrs\n' + code + '```\n\n'
                          '### Try it\n\nA *live* value.\n')
        self.sources['docs/tour.md'] = 'docs/tour.html'
        with patch.object(docs, 'SOURCES', self.sources):
            self.build()
            first = self.snapshot()
            self.build()
        self.assertEqual(first, self.snapshot())
        markup = (self.site / 'docs/tour.html').read_text()
        self.assertIn('href="https://github.com/leoshimo/vrs" aria-label="VRS on GitHub"', markup)
        self.assertIn('href="https://github.com/leoshimo/vrs/blob/main/docs/tour.md"', markup)
        self.assertNotIn('download>Source</a>', markup)
        self.assertNotIn('Markdown source', markup)
        self.assertIn('class="github-source"', markup)
        self.assertIn('>View source</a>', markup)
        title = re.search(r'<header class="document-title">(.*?)</header>', markup, re.S)[1]
        self.assertNotIn('<a ', title)
        self.assertNotIn('↗', title)
        self.assertIn('aria-current="page">Tour</a>', markup)
        self.assertIn('<article class="document">', markup)
        self.assertNotIn('href="tour.md"', markup)
        self.assertEqual(2, markup.count('href="other.html#other"'))
        self.assertIn('id="try-it"', markup)
        self.assertIn('id="try-it-2"', markup)
        block = re.search(r'<pre[^>]*class="src src-vrs"[^>]*>(.*?)</pre>', markup, re.S)[1]
        self.assertEqual(code, ''.join(Text(block).parts))
        self.assertFalse((self.site / 'docs/tour.md').exists())

    def test_broken_link_leaves_previous_site_unchanged(self):
        self.build()
        before = self.snapshot()
        path = self.root / 'docs/other.md'
        path.write_text(path.read_text() + '\n[Missing](absent.md)\n')
        with self.assertRaises(ValueError):
            self.build()
        self.assertEqual(before, self.snapshot())

    def test_missing_anchor_and_duplicate_id_are_errors(self):
        self.build()
        path = self.site / 'docs/index.html'
        text = path.read_text()
        path.write_text(text.replace('other.html#other', 'other.html#absent'))
        with self.assertRaisesRegex(ValueError, 'missing anchor'):
            docs.check_site(self.site)
        path.write_text(text + '<div id="a-heading"></div>')
        with self.assertRaisesRegex(ValueError, 'duplicate IDs'):
            docs.check_site(self.site)

    def test_only_old_generated_files_are_removed(self):
        self.build()
        unmanaged = self.site / 'keep.txt'
        unmanaged.write_text('not produced by the exporter')
        source = self.root / 'docs/index.md'
        source.write_text(source.read_text().replace('![Picture](<../assets/a picture.svg>)', ''))
        self.build()
        self.assertFalse((self.site / 'assets/a picture.svg').exists())
        self.assertEqual('not produced by the exporter', unmanaged.read_text())


class UtilityTests(unittest.TestCase):
    def test_explicit_markdown_anchors_and_documents_without_sections(self):
        text = '# Guide\n\n<a id="editor"></a>\n\n## Working in the editor\n\n## Editor\n'
        markup = docs.render_markdown(text)
        self.assertIn('<h2 id="editor">Working in the editor</h2>', markup)
        self.assertIn('<h2 id="editor-2">Editor</h2>', markup)
        self.assertNotIn('<a id="editor"></a>', markup)
        with self.assertRaisesRegex(ValueError, 'Duplicate explicit'):
            docs.render_markdown(text + '\n<a id="editor"></a>\n\n## Another\n')
        self.assertNotIn('table-of-contents', docs.render_markdown('# Index\n\nA document.\n'))

    def test_highlighting_preserves_every_character(self):
        source = '(fn () """raw\n<&>\\path\n""")\n# > comment\n\'(thing :url "https://example.com?a=1&b=2")\n'
        for language in ('vrs', 'sh', 'emacs-lisp'):
            self.assertEqual(source, ''.join(Text(docs.highlight(source, language)).parts))

    def test_watch_recovers_without_repeating_the_same_failed_build(self):
        states = iter(['a', 'b', 'b', 'c'])
        calls = []

        def scan():
            try:
                return next(states)
            except StopIteration:
                raise KeyboardInterrupt

        def rebuild():
            calls.append(True)
            if len(calls) == 1:
                raise ValueError('invalid edit')

        output = io.StringIO()
        with self.assertRaises(KeyboardInterrupt), redirect_stdout(output):
            docs.watch(rebuild, scan=scan, sleep=lambda _: None)
        self.assertEqual(2, len(calls))
        self.assertEqual(1, output.getvalue().count('Build failed'))


class LandingTests(unittest.TestCase):
    def test_production_landing_and_assets_work_under_project_prefix(self):
        before = {name: (docs.ROOT / name).read_bytes() for name in docs.SOURCES}
        with tempfile.TemporaryDirectory() as directory:
            site = Path(directory) / 'vrs'
            with redirect_stdout(io.StringIO()):
                docs.build(site)
            home = (site / 'docs/index.html').read_text()
            self.assertIn('Under heavy construction, in perpetuity.', home)
            self.assertIn('class="orb-fallback"', home)
            self.assertIn('prefers-color-scheme: dark', home)
            self.assertNotIn('class="wordmark"', home)
            self.assertIn('>Take the tour</a>', home)
            self.assertTrue((site / 'assets/visuals/ink.js').is_file())
            self.assertTrue((site / 'assets/visuals/sphere.js').is_file())
            docs.check_site(site)
            # A missing dark fallback must fail link validation, including srcset.
            (site / 'assets/visuals/sphere-dark.png').unlink()
            with self.assertRaisesRegex(ValueError, 'sphere-dark.png'):
                docs.check_site(site)
        self.assertEqual(before, {name: (docs.ROOT / name).read_bytes() for name in docs.SOURCES})


if __name__ == '__main__':
    unittest.main()
