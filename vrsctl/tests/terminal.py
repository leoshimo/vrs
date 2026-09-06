"""Real CLI/PTY checks. Run after: cargo build -p vrsctl -p vrsd

Uses a temporary socket, an ephemeral node port, and no init scripts. Does not
connect to the user's runtime. Requires only Python's standard library (Unix).
"""
import errno
import fcntl
import os
from pathlib import Path
import pty
import re
import select
import shlex
import struct
import subprocess
import tempfile
import termios
import time
import unittest


ROOT = Path(__file__).resolve().parents[2]
BIN = ROOT / "target" / "debug"
VALUE = "((:name :echo :node \"alpha\" :interface ((ping x) (pong y))) (:name :clock :interface ((now))))"
EXPR = "'" + VALUE
PRETTY = ('((:name :echo\n  :node "alpha"\n'
          '  :interface ((ping x) (pong y)))\n'
          ' (:name :clock :interface ((now))))\n')


class TerminalTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        # /tmp keeps the Unix socket below macOS's short path limit.
        cls.temp = tempfile.TemporaryDirectory(prefix="vrs-pretty-", dir="/tmp")
        cls.socket = str(Path(cls.temp.name) / "runtime.sock")
        cls.daemon = subprocess.Popen(
            [BIN / "vrsd", "--node", "format-test", "--node-port", "0", "--socket", cls.socket],
            stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        deadline = time.monotonic() + 10
        while not Path(cls.socket).exists():
            if cls.daemon.poll() is not None or time.monotonic() > deadline:
                cls.tearDownClass()
                raise RuntimeError("Test daemon failed to start")
            time.sleep(0.02)

    @classmethod
    def tearDownClass(cls):
        cls.daemon.terminate()
        cls.daemon.wait(timeout=5)
        cls.temp.cleanup()

    def command(self, *args):
        return [str(BIN / "vrsctl"), "--socket", self.socket, *args]

    def pipe(self, *args, source=None):
        return subprocess.run(self.command(*args), input=source, text=True,
                              stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                              timeout=5, check=True).stdout

    def terminal(self, *args, stdin=False, columns=40):
        master, slave = pty.openpty()
        fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 24, columns, 0, 0))
        proc = subprocess.Popen(self.command(*args),
                                stdin=slave if stdin else subprocess.DEVNULL,
                                stdout=slave, stderr=slave,
                                env={**os.environ, "TERM": "xterm-256color"})
        os.close(slave)

        def cleanup():
            if proc.poll() is None:
                # In the REPL, terminate before normal exit so no user history
                # file is saved. All runtime processes belong to our daemon.
                proc.terminate()
            proc.wait(timeout=5)
            os.close(master)
        self.addCleanup(cleanup)
        return proc, master

    def read_until(self, master, expected):
        output = b""
        deadline = time.monotonic() + 5
        while time.monotonic() < deadline:
            if select.select([master], [], [], 0.1)[0]:
                try:
                    chunk = os.read(master, 65536)
                except OSError as error:
                    if error.errno == errno.EIO:
                        break
                    raise
                if not chunk:
                    break
                output += chunk
                clean = re.sub(r"\x1b\[[0-?]*[ -/]*[@-~]", "", output.decode()).replace("\r", "")
                if expected in clean:
                    return clean
        self.fail(f"Expected {expected!r} in PTY output: {output!r}")

    def test_command_terminal_auto_width_and_compact_override(self):
        for args, expected in [((), PRETTY), (("--format", "compact"), VALUE + "\n")]:
            proc, master = self.terminal(*args, "-c", EXPR)
            self.read_until(master, expected)
            self.assertEqual(proc.wait(timeout=5), 0)

    def test_pipes_files_and_explicit_width(self):
        self.assertEqual(self.pipe("-c", EXPR), VALUE + "\n")
        self.assertEqual(self.pipe("--format", "pretty", "--width", "40", "-c", EXPR), PRETTY)
        self.assertEqual(self.pipe(source=EXPR), VALUE + "\n")
        source = Path(self.temp.name) / "values.ll"
        source.write_text(EXPR)
        self.assertEqual(self.pipe(str(source)), VALUE + "\n")
        proc, master = self.terminal(str(source))
        self.read_until(master, PRETTY)
        self.assertEqual(proc.wait(timeout=5), 0)
        proc, master = self.terminal("--width", "200", "-c", EXPR)
        self.read_until(master, VALUE + "\n")
        self.assertEqual(proc.wait(timeout=5), 0)

    def test_pretty_builtin_raw_strings_and_editor_comments(self):
        self.assertEqual(self.pipe("--raw", "-c", f"(pretty {EXPR} 40)"), PRETTY)
        self.assertEqual(self.pipe("--format", "pretty", "-c", r'"one\n\"two\""'), r'"one\n\"two\""' + "\n")
        transcript = self.pipe("--format", "editor", "--width", "45", "-c", EXPR)
        self.assertEqual(transcript.splitlines()[0], EXPR)
        self.assertTrue(all(line.startswith("# ") for line in transcript.splitlines()[1:]))

    def test_repl_pretty_and_compact(self):
        for args, expected in [((), PRETTY), (("--format", "compact"), VALUE + "\n")]:
            _, master = self.terminal(*args, stdin=True)
            self.read_until(master, "vrs> ")
            os.write(master, (EXPR + "\n").encode())
            self.read_until(master, expected)

    def test_quotation_round_trips_through_client_and_runtime(self):
        source = "(let ((name 'focus_window) (window '(:id 7))) `(continue_call ',name '(,window)))"
        self.assertEqual(self.pipe("-c", source), "(continue_call 'focus_window '((:id 7)))\n")
        self.assertEqual(self.pipe("-c", "'(unquote @name)"), ", @name\n")
        self.assertEqual(self.pipe("-c", f"(eq? (read (pretty {source} 8)) {source})"), "true\n")
        self.assertEqual(self.pipe("-c", "(let ((xs '(1 2))) `(a ,@xs))"), "(a 1 2)\n")

    def test_macros_expand_as_data_and_work_across_file_requests(self):
        self.assertEqual(self.pipe("-c", "(macroexpand_1 '(when! true (missing)))"),
                         "(if true (begin (missing)) nil)\n")
        source = "(defmacro plus_one (x) `(+ ,x 1))\n(plus_one! 41)\n"
        self.assertEqual(self.pipe(source=source), "plus_one\n42\n")
        self.assertEqual(self.pipe("-c", "(list (err? (try (plus_one! 1))) (when! true 42))"),
                         "(true 42)\n")

    def test_subscriptions_once_follow_and_clear(self):
        for index, mode in enumerate([(), ("-f",), ("-F",)]):
            topic = f"format_test_{index}"
            proc, master = self.terminal("-s", topic, *mode)
            # Publish repeatedly until the subscriber is ready; no arbitrary
            # assumption about process startup or subscription timing.
            deadline = time.monotonic() + 5
            while not select.select([master], [], [], 0.03)[0]:
                self.pipe("-c", f"(publish :{topic} {EXPR})")
                self.assertLess(time.monotonic(), deadline)
            self.read_until(master, PRETTY)
            if mode:
                self.pipe("-c", f"(publish :{topic} '(:second 42))")
                self.read_until(master, "(:second 42)\n")
                self.assertIsNone(proc.poll())
                fcntl.ioctl(master, termios.TIOCSWINSZ, struct.pack("HHHH", 24, 200, 0, 0))
                self.pipe("-c", f"(publish :{topic} {EXPR})")
                self.read_until(master, VALUE + "\n")
            else:
                self.assertEqual(proc.wait(timeout=5), 0)

    @unittest.skipUnless(os.environ.get("JANET_MODE_DIR"), "Set JANET_MODE_DIR to include Emacs integration")
    def test_emacs_evaluation_against_test_runtime(self):
        subprocess.run(
            ["emacs", "-Q", "--batch", "-L", os.environ["JANET_MODE_DIR"],
             "-L", str(ROOT / "emacs"), "-l", "lyric-mode-tests",
             "-f", "ert-run-tests-batch-and-exit"],
            env={**os.environ, "LYRIC_TEST_VRSCTL": shlex.join(self.command())},
            check=True, timeout=20)


if __name__ == "__main__":
    unittest.main(verbosity=2)
