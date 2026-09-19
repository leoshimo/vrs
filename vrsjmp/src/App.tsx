import { useEffect, useLayoutEffect, useRef, useState, type KeyboardEvent } from "react";
import { flushSync } from "react-dom";
import { Navigation } from "./navigation.mjs";
import { createShowHandler } from "./transport.mjs";
import { menuEntries, defaultMenuSelection } from "./action-menu.mjs";
import { defaultUiConfig, type Bridge, type Item, type Snapshot } from "./protocol";
import { useAppearance } from "./hooks/useAppearance";
import { useAvatarEvents } from "./hooks/useAvatarEvents";
import { pointerSelection } from "./pointer-selection.mjs";
import { PalettePresence, presentedSnapshot } from "./palette-presence.mjs";
import { animatePalette } from "./palette-motion";
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
  const [rendered, setRendered] = useState(false);
  const onComplete = useRef(() => {});
  const palette = useRef<HTMLDivElement>(null);
  const reducedMotion = useRef(false);
  const [presence] = useState(() => new PalettePresence({
    show: () => bridge.show(),
    hide: () => bridge.transport.close(),
    onHidden: () => setRendered(false),
    prepare: () => {
      setRendered(true);
      if (!palette.current) return;
      if (palette.current.dataset.presence === "hidden" || !palette.current.dataset.presence) {
        palette.current.style.opacity = "0";
        palette.current.style.transform = reducedMotion.current ? "scale(1)" : "scale(.99)";
      }
      palette.current.style.visibility = "visible";
    },
    animate: (appearing: boolean, reason?: "dismiss" | "complete") =>
      animatePalette(palette.current, appearing, reducedMotion.current, reason),
  }));
  const [navigation] = useState(() => new Navigation({
    ...bridge.transport,
    close: (reason) => {
      if (reason === "complete") onComplete.current();
      void presence.hide(reason).catch(console.error);
    },
  }, setState));
  const [config, setConfig] = useState(defaultUiConfig);
  const [menu, setMenu] = useState<Menu | null>(null);
  const [message, setMessage] = useState("");
  const input = useRef<HTMLInputElement>(null);
  const presented = useRef(false);
  const blurred = useRef(false);
  const reopen = useRef<(() => Promise<void>) | null>(null);
  const [pointerMoved] = useState(pointerSelection);
  const { dark, reduced } = useAppearance();
  reducedMotion.current = reduced;
  const lastView = useRef<Snapshot | undefined>(undefined);
  const view: Snapshot = presentedSnapshot(state, lastView.current);
  useLayoutEffect(() => { lastView.current = view; }, [view]);
  const lastMenu = useRef<Menu | null>(null);
  useLayoutEffect(() => { if (state.visible) lastMenu.current = menu; }, [menu, state.visible]);
  const shownMenu = state.visible ? menu : lastMenu.current;
  const avatar = useAvatarEvents(reduced, state.visible || rendered, state.visible && view.loading);
  onComplete.current = avatar.complete;
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
  const openMenu = (row = state.selected) => {
    const item = state.items[row];
    if (state.loading || !item?.actions.some((action) => !action.primary)) return;
    navigation.select(row);
    setMenu({ item, row, query: "", selected: defaultMenuSelection(menuEntries(item)) });
    focus();
  };
  const toggleMenu = () => {
    if (menu) {
      closeMenu();
      return;
    }
    openMenu();
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
      blurred.current = false;
      pointerMoved.reset();
      if (!presented.current) flushSync(() => onOpen.current());
      await presence.show();
      if (disposed || !navigation.visible) return;
      presented.current = true;
      focus();
    });
    reopen.current = opening;
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
      reopen.current = null;
      unlisten.forEach((off) => off());
      navigation.dispose();
      presence.dispose();
    };
  }, [bridge, navigation, pointerMoved, presence]);
  useEffect(() => {
    if (!bridge.native || bridge.preview) return;
    let blurFrame = 0;
    const blur = () => {
      cancelAnimationFrame(blurFrame);
      blurFrame = requestAnimationFrame(() => {
        if (document.hasFocus() || !presented.current) return;
        blurred.current = true;
        setMenu(null);
        navigation.suspend();
        void presence.hide().catch(console.error);
      });
    };
    const regainFocus = () => {
      if (blurred.current && palette.current?.dataset.presence === "closing") {
        blurred.current = false;
        void reopen.current?.().catch(console.error);
      }
      focus();
    };
    window.addEventListener("blur", blur);
    window.addEventListener("focus", regainFocus);
    return () => {
      cancelAnimationFrame(blurFrame);
      window.removeEventListener("blur", blur);
      window.removeEventListener("focus", regainFocus);
    };
  }, [bridge, navigation, presence]);
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
    const revealSelection = () => {
      const r = row.getBoundingClientRect(),
        l = list.getBoundingClientRect();
      if (r.top < l.top + 4) list.scrollTop -= l.top + 4 - r.top;
      else if (r.bottom > l.bottom - 4) list.scrollTop += r.bottom - l.bottom + 4;
    };
    revealSelection();
    const resize = new ResizeObserver(revealSelection);
    resize.observe(list);
    return () => resize.disconnect();
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
  return (
    <main
      className="palette-stage"
      data-theme={config.theme}
      data-appearance={dark ? "dark" : "light"}
      data-reduced-motion={reduced}
      onKeyDown={onKeyDown}
      onPointerMoveCapture={(event) => {
        if (!pointerMoved(event)) event.stopPropagation();
      }}
      onPointerDown={(event) => {
        if (event.target === event.currentTarget) navigation.close();
      }}
    >
      <div
        ref={palette}
        className="palette flex w-full flex-col"
        inert={!state.visible}
      >
        <SearchBar
          inputRef={input}
          pageTitle={shownMenu?.item.title ?? view.page?.title}
          onBack={shownMenu ? closeMenu : view.canBack ? () => { navigation.back(); focus(); } : undefined}
          value={shownMenu?.query ?? view.query}
          placeholder={
            shownMenu
              ? "Search actions…"
              : view.page?.get_items === "root_items" && !view.page.on_cancel
                ? "Search…"
                : (view.page?.prompt ?? "Search…")
          }
          activeId={activeId}
          loading={view.loading}
          onChange={onChange}
          avatar={
            <Avatar
              player={avatar.player}
              revision={avatar.revision}
              inputRef={input}
              active={rendered}
              dark={dark}
              reduced={reduced}
            />
          }
        />
        <ActivityView
          active={state.visible}
          reduced={reduced}
          pageKey={
            shownMenu
              ? `actions:${shownMenu.item.id}`
              : `${view.page?.get_items}:${view.page?.args}:${view.items.length ? "rows" : view.loading ? "loading" : "empty"}`
          }
        >
          {shownMenu ? (
            <ActionMenu
              item={shownMenu.item}
              entries={menuEntries(shownMenu.item, shownMenu.query)}
              selected={shownMenu.selected}
              onSelect={(selected) => {
                if (state.visible && shownMenu.selected !== selected) setMenu({ ...shownMenu, selected });
              }}
              onActivate={(index) => activate(shownMenu.row, index)}
            />
          ) : (
            <div
              id="activity-list"
              role="listbox"
              aria-label={view.page?.title ?? "Commands"}
              aria-busy={state.loading}
              className="activity-list"
            >
              {view.items.map((item, index) => (
                <ResultRow
                  key={`${item.id}:${index}`}
                  item={item}
                  index={index}
                  selected={view.selected === index}
                  disabled={state.loading}
                  onSelect={() => {
                    if (state.selected !== index) navigation.select(index);
                  }}
                  onActivate={() => activate(index)}
                  onActions={() => openMenu(index)}
                />
              ))}
              {!view.items.length && (
                <div className="empty-state" role="status">
                  {view.error ? (
                    <>
                      <span>Couldn’t load items</span>
                      <button
                        className="mt-2 underline underline-offset-4"
                        onClick={() => navigation.search(state.query, true)}
                      >
                        Try again
                      </button>
                    </>
                  ) : view.loading ? (
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
