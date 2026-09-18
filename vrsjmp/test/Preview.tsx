import { useMemo, useState } from "react";
import { App } from "../src/App";
import type { Bridge, Item, Page, UiConfig } from "../src/protocol";
import wallpaper from "../../tools/avatar-lab/public/tahoe.png";

const page = (title: string, get_items: string): Page => ({
  title,
  get_items,
  args: "()",
  prompt: "Search…",
  debounce_ms: 0,
});
const item = (title: string, id = title, subtitle?: string, aside?: string): Item => ({
  id,
  title,
  subtitle,
  aside,
  subtitle_spans: [],
  actions: [],
  on_click: id,
});
const articles = [
  item(
    "A programmable environment for everyday work",
    "article-1",
    "Ink & Switch · Research notes",
    "12 min",
  ),
  item("The web as a place to think", "article-2", "Maggie Appleton · Design", "Yesterday"),
  item("Small tools, loosely joined", "article-3", "Personal computing · Essays", "8 min"),
  item("Reading and remembering", "article-4", "Andy Matuschak · Notes", "Sep 12"),
  item(
    "How we build shared understanding",
    "article-5",
    "Local-first software · Collaboration",
    "16 min",
  ),
  item(
    "A very long title to check truncation without overlapping the trailing metadata or the selection background",
    "article-6",
    "A similarly long source description for checking ellipsis and selection contrast",
    "Saved today",
  ),
].map((row) => ({
  ...row,
  actions: [
    { title: "Open in Browser", primary: true, on_click: "open" },
    { title: "Copy URL", primary: false, on_click: "copy" },
    { title: "Remove from Read Later", primary: false, on_click: "remove" },
  ],
}));
const home = [
  item("Read Later", "read"),
  item("Browse Functions", "functions"),
  item("Browse Services", "services"),
  item("Browser History", "history"),
  item("iCloud Tabs", "tabs"),
  item("Windows", "windows"),
  item("Display Resolution", "display"),
  item("Toggle Keyboard Backlight", "keyboard"),
  item("One primary action", "primary"),
  item("One additional action", "secondary"),
  item("Save to Read Later", "save", undefined, "4 second request"),
];
home[8].actions = [{ title: "Run", primary: true, on_click: "primary" }];
home[9].actions = [{ title: "Inspect", primary: false, on_click: "inspect" }];
const scenarios = ["Home", "Read Later", "Two items", "No results", "Error", "Working"] as const;
type Scenario = (typeof scenarios)[number];
function previewBridge(
  scenario: Scenario,
): Bridge & { emit: (event: string) => void; setConfig: (value: UiConfig) => void } {
  const listeners = new Map<string, Set<() => void>>();
  let config: UiConfig = { theme: "neutral", appearance: "system" };
  const emit = (event: string) => listeners.get(event)?.forEach((fn) => fn());
  const root = page(
    scenario === "Read Later" ? "Read Later" : "Home",
    scenario === "Read Later" ? "reading" : "home",
  );
  return {
    native: false,
    emit,
    setConfig(value) {
      config = value;
      emit("vrs-config-changed");
    },
    async show() {},
    async blur() {},
    async config() {
      return config;
    },
    async listen(event, callback) {
      const set = listeners.get(event) ?? new Set();
      set.add(callback);
      listeners.set(event, set);
      return () => {
        set.delete(callback);
      };
    },
    transport: {
      async begin() {
        return { type: "push_page", page: root };
      },
      async query(p, query) {
        if (scenario === "Working") await new Promise((resolve) => setTimeout(resolve, 1800));
        else if (p.get_items === "reading" && !query) await new Promise((resolve) => setTimeout(resolve, 800));
        if (scenario === "Error" || query === "error")
          throw new Error("The service could not complete this request. Try again.");
        const rows =
          scenario === "No results"
            ? []
            : scenario === "Two items"
              ? articles.slice(0, 2)
              : p.get_items === "reading"
                ? articles
                : home;
        return rows.filter((row) =>
          `${row.title} ${row.subtitle ?? ""}`.toLowerCase().includes(query.toLowerCase()),
        );
      },
      async dispatch(form) {
        if (form === "save") {
          await new Promise((resolve) => setTimeout(resolve, 4000));
          return { type: "close" };
        }
        if (form === "functions") return { type: "push_page", page: page("A long current page title that should stay on one line", "reading") };
        if (form === "read") return { type: "push_page", page: page("Read Later", "reading") };
        if (form === "tabs") return { type: "push_page", page: page("iCloud Tabs", "reading") };
        if (form === "copy") throw new Error("Preview message: copied the selected article URL.");
        return { type: "refresh" };
      },
      close() {},
    },
  };
}
export function Preview() {
  const [scenario, setScenario] = useState<Scenario>("Home");
  const [config, setConfig] = useState<UiConfig>({ theme: "neutral", appearance: "system" });
  const bridge = useMemo(() => previewBridge(scenario), [scenario]);
  const change = (next: UiConfig) => {
    setConfig(next);
    bridge.setConfig(next);
  };
  return (
    <div
      className="flex h-full flex-col items-center overflow-auto"
      style={{ background: `url(${wallpaper}) center/cover` }}
    >
      <div
        className="w-full max-w-[688px] shrink-0"
        style={{ height: "min(560px,calc(100dvh - 84px))" }}
      >
        <App key={scenario} bridge={bridge} />
      </div>
      <div className="mx-3 flex max-w-3xl flex-wrap justify-center gap-2 rounded-lg bg-white/95 p-3 text-xs text-black shadow">
        {scenarios.map((name) => (
          <button
            key={name}
            aria-pressed={scenario === name}
            className="rounded border px-2 py-1 aria-pressed:bg-gray-200"
            onClick={() => {
              setScenario(name);
              setConfig({ theme: "neutral", appearance: "system" });
            }}
          >
            {name}
          </button>
        ))}
        <button className="rounded border px-2 py-1" onClick={() => bridge.emit("toggle-palette")}>
          Toggle appearance
        </button>
        <select
          aria-label="Theme"
          value={config.theme}
          onChange={(e) => change({ ...config, theme: e.target.value as UiConfig["theme"] })}
        >
          <option value="neutral">Neutral</option>
          <option value="warm">Warm</option>
          <option value="cool">Cool</option>
        </select>
        <select
          aria-label="Appearance"
          value={config.appearance}
          onChange={(e) =>
            change({ ...config, appearance: e.target.value as UiConfig["appearance"] })
          }
        >
          <option value="system">System</option>
          <option value="light">Light</option>
          <option value="dark">Dark</option>
        </select>
      </div>
    </div>
  );
}
