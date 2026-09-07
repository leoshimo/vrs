"""Safari adapter regression checks using a temporary database and runtime.

Run after `cargo build -p vrsd -p vrsctl`:
    python3 scripts/test_safari_history.py
Does not read or modify the user's browser history.
"""
import json
import sqlite3
import subprocess
import tempfile
import time
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
BIN = ROOT / "target/debug"


class SafariHistoryTests(unittest.TestCase):
    def history(self, visits):
        with tempfile.TemporaryDirectory(prefix="vrs-history-", dir="/tmp") as folder:
            folder = Path(folder)
            database = folder / "History.db"
            with sqlite3.connect(database) as connection:
                connection.executescript("""
                    CREATE TABLE history_items (id INTEGER PRIMARY KEY, url TEXT);
                    CREATE TABLE history_visits (
                        id INTEGER PRIMARY KEY, history_item INTEGER, title TEXT, visit_time REAL
                    );
                """)
                for index, (url, title) in enumerate(visits, 1):
                    connection.execute("INSERT INTO history_items VALUES (?, ?)", (index, url))
                    # Equal timestamps reproduce Safari's paired navigation records.
                    connection.execute("INSERT INTO history_visits VALUES (?, ?, ?, 100)",
                                       (index, index, title))
            source = (ROOT / "scripts/safari_history.ll").read_text().replace(
                '(shell_expand "~/Library/Safari/History.db")', json.dumps(str(database)))
            init = folder / "init.ll"
            init.write_text(source)
            socket = folder / "runtime.sock"
            with tempfile.TemporaryFile(mode="w+") as log:
                daemon = subprocess.Popen(
                    [BIN / "vrsd", "--node", "history-test", "--node-port", "0",
                     "--socket", socket, "--init", init], stdout=log, stderr=log)
                try:
                    deadline = time.monotonic() + 5
                    while not socket.exists():
                        if daemon.poll() is not None or time.monotonic() > deadline:
                            log.seek(0)
                            self.fail("Test daemon failed to start: " + log.read())
                        time.sleep(0.01)
                    return subprocess.run(
                        [BIN / "vrsctl", "--socket", socket, "-c", """
                          (begin
                            (bind_srv :safari_history)
                            (refresh_safari_history)
                            (map (get_safari_history)
                              (fn (entry) (list (get entry :url) (get entry :title)))))
                        """], capture_output=True, text=True, check=True, timeout=5).stdout
                finally:
                    daemon.terminate()
                    daemon.wait(timeout=5)

    def test_variants_merge_but_distinct_pages_remain(self):
        visits = [
            ("https://www.youtube.com/watch?v=first", "Video"),
            ("https://www.youtube.com/watch?v=first&pp=search-context", "Video"),
            ("https://www.youtube.com/watch?v=second", "Video"),
            ("https://example.test/article#section", "Old article title"),
            ("https://example.test/article?utm_source=news&gclid=tracking", "New article title"),
            ("https://example.test/other-article", "New article title"),
            ("https://www.google.com/search?q=one", "Search"),
            ("https://www.google.com/search?q=one&zx=transient", "Search"),
            ("https://www.google.com/search?q=two", "Search"),
            ("https://example.test/app#/one", "App"),
            ("https://example.test/app#/two", "App"),
            ("https://example.test/app#view=one", "App"),
            ("https://example.test/app#view=two", "App"),
            ("https://mail.google.com/mail/u/0/#inbox", "Mail"),
            ("https://mail.google.com/mail/u/0/#sent", "Mail"),
            ("https://example.test/?pp=one", "Other parameter"),
            ("https://example.test/?pp=two", "Other parameter"),
            ("https://example.test/?q=one?two", "Question mark in query"),
            ("https://example.test/repeated", "Old visit"),
            ("https://example.test/repeated", "Latest visit"),
            ("https://example.test/untitled", "   "),
            ("", "No URL"),
        ]
        # Latest original URLs and titles survive; equal-looking titles do not
        # collapse different video IDs, searches, paths, or application routes.
        retained = [row for index, row in enumerate(visits) if index not in (0, 3, 6, 18, 20, 21)]
        expected = "(" + " ".join(
            "(" + " ".join(json.dumps(value) for value in row) + ")"
            for row in reversed(retained)) + ")\n"
        self.assertEqual(self.history(visits), expected)

    def test_empty_history(self):
        self.assertEqual(self.history([]), "()\n")


if __name__ == "__main__":
    unittest.main()
