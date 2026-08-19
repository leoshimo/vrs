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

## Development

```shell
$ cargo tauri dev
```
