import { createRoot } from "react-dom/client";
import { isTauri } from "@tauri-apps/api/core";
import { App } from "./App";
import { nativeBridge } from "./bridge";
import "./styles.css";

async function main() {
  const root = createRoot(document.getElementById("root")!);
  if (isTauri()) root.render(<App bridge={await nativeBridge()} />);
  else if (import.meta.env.DEV && new URLSearchParams(location.search).has("preview")) {
    const { Preview } = await import("../test/Preview");
    root.render(<Preview />);
  } else root.render(<p>Open vrsjmp through Tauri.</p>);
}
void main();
