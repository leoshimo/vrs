import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { createTransport } from "./transport.mjs";
import type { Bridge, UiConfig } from "./protocol";

export function nativeBridge(): Bridge {
  return {
    native: true,
    transport: createTransport(invoke),
    show: () => invoke("show"),
    blur: () => invoke("on_blur"),
    config: () => invoke<UiConfig>("ui_config"),
    listen: (event, callback) => listen(event, callback),
  };
}
