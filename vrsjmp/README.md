# vrsjmp

A launch bar GUI client for [vrs](https://github.com/leoshimo/vrs/).

Choose **Browse Services** from Home, select a registered service, then search
its exported functions by signature or documentation. Selecting a function
prompts for its arguments and invokes it. Typed arguments use the service's
completion providers, falling back to providers already configured in vrsjmp;
otherwise enter a Lyric expression such as `"hello"`, `42`, or `'(a b)`.
Escape returns to the previous page.

The service browser reads the live registry, including services that vrsjmp has
not imported. Browsing leaves vrsjmp's function bindings intact. **Browse
Functions** continues to search functions already bound in vrsjmp.

## Failed actions

Failed actions remain searchable on Home with their original title, **Failed**,
the error, and the captured call. Select the row to retry, or choose **Retry**
or **Dismiss** from its action menu. A retry uses the original argument values,
even after switching apps or browser tabs. Actions run in separate processes;
return to Home to see their latest outcome. Repeated activation of a running
retry does not start another attempt.

Items keep ordinary calls, such as `:on_click (save_page (active_tab))`. The
shared click handler evaluates each argument once and retains the resulting
call. It recognizes functions with existing `interactive` or bound-service
metadata; completed interactive calls also retain their filled arguments.
Palette navigation and macro controls keep their synchronous behavior. Compound
forms such as `begin` use normal execution without retry capture. Capture stops
at the outer call: input read inside a function's body is read again on retry.

Failures survive closing the window, but restarting or reloading the vrsjmp
service clears them. Retry starts the whole captured call again, including any
effects that previously completed. Timeouts keep their original error text;
they do not imply cancellation of the remote operation. No automatic retry is
performed. Items use the existing title, aside, multiline subtitle, and actions.

## Display resolution

Choose **Display Resolution** from Home. Selecting a resolution keeps the page
and search open, preserves the selected mode, and refreshes the **(current)**
marker from the display. The marker is only a label; that row can be selected
again after switching modes. Unavailable modes and failed changes show an error.

Page actions can return `:refresh` to query the current page again without
closing it or adding a history entry.

## MacBook keyboard backlight

On the MacBook node, `:os_keyboard` uses macOS's built-in `osascript` and its
Objective-C bridge to access the private CoreBrightness interface. No separate
executable or installation is needed. API and hardware failures surface as errors.

Search Home for **Toggle Keyboard Backlight**. It changes 0% to 5% and any
positive brightness to 0%; it does not remember the previous level.

The same operations are available under **Browse Services → :os_keyboard** or
from Lyric:

```lisp
(bind_srv :os_keyboard)
(get_keyboard_brightness)
(set_keyboard_brightness 5)
(toggle_keyboard_backlight)
```

All three functions return macOS's brightness readback as an integer percentage.
To load changes into a running node, evaluate `scripts/os_keyboard.ll`, then
`scripts/vrsjmp.ll`. Normal startup loads both from `scripts/init.ll`.

## Development

```shell
$ cargo tauri dev
```
