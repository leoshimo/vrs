import {
  useEffect,
  useLayoutEffect,
  useRef,
  useState,
  type CSSProperties,
  type KeyboardEvent,
} from "react";
import { flushSync } from "react-dom";
import { Navigation } from "./navigation.mjs";
import { createShowHandler } from "./transport.mjs";
import { menuEntries, defaultMenuSelection } from "./action-menu.mjs";
import { defaultUiConfig, type Bridge, type Item, type Snapshot } from "./protocol";
import { useAppearance } from "./hooks/useAppearance";
import { useAvatarEvents } from "./hooks/useAvatarEvents";
import { appearanceMask } from "./avatar/appearance-mask";
import { Avatar } from "./components/Avatar";
import { SearchBar } from "./components/SearchBar";
import { ActivityView } from "./components/ActivityView";
import { ResultRow } from "./components/ResultRow";
import { ActionMenu, type MenuEntry } from "./components/ActionMenu";
import { Toast } from "./components/Toast";

const initial: Snapshot = {
  query: "",
  items: [],
  selected: 0,
  loading: false,
  error: "",
  canBack: false,
  visible: false,
};
type Menu = { item: Item; row: number; query: string; selected: number };

export function App({ bridge }: { bridge: Bridge }) {
  const [state, setState] = useState(initial);
  const [navigation] = useState(() => new Navigation(bridge.transport, setState));
  const [config, setConfig] = useState(defaultUiConfig);
  const [menu, setMenu] = useState<Menu | null>(null);
  const [message, setMessage] = useState("");
  const [working, setWorking] = useState(false);
  const input = useRef<HTMLInputElement>(null);
  const presented = useRef(false);
  const { dark, reduced } = useAppearance(config);
  const avatar = useAvatarEvents(reduced);
  const onOpen = useRef(avatar.open);
  onOpen.current = avatar.open;
  const selected = state.items[state.selected];
  const entries: MenuEntry[] = menu ? menuEntries(menu.item, menu.query) : [];
  const activeId = menu
    ? entries[menu.selected]
      ? `action-${entries[menu.selected].index}`
      : undefined
    : selected
      ? `result-${state.selected}`
      : undefined;
  const focus = () => input.current?.focus({ preventScroll: true });
  const closeMenu = () => {
    setMenu(null);
    focus();
  };
  const toggleMenu = () => {
    if (menu) {
      closeMenu();
      return;
    }
    if (state.loading || !selected?.actions.some((action) => !action.primary)) return;
    setMenu({
      item: selected,
      row: state.selected,
      query: "",
      selected: defaultMenuSelection(menuEntries(selected)),
    });
    focus();
  };
  const activate = (row: number, action?: number) => {
    if (state.loading) return;
    avatar.submit();
    setMenu(null);
    void navigation.activate(row, action);
    focus();
  };
  useLayoutEffect(() => {
    if (state.visible) focus();
    else presented.current = false;
  }, [state.visible, Boolean(menu)]);
  useEffect(() => {
    let disposed = false;
    let revision = 0;
    const unlisten: (() => void)[] = [];
    const refreshConfig = async () => {
      const request = ++revision;
      try {
        const next = await bridge.config();
        if (!disposed && revision === request) setConfig(next);
      } catch (error) {
        console.warn("Could not refresh VRS appearance:", error);
      }
    };
    const opening = createShowHandler(navigation, async () => {
      if (disposed) return;
      if (!presented.current) flushSync(() => onOpen.current());
      await bridge.show();
      if (disposed || !navigation.visible) return;
      presented.current = true;
      focus();
    });
    async function start() {
      for (const [event, callback] of [
        ["show-palette", () => void opening().catch(console.error)],
        [
          "toggle-palette",
          () => (navigation.visible ? navigation.close() : void opening().catch(console.error)),
        ],
        ["vrs-config-changed", () => void refreshConfig()],
        [
          "vrs-connected",
          () => {
            void refreshConfig();
            void opening({ background: true }).catch(console.error);
          },
        ],
      ] as const) {
        const off = await bridge.listen(event, callback);
        if (disposed) {
          off();
          return;
        }
        unlisten.push(off);
      }
      await refreshConfig();
      if (!disposed) await opening();
    }
    void start().catch((error) => setMessage(String(error)));
    return () => {
      disposed = true;
      unlisten.forEach((off) => off());
      navigation.dispose();
    };
  }, [bridge, navigation]);
  useEffect(() => {
    if (!bridge.native) return;
    const blur = () => {
      setMenu(null);
      navigation.suspend();
      void bridge.blur().catch(console.error);
    };
    window.addEventListener("blur", blur);
    window.addEventListener("focus", focus);
    return () => {
      window.removeEventListener("blur", blur);
      window.removeEventListener("focus", focus);
    };
  }, [bridge, navigation]);
  useEffect(() => {
    const interact = () => {
      if (navigation.visible) navigation.interact();
    };
    const events = ["keydown", "pointerdown", "pointermove", "wheel", "input"];
    events.forEach((event) =>
      window.addEventListener(event, interact, { passive: true, capture: true }),
    );
    return () => events.forEach((event) => window.removeEventListener(event, interact, true));
  }, [navigation]);
  useEffect(() => {
    if (!state.loading || !state.visible) {
      setWorking(false);
      return;
    }
    const timer = setTimeout(() => setWorking(true), 180);
    return () => clearTimeout(timer);
  }, [state.loading, state.visible]);
  useEffect(() => {
    if (state.loading || !state.visible || selected?.id !== menu?.item.id) setMenu(null);
  }, [state.loading, state.visible, selected?.id, menu?.item.id]);
  useEffect(() => {
    setMessage(state.error);
    if (!state.error) return;
    const timer = setTimeout(() => setMessage(""), 6000);
    return () => clearTimeout(timer);
  }, [state.error]);
  useLayoutEffect(() => {
    const row = activeId ? document.getElementById(activeId) : null;
    const list = document.getElementById("activity-list");
    if (!row || !list) return;
    const r = row.getBoundingClientRect(),
      l = list.getBoundingClientRect();
    if (r.top < l.top + 4) list.scrollTop -= l.top + 4 - r.top;
    else if (r.bottom > l.bottom - 4) list.scrollTop += r.bottom - l.bottom + 4;
  }, [activeId, state.items]);
  const onKeyDown = (event: KeyboardEvent) => {
    if (event.nativeEvent.isComposing) return;
    if (event.metaKey && event.key.toLowerCase() === "k") {
      event.preventDefault();
      toggleMenu();
      return;
    }
    const step =
      event.key === "ArrowDown" || (event.ctrlKey && event.key === "n")
        ? 1
        : event.key === "ArrowUp" || (event.ctrlKey && event.key === "p")
          ? -1
          : 0;
    if (event.key === "Escape") {
      event.preventDefault();
      if (menu) closeMenu();
      else navigation.back();
      return;
    }
    if (step) {
      event.preventDefault();
      if (menu)
        setMenu({
          ...menu,
          selected: entries.length ? (menu.selected + step + entries.length) % entries.length : 0,
        });
      else navigation.select(state.selected + step);
    } else if (event.key === "Enter" && event.target === input.current) {
      event.preventDefault();
      if (menu && entries[menu.selected]) activate(menu.row, entries[menu.selected].index);
      else if (!menu) activate(state.selected);
    }
  };
  const onChange = (query: string) => {
    avatar.type();
    if (menu) setMenu({ ...menu, query, selected: 0 });
    else navigation.search(query);
  };
  const revealStyle: CSSProperties = reduced
    ? {}
    : avatar.flourish
      ? {
          maskImage: appearanceMask(avatar.progress, "print"),
          maskSize: "100% 100%",
          opacity: avatar.progress === 0 ? 0 : 1,
        }
      : { opacity: avatar.progress };
  return (
    <main
      className="palette-stage"
      data-theme={config.theme}
      data-appearance={dark ? "dark" : "light"}
      data-reduced-motion={reduced}
      onKeyDown={onKeyDown}
      onPointerDown={(event) => {
        if (event.target === event.currentTarget) navigation.close();
      }}
    >
      <div
        className="palette flex w-full flex-col gap-2"
        style={{ ...revealStyle, visibility: state.visible ? "visible" : "hidden" }}
        inert={!state.visible}
      >
        <SearchBar
          inputRef={input}
          actions={
            selected?.actions.some((action) => !action.primary) && (
              <button
                type="button"
                className="search-actions flex shrink-0 items-center gap-1.5 rounded-md px-2 py-1 text-[11px]"
                disabled={state.loading}
                onClick={toggleMenu}
                aria-haspopup="listbox"
                aria-expanded={Boolean(menu)}
              >
                Actions <kbd>⌘ K</kbd>
              </button>
            )
          }
          value={menu?.query ?? state.query}
          placeholder={
            menu
              ? "Search actions…"
              : state.page?.get_items === "root_items" && !state.page.on_cancel
                ? "Search…"
                : (state.page?.prompt ?? "Search…")
          }
          activeId={activeId}
          loading={state.loading}
          onChange={onChange}
          avatar={
            <Avatar
              config={avatar.config}
              mode={avatar.mode}
              pulse={avatar.pulse}
              working={working}
              active={state.visible}
              dark={dark}
              reduced={reduced}
            />
          }
        />
        <ActivityView
          title={menu ? (selected?.title ?? "Actions") : (state.page?.title ?? "Home")}
          back={
            menu
              ? closeMenu
              : state.canBack
                ? () => {
                    navigation.back();
                    focus();
                  }
                : undefined
          }
          showHeading={Boolean(menu) || state.canBack}
          pageKey={
            menu
              ? `actions:${menu.item.id}`
              : `${navigation.frames.length}:${state.page?.get_items}:${state.page?.args}`
          }
          reduced={reduced}
        >
          {menu ? (
            <ActionMenu
              item={menu.item}
              entries={entries}
              selected={menu.selected}
              onSelect={(selected) => {
                if (menu.selected !== selected) setMenu({ ...menu, selected });
              }}
              onActivate={(index) => activate(menu.row, index)}
            />
          ) : (
            <div
              id="activity-list"
              role="listbox"
              aria-label={state.page?.title ?? "Commands"}
              aria-busy={state.loading}
              className="activity-list px-[6px] pb-[6px]"
            >
              {state.items.map((item, index) => (
                <ResultRow
                  key={`${item.id}:${index}`}
                  item={item}
                  index={index}
                  selected={state.selected === index}
                  disabled={state.loading}
                  onSelect={() => {
                    if (state.selected !== index) navigation.select(index);
                  }}
                  onActivate={() => activate(index)}
                />
              ))}
              {!state.items.length && (
                <div className="empty-state" role="status">
                  {state.error ? (
                    <>
                      <span>Couldn’t load items</span>
                      <button
                        className="mt-2 underline underline-offset-4"
                        onClick={() => navigation.search(state.query, true)}
                      >
                        Try again
                      </button>
                    </>
                  ) : state.loading ? (
                    "Working…"
                  ) : (
                    "No results"
                  )}
                </div>
              )}
            </div>
          )}
        </ActivityView>
        <Toast message={message} dismiss={() => setMessage("")} />
      </div>
    </main>
  );
}
