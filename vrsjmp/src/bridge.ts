import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { createTransport } from "./transport.mjs";
import type { Bridge, UiConfig } from "./protocol";

export async function nativeBridge(): Promise<Bridge> {
  return {
    native: true,
    preview: await invoke<boolean>("preview_mode"),
    transport: createTransport(invoke),
    show: () => invoke("show"),
    blur: () => invoke("on_blur"),
    config: () => invoke<UiConfig>("ui_config"),
    listen: (event, callback) => listen(event, callback),
  };
}
