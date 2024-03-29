import { Navigation } from "./navigation.mjs";
import { createShowHandler, createTransport } from "./transport.mjs";
import { menuEntries, defaultMenuSelection } from "./action-menu.mjs";

const { invoke } = window.__TAURI__.tauri;
const input = document.querySelector("#input-field");
const output = document.querySelector("#output-list");
const status = document.querySelector("#status");
const back = document.querySelector("#back");
const loading = document.querySelector("#loading");
const toast = document.querySelector("#toast");
const pageTitle = document.querySelector("#page-title");
const count = document.querySelector("#count");
const actions = document.querySelector("#actions");
const menu = document.querySelector("#action-menu");
const actionList = document.querySelector("#action-list");
const actionFilter = document.querySelector("#action-filter");
const actionSearch = document.querySelector("#action-search");
const actionStatus = document.querySelector("#action-status");
let loadingTimer = null, toastTimer = null;
let previousItems = null, previousSelected = null, previousError = "";
let menuItem = null;
let menuRow = -1, menuSelected = 0;

function closeMenu(focus = false) {
    menu.hidden = true;
    menuItem = null;
    menuRow = -1;
    actionFilter.value = "";
    actionSearch.dataset.active = "false";
    input.readOnly = false;
    actions.setAttribute("aria-expanded", "false");
    if (focus) input.focus();
}

function textSpan(className, value) {
    const span = document.createElement("span");
    span.className = className;
    span.textContent = value;
    return span;
}

const navigation = new Navigation(createTransport(invoke), state => {
    if (input.value !== state.query) input.value = state.query;
    input.placeholder = state.page?.prompt ?? "Search commands…";
    input.setAttribute("aria-label", input.placeholder);
    input.setAttribute("aria-busy", String(state.loading));
    pageTitle.textContent = state.page?.title ?? state.page?.prompt ?? "Home";
    back.hidden = !state.canBack;
    status.textContent = !state.loading && !state.error && !state.items.length ? "No results" : "";
    count.textContent = state.items.length + (state.items.length === 1 ? " item" : " items");
    if (state.loading && state.visible) {
        if (loadingTimer === null) loadingTimer = setTimeout(() => { loading.hidden = false; }, 250);
    } else {
        clearTimeout(loadingTimer);
        loadingTimer = null;
        loading.hidden = true;
    }
    if (state.error !== previousError) {
        clearTimeout(toastTimer);
        previousError = state.error;
        toast.textContent = state.error;
        toast.hidden = !state.error;
        if (state.error) toastTimer = setTimeout(() => { toast.hidden = true; }, 6000);
    }
    const selected = state.items[state.selected];
    actions.hidden = !selected?.actions?.length;
    actions.disabled = state.loading;
    if (state.loading || !state.visible || (menuItem && menuItem !== selected)) closeMenu();
    const changedItems = state.items !== previousItems;
    if (changedItems) {
        output.replaceChildren();
        state.items.forEach((item, index) => {
            const element = document.createElement("button");
            element.type = "button";
            element.id = "result-" + index;
            element.className = "item";
            element.dataset.rich = String(Boolean(item.subtitle));
            element.setAttribute("role", "option");
            element.setAttribute("aria-label", [item.title, item.subtitle, item.aside].filter(Boolean).join(" — "));
            const copy = document.createElement("span");
            copy.className = "item-copy";
            copy.append(textSpan("item-title", item.title));
            if (item.subtitle) {
                const subtitle = textSpan("item-subtitle", "");
                if (item.subtitle_spans?.length) {
                    for (const span of item.subtitle_spans) {
                        if (span.matched) {
                            const mark = document.createElement("mark");
                            mark.textContent = span.text;
                            subtitle.append(mark);
                        } else subtitle.append(document.createTextNode(span.text));
                    }
                } else subtitle.textContent = item.subtitle;
                copy.append(subtitle);
            }
            element.append(copy);
            if (item.aside) element.append(textSpan("item-aside", item.aside));
            element.addEventListener("click", () => { navigation.activate(index); input.focus(); });
            output.appendChild(element);
        });
        previousItems = state.items;
    }
    Array.from(output.children).forEach((element, index) => {
        element.classList.toggle("item--focus", index === state.selected);
        element.setAttribute("aria-selected", String(index === state.selected));
        element.setAttribute("aria-disabled", String(state.loading));
    });
    if (selected) input.setAttribute("aria-activedescendant", "result-" + state.selected);
    else input.removeAttribute("aria-activedescendant");
    if (changedItems || previousSelected !== state.selected) {
        output.children[state.selected]?.scrollIntoView({ block: "nearest" });
        previousSelected = state.selected;
    }
});

function selectMenu(index) {
    const buttons = Array.from(actionList.children);
    menuSelected = buttons.length ? (index + buttons.length) % buttons.length : 0;
    buttons.forEach((button, i) => {
        button.classList.toggle("action--focus", i === menuSelected);
        button.setAttribute("aria-selected", String(i === menuSelected));
    });
    if (buttons.length) {
        actionFilter.setAttribute("aria-activedescendant", buttons[menuSelected].id);
        buttons[menuSelected].scrollIntoView({block: "nearest"});
    } else actionFilter.removeAttribute("aria-activedescendant");
}

function renderMenu() {
    const entries = menuEntries(menuItem, actionFilter.value);
    actionList.replaceChildren();
    entries.forEach(({ command, index }, visibleIndex) => {
        const button = document.createElement("button");
        button.type = "button";
        button.role = "option";
        button.tabIndex = -1;
        button.id = "action-" + index;
        button.dataset.separator = String(!actionFilter.value.trim() && visibleIndex > 0
            && entries[visibleIndex - 1].command.primary && !command.primary);
        const icon = document.createElementNS("http://www.w3.org/2000/svg", "svg");
        icon.classList.add("action-icon");
        icon.setAttribute("viewBox", "0 0 24 24");
        icon.setAttribute("aria-hidden", "true");
        const use = document.createElementNS("http://www.w3.org/2000/svg", "use");
        const name = ["open", "link", "copy", "command"].includes(command.icon) ? command.icon : "command";
        use.setAttribute("href", "assets/action-icons.svg#" + name);
        icon.append(use);
        const key = document.createElement("kbd");
        key.textContent = "↵";
        key.setAttribute("aria-hidden", "true");
        button.append(icon, textSpan("action-label", command.title), key);
        button.addEventListener("pointermove", () => selectMenu(visibleIndex));
        button.onclick = () => {
            // Menu actions apply to the exact row that opened the menu.
            if (navigation.current?.items[menuRow] !== menuItem) { closeMenu(true); return; }
            const row = menuRow;
            closeMenu(true);
            navigation.activate(row, index);
        };
        actionList.append(button);
    });
    actionStatus.hidden = actionList.children.length > 0;
    selectMenu(defaultMenuSelection(entries, actionFilter.value));
}

function toggleMenu() {
    if (!menu.hidden) { closeMenu(true); return; }
    const state = navigation.snapshot();
    const item = state.items[state.selected];
    if (state.loading || !item?.actions?.length) return;
    menuItem = item;
    menuRow = state.selected;
    actionFilter.value = "";
    actionSearch.dataset.active = "false";
    document.querySelector("#action-title").textContent = item.title;
    menu.hidden = false;
    input.readOnly = true;
    actions.setAttribute("aria-expanded", "true");
    renderMenu();
    actionFilter.focus({ preventScroll: true });
}

const revealActionSearch = () => { actionSearch.dataset.active = "true"; };
actionFilter.addEventListener("beforeinput", revealActionSearch);
actionFilter.addEventListener("compositionstart", revealActionSearch);
actionFilter.addEventListener("input", () => { revealActionSearch(); renderMenu(); });
actionList.addEventListener("pointerdown", event => event.preventDefault());
input.addEventListener("input", () => { closeMenu(); navigation.search(input.value); });
window.addEventListener("keydown", event => {
    if (event.isComposing) return;
    if (event.metaKey && event.key.toLowerCase() === "k") { event.preventDefault(); toggleMenu(); return; }
    const step = event.key === "ArrowDown" || (event.ctrlKey && event.key === "n") ? 1
        : event.key === "ArrowUp" || (event.ctrlKey && event.key === "p") ? -1 : 0;
    if (!menu.hidden) {
        if (event.key === "Escape") {
            event.preventDefault();
            closeMenu(true);
        }
        else if (step) {
            event.preventDefault();
            selectMenu(menuSelected + step);
        } else if (event.key === "Enter") {
            event.preventDefault();
            actionList.children[menuSelected]?.click();
        }
        return;
    }
    if (event.key === "Escape") { event.preventDefault(); navigation.back(); }
    else if (event.key === "Enter" && (event.target === input || event.target.closest(".item"))) {
        event.preventDefault();
        const row = event.target.closest(".item");
        navigation.activate(row ? Array.from(output.children).indexOf(row) : undefined);
    } else if (step) {
        event.preventDefault(); navigation.select((navigation.current?.selected ?? 0) + step);
    }
});
document.addEventListener("pointerdown", event => {
    if (!menu.hidden && !menu.contains(event.target) && !actions.contains(event.target)) closeMenu();
});
actions.addEventListener("click", toggleMenu);
back.addEventListener("click", () => { navigation.back(); input.focus(); });
window.addEventListener("focus", () => input.focus());
window.addEventListener("blur", () => {
    closeMenu();
    navigation.suspend();
    invoke("on_blur").catch(console.error);
});
const openPalette = createShowHandler(navigation, () => invoke("show"));
await window.__TAURI__.event.listen("show-palette", () => {
    openPalette().catch(console.error);
});
await window.__TAURI__.event.listen("vrs-connected", () => {
    openPalette({ background: true }).catch(console.error);
});
await window.__TAURI__.event.listen("toggle-palette", () => {
    if (navigation.visible) navigation.close();
    else openPalette().catch(console.error);
});
openPalette().catch(console.error);
