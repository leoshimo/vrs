#!/usr/bin/env -S uv run --locked --script
# /// script
# requires-python = ">=3.10"
# dependencies = ["markdown-it-py==4.2.0"]
# ///
"""Build and preview Markdown documentation without executing examples."""
import argparse
import hashlib
import html
from html.parser import HTMLParser
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer
import json
import os
from pathlib import Path
import re
import shutil
import tempfile
import threading
import time
from urllib.parse import quote, unquote, urlsplit

ROOT = Path(__file__).resolve().parents[2]
SOURCES = {
    'tools/docs/index.md': 'docs/index.html',
    'docs/tour.md': 'docs/tour.html',
    'docs/design.md': 'docs/design.html',
    'docs/manual.md': 'docs/manual.html',
}
ASSETS = {
    **{f'tools/docs/assets/{name}': f'docs/{name}' for name in (
        'site.css', 'site.js', 'identity.css', 'landing.css', 'landing.js',
        'fonts/charter-regular.woff2', 'fonts/charter-bold.woff2',
        'fonts/charter-italic.woff2', 'fonts/charter-bold-italic.woff2',
        'fonts/charter-LICENSE.txt')},
    **{f'assets/visuals/{name}': f'assets/visuals/{name}' for name in (
        'logomark.png', 'manicule.png', 'sphere-light.png', 'sphere-dark.png')},
    'tools/visuals/sphere.js': 'assets/visuals/sphere.js',
    'tools/visuals/ink.js': 'assets/visuals/ink.js',
}
REPOSITORY = 'https://github.com/leoshimo/vrs'
LANDING = Path('tools/docs/landing.html')
GITHUB_ICON = '''<svg viewBox="0 0 24 24" width="21" height="21" aria-hidden="true"><path fill="currentColor" d="M12 .7a11.3 11.3 0 0 0-3.57 22.02c.56.1.77-.24.77-.54v-2.1c-3.15.69-3.82-1.34-3.82-1.34-.51-1.31-1.26-1.66-1.26-1.66-1.03-.7.08-.69.08-.69 1.14.08 1.73 1.17 1.73 1.17 1.01 1.73 2.65 1.23 3.3.94.1-.73.4-1.23.72-1.51-2.51-.28-5.15-1.26-5.15-5.59 0-1.23.44-2.24 1.17-3.03-.12-.28-.51-1.43.11-2.99 0 0 .95-.3 3.11 1.16A10.8 10.8 0 0 1 12 6.16c.96 0 1.93.13 2.83.38 2.16-1.46 3.11-1.16 3.11-1.16.62 1.56.23 2.71.11 2.99.73.79 1.17 1.8 1.17 3.03 0 4.34-2.65 5.3-5.17 5.58.41.35.77 1.03.77 2.09v3.11c0 .3.21.65.78.54A11.3 11.3 0 0 0 12 .7Z"/></svg>'''



def markdown_parser():
    try:
        from markdown_it import MarkdownIt
    except ImportError as error:
        raise RuntimeError('Run this builder with uv: uv run --locked tools/docs/build.py') from error
    return MarkdownIt('commonmark').enable('table')


def render_markdown(text):
    """Render CommonMark with stable anchors, a contents list, and intact code blocks."""
    parser = markdown_parser()
    tokens = parser.parse(text)
    if not tokens or tokens[0].type != 'heading_open' or tokens[0].tag != 'h1':
        raise ValueError('Markdown documents must begin with a # title.')
    title = parser.renderer.render([tokens[1]], parser.options, {})
    plain_title = ''.join(t.content for t in tokens[1].children or [])
    tokens = tokens[3:]
    # Plain HTML anchors keep existing section links valid in Markdown readers.
    # In our HTML export, attach an anchor immediately before a heading to it.
    anchors = {}
    for i, token in enumerate(tokens[:-1]):
        heading, paragraph = i + 1, False
        if (token.type == 'inline' and i > 0 and i + 2 < len(tokens) and
                tokens[i - 1].type == 'paragraph_open' and tokens[i - 1].level == 0 and
                tokens[i + 1].type == 'paragraph_close'):
            heading, paragraph = i + 2, True
        elif token.type != 'html_block':
            continue
        if tokens[heading].type != 'heading_open':
            continue
        match = re.fullmatch(r'\s*<a id="([a-zA-Z0-9_-]+)"></a>\s*', token.content)
        if match:
            anchors[heading] = match[1]
            token.content = ''
            if paragraph:
                token.children = []
                tokens[i - 1].hidden = tokens[i + 1].hidden = True
    if len(set(anchors.values())) != len(anchors):
        raise ValueError('Duplicate explicit Markdown heading anchor')
    used, headings = set(), []
    for i, token in enumerate(tokens):
        if token.type != 'heading_open' or token.level != 0:
            continue
        if token.tag == 'h1':
            raise ValueError('Use one # title and ## headings for document sections.')
        label = tokens[i + 1]
        plain = ''.join(t.content for t in label.children or [])
        slug = re.sub(r'[^a-z0-9]+', '-', plain.lower()).strip('-') or 'section'
        ident, number = anchors.get(i, slug), 2
        if i not in anchors:
            while ident in used or ident in anchors.values():
                ident = f'{slug}-{number}'
                number += 1
        used.add(ident)
        token.attrSet('id', ident)
        headings.append((int(token.tag[1]), ident,
                         parser.renderer.render([label], parser.options, {})))

    def fence(tokens, index, options, env):
        token = tokens[index]
        language = token.info.split()[0] if token.info.strip() else ''
        code = html.escape(token.content, quote=False)
        if not language:
            return f'<div class="code-block"><pre class="example">{code}</pre></div>\n'
        if not re.fullmatch(r'[\w+-]+', language):
            raise ValueError(f'Invalid code language: {language}')
        return f'<div class="code-block"><pre class="src src-{language}">{code}</pre></div>\n'

    parser.renderer.rules['fence'] = fence
    # Nested outline containers keep the existing styles and contents controls.
    body, levels, start = [], [], 0
    for i, token in enumerate(tokens):
        if token.type != 'heading_open' or token.level != 0:
            continue
        body.append(parser.renderer.render(tokens[start:i], parser.options, {}))
        level = int(token.tag[1])
        while levels and levels[-1] >= level:
            body.append('</div>\n')
            levels.pop()
        ident = token.attrGet('id')
        body.append(f'<div id="outline-container-{ident}" class="outline-{level}">\n')
        levels.append(level)
        start = i
    body.append(parser.renderer.render(tokens[start:], parser.options, {}))
    body.extend('</div>\n' for _ in levels)

    tree = []
    parents = [(1, tree)]
    for level, ident, label in headings:
        if level > 3:
            continue
        while parents[-1][0] >= level:
            parents.pop()
        children = []
        parents[-1][1].append((ident, label, children))
        parents.append((level, children))

    def contents(nodes):
        return '<ul>\n' + ''.join(
            f'<li><a href="#{ident}">{label}</a>' +
            (contents(children) if children else '') + '</li>\n'
            for ident, label, children in nodes) + '</ul>\n'

    toc = ('<nav id="table-of-contents" role="doc-toc"><h2>Table of Contents</h2>'
           '<div id="text-table-of-contents">' + contents(tree) + '</div></nav>\n') if tree else ''
    return ('<!DOCTYPE html>\n<html lang="en"><head>\n<meta charset="utf-8">\n'
            '<meta name="viewport" content="width=device-width, initial-scale=1">\n'
            f'<title>{html.escape(plain_title)}</title>\n</head>\n<body>\n'
            '<div id="content" class="content">\n'
            f'<header><h1 class="title">{title}</h1></header>' + toc +
            ''.join(body) + '</div>\n</body></html>\n')


def highlight(code, language):
    """Source-preserving highlighting for Lyric and shell examples."""
    if language not in {'vrs', 'sh', 'bash'}:
        return html.escape(code, quote=False)
    pattern = re.compile(r'(""".*?"""|"(?:\\.|[^"\\])*"|\#[^\n]*|:[\w/!?-]+|(?<![\w])[-+]?\d+(?:\.\d+)?\b|[\w_!?+*/<>=.-]+|.)', re.S)
    pieces, after_paren = [], False
    for match in pattern.finditer(code):
        token, kind = match[0], None
        if token.startswith('"'):
            kind = 'string'
        elif token.startswith('#'):
            kind = 'comment'
        elif language == 'vrs' and token.startswith(':'):
            kind = 'keyword'
        elif re.fullmatch(r'[-+]?\d+(?:\.\d+)?', token):
            kind = 'number'
        elif language == 'vrs' and after_paren and re.match(r'[\w_!?+*/<>=.-]', token):
            kind = 'form'
        escaped = html.escape(token, quote=False)
        pieces.append(f'<span class="token-{kind}">{escaped}</span>' if kind else escaped)
        if not token.isspace() and kind != 'comment':
            after_paren = token == '('
    return ''.join(pieces)


def rewrite_links(document, source, output):
    exports = {ROOT / name: Path(target) for name, target in SOURCES.items()}
    assets = set()

    def rewrite(match):
        attribute, raw = match.groups()
        url = urlsplit(html.unescape(raw))
        if url.scheme or url.netloc or not url.path:
            return match[0]
        path = (source.parent / unquote(url.path)).resolve()
        if not path.is_relative_to(ROOT):
            raise ValueError(f'{source.name}: link escapes repository: {raw}')
        relative = path.relative_to(ROOT)
        if path in exports:
            target = os.path.relpath(exports[path], output.parent)
        elif path.suffix == '.html' and path.with_suffix('.md') in exports:
            target = os.path.relpath(exports[path.with_suffix('.md')], output.parent)
        elif relative.parts and relative.parts[0] == 'assets':
            if not path.is_file():
                raise ValueError(f'{source.name}: missing asset: {raw}')
            assets.add(relative)
            target = os.path.relpath(relative, output.parent)
        else:
            if not path.exists():
                raise ValueError(f'{source.name}: missing source link: {raw}')
            kind = 'tree' if path.is_dir() else 'blob'
            target = f'{REPOSITORY}/{kind}/main/{quote(relative.as_posix())}'
        if not target.startswith('https://'):
            target = quote(target)
        if url.query:
            target += '?' + url.query
        if url.fragment:
            target += '#' + url.fragment
        return f'{attribute}="{html.escape(target, quote=True)}"'

    return re.sub(r'(href|src)="([^"]+)"', rewrite, document), assets


class Page(HTMLParser):
    def __init__(self, text):
        super().__init__()
        self.ids, self.duplicates, self.links = set(), set(), []
        self.feed(text)

    def handle_starttag(self, tag, attributes):
        attrs = dict(attributes)
        if 'id' in attrs:
            if attrs['id'] in self.ids:
                self.duplicates.add(attrs['id'])
            self.ids.add(attrs['id'])
        self.links.extend(attrs[key] for key in ('href', 'src') if key in attrs)
        if 'srcset' in attrs:
            self.links.extend(candidate.strip().split()[0] for candidate in attrs['srcset'].split(',') if candidate.strip())


class LayoutParts(HTMLParser):
    """Locate exporter wrappers without parsing or rewriting authored content."""
    void = {'area', 'base', 'br', 'col', 'embed', 'hr', 'img', 'input', 'link',
            'meta', 'param', 'source', 'track', 'wbr'}

    def __init__(self, document):
        super().__init__()
        self.document, self.stack, self.parts = document, [], {}
        self.lines = [0]
        for line in document.splitlines(keepends=True):
            self.lines.append(self.lines[-1] + len(line))
        self.feed(document)

    def source_offset(self):
        line, column = self.getpos()
        return self.lines[line - 1] + column

    def handle_starttag(self, tag, attributes):
        if tag not in self.void:
            start = self.source_offset()
            self.stack.append((tag, dict(attributes), start, start + len(self.get_starttag_text())))

    def handle_endtag(self, tag):
        if tag in self.void:
            return
        if not self.stack or self.stack[-1][0] != tag:
            raise ValueError(f'Unbalanced exported HTML at </{tag}>')
        _, attrs, start, inner = self.stack.pop()
        key = attrs.get('id')
        if tag == 'header' and self.stack and self.stack[-1][1].get('id') == 'content':
            key = 'title'
        if key in {'content', 'title', 'table-of-contents'}:
            close = self.source_offset()
            self.parts[key] = (start, inner, close, self.document.index('>', close) + 1)


def book_layout(document, source, output):
    parts = LayoutParts(document).parts
    _, start, end, _ = parts['content']
    article = document[start:end]
    for key in sorted((k for k in ('title', 'table-of-contents') if k in parts),
                      key=lambda k: parts[k][0], reverse=True):
        first, _, _, last = parts[key]
        article = article[:first - start] + article[last - start:]
    _, first, last, _ = parts['title']
    title = document[first:last]
    toc = ''
    if 'table-of-contents' in parts:
        _, first, last, _ = parts['table-of-contents']
        contents = re.sub(r'^\s*<h2[^>]*>.*?</h2>', '', document[first:last], count=1, flags=re.S)
        toc = ('<details id="table-of-contents" class="contents" open>'
               '<summary>Contents</summary><nav aria-label="Page contents">' +
               contents + '</nav></details>')
    exports = {Path(target).stem: Path(target) for target in SOURCES.values()}
    index = os.path.relpath('docs/index.html', output.parent)
    links = []
    for name, label in (('tour', 'Tour'), ('design', 'Design'), ('manual', 'Manual')):
        if name in exports:
            href = os.path.relpath(exports[name], output.parent)
            current = ' aria-current="page"' if output.stem.split('-')[0] == name else ''
            links.append(f'<a href="{href}"{current}>{label}</a>')
    home = output == Path('docs/index.html') and LANDING is not None
    mark = os.path.relpath('assets/visuals/logomark.png', output.parent)
    brand = '' if home else (f'<a class="wordmark" href="{index}" aria-label="VRS home">'
                            f'<img src="{mark}" width="57" height="19" alt="vrs"></a>')
    masthead = ('<header class="site-masthead">' + brand +
                '<nav aria-label="Documentation">' + ''.join(links) + '</nav>' +
                f'<a class="github" href="{REPOSITORY}" aria-label="VRS on GitHub">'
                + GITHUB_ICON + '</a></header>')
    author = '<span>Built by <a href="https://leoshimo.com/">leoshimo</a></span>'
    if home:
        body = ((ROOT / LANDING).read_text().replace('$NAV', masthead)
                .replace('$COPY', article).replace('$AUTHOR', author))
    else:
        source_link = f'{REPOSITORY}/blob/main/{quote(source.relative_to(ROOT).as_posix())}'
        sidebar = ('<aside class="document-sidebar">' + toc +
                   f'<a class="github-source" href="{source_link}">View source</a></aside>')
        body = (masthead + '<main id="content" class="page-grid">'
                f'<header class="document-title">{title}</header>' + sidebar +
                f'<article class="document">{article}</article></main>'
                f'<footer class="project-footer document-footer">{author}</footer>')
    before, rest = document.split('<body>', 1)
    _, after = rest.rsplit('</body>', 1)
    return before + '<body>\n' + body + '\n</body>' + after


def check_site(directory):
    pages = {p.resolve(): Page(p.read_text()) for p in directory.rglob('*.html')}
    errors, count = [], 0
    for path, page in pages.items():
        if page.duplicates:
            errors.append(f'{path.name}: duplicate IDs {sorted(page.duplicates)}')
        for value in page.links:
            url = urlsplit(value)
            if url.scheme or url.netloc:
                continue
            target = (path.parent / unquote(url.path)).resolve() if url.path else path
            count += 1
            if not target.is_relative_to(directory.resolve()) or not target.exists():
                errors.append(f'{path.name}: missing {value}')
            elif target.suffix == '.html' and url.fragment:
                if unquote(url.fragment) not in pages[target].ids:
                    errors.append(f'{path.name}: missing anchor {value}')
    if errors:
        raise ValueError('\n'.join(errors))
    return count


def build(destination):
    with tempfile.TemporaryDirectory(prefix='vrs-doc-export-') as temporary:
        work = Path(temporary)
        site = work / 'site'
        site.mkdir()
        source_texts = {name: (ROOT / name).read_text() for name in SOURCES}
        copied_assets = set()
        for name, target in SOURCES.items():
            source, output = ROOT / name, Path(target)
            document, assets = rewrite_links(render_markdown(source_texts[name]), source, output)
            copied_assets.update(assets)
            css = os.path.relpath('docs/site.css', output.parent)
            js = os.path.relpath('docs/site.js', output.parent)
            document = document.replace('</head>', f'<link rel="stylesheet" href="{css}">\n<script src="{js}" defer></script>\n</head>')
            identity = os.path.relpath('docs/identity.css', output.parent)
            document = document.replace('</head>', f'<link rel="stylesheet" href="{identity}">\n</head>')
            if output == Path('docs/index.html') and LANDING is not None:
                document = re.sub(r'<title>.*?</title>',
                                  '<title>VRS — A live programming environment for personal computing</title>',
                                  document, count=1)
                document = document.replace('</head>',
                    '<meta name="description" content="Build and connect personal software across applications and devices '
                    'with VRS, a live programming environment by leoshimo.">\n'
                    '<link rel="stylesheet" href="landing.css">\n'
                    '<script src="landing.js" type="module"></script>\n</head>')
            document = book_layout(document, source, output)
            blocks = []

            def code_block(match):
                attrs, _, language, content = match.groups()
                code = html.unescape(content)
                blocks.append(code)
                label = {'vrs': 'LYRIC', 'sh': 'SHELL', 'emacs-lisp': 'EMACS LISP'}.get(language, (language or '').upper())
                caption = f'<span>{label}</span>' if label else ''
                copy_label = f'Copy {label.lower()} code' if label else 'Copy code'
                toolbar = f'<div class="code-toolbar">{caption}<button class="copy-code" type="button" aria-label="{copy_label}">Copy</button></div>'
                # HTML discards the first newline after <pre>; preserve it in copied code.
                prefix = '\n' if code.startswith('\n') else ''
                return toolbar + f'<pre{attrs}>' + prefix + highlight(code, language) + '</pre>'

            document = re.sub(r'<pre([^>]*class="(src src-([^" ]+)|example)"[^>]*)>(.*?)</pre>', code_block, document, flags=re.S)
            expected = [t.content for t in markdown_parser().parse(source_texts[name]) if t.type == 'fence']
            if blocks != expected:
                raise ValueError(f'{name}: exported code blocks differ from source')
            target = site / output
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_text(document)
        for source, output in {**ASSETS, **{name: name for name in copied_assets}}.items():
            target = site / output
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(ROOT / source, target)
        (site / '.nojekyll').touch()
        (site / 'index.html').write_text('<!doctype html><html lang="en"><head><meta charset="utf-8"><meta http-equiv="refresh" content="0; url=docs/index.html"><title>VRS</title></head><body><a href="docs/index.html">VRS documentation</a></body></html>\n')
        count = check_site(site)
        if any((ROOT / name).read_text() != text for name, text in source_texts.items()):
            raise RuntimeError('A source changed during export; run the build again.')
        destination.mkdir(parents=True, exist_ok=True)
        manifest = destination / '.site-files.json'
        previous = json.loads(manifest.read_text()) if manifest.exists() else []
        current = sorted(str(p.relative_to(site)) for p in site.rglob('*') if p.is_file())
        for old in set(previous) - set(current):
            stale = (destination / old).resolve()
            if stale.is_relative_to(destination):
                stale.unlink(missing_ok=True)
        shutil.copytree(site, destination, dirs_exist_ok=True)
        manifest.write_text(json.dumps(current, indent=2) + '\n')
        print(f'Built {len(SOURCES)} documents; checked {count} local links → {destination}', flush=True)


def fingerprint():
    # An editor may save by temporarily removing/replacing a file.
    paths = [ROOT / name for name in (*SOURCES, *ASSETS)]
    if LANDING is not None:
        paths.append(ROOT / LANDING)
    paths.extend(sorted((ROOT / 'assets').rglob('*')) if (ROOT / 'assets').exists() else [])
    result = []
    for path in paths:
        try:
            value = hashlib.sha256(path.read_bytes()).digest() if path.is_file() else None
        except FileNotFoundError:
            value = None
        result.append((str(path), value))
    return tuple(result)


def watch(rebuild, scan=fingerprint, sleep=time.sleep):
    """Build once per edited snapshot, retaining the last successful preview."""
    previous = scan()
    while True:
        sleep(0.5)
        current = scan()
        if current == previous:
            continue
        previous = current
        try:
            rebuild()
        except (OSError, RuntimeError, ValueError) as error:
            print(f'Build failed; keeping previous preview: {error}', flush=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--test', action='store_true', help='Run the exporter tests')
    parser.add_argument('--output', type=Path, default=ROOT / '_site', help='Output directory (default: _site)')
    parser.add_argument('--serve', type=int, metavar='PORT', help='Serve on localhost and rebuild when sources change')
    parser.add_argument('--watch', action='store_true', help='Rebuild on changes without starting an HTTP server')
    args = parser.parse_args()
    if args.test:
        import unittest
        suite = unittest.defaultTestLoader.discover(str(Path(__file__).parent), pattern='test_build.py')
        result = unittest.TextTestRunner(verbosity=2).run(suite)
        raise SystemExit(0 if result.wasSuccessful() else 1)
    destination = args.output.resolve()
    if destination == ROOT or any((ROOT / name).is_relative_to(destination) for name in (*SOURCES, *ASSETS)):
        parser.error('Use an output directory separate from source files, such as _site.')

    def rebuild():
        build(destination)

    rebuild()
    server = None
    if args.serve:
        class Handler(SimpleHTTPRequestHandler):
            def __init__(self, *a, **kw):
                super().__init__(*a, directory=str(destination), **kw)

            def end_headers(self):
                self.send_header('Cache-Control', 'no-cache')
                super().end_headers()

        server = ThreadingHTTPServer(('127.0.0.1', args.serve), Handler)
        threading.Thread(target=server.serve_forever, daemon=True).start()
        print(f'Preview: http://127.0.0.1:{args.serve}/ — watching source files; Ctrl-C to stop.', flush=True)
    try:
        if args.watch or args.serve:
            watch(rebuild)
    except KeyboardInterrupt:
        pass
    finally:
        if server:
            server.shutdown()


if __name__ == '__main__':
    main()
