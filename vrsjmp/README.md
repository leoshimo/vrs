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
