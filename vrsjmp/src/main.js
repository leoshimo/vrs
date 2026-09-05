import { Navigation } from "./navigation.mjs";
import { createOpening, createTransport } from "./transport.mjs";

const { invoke } = window.__TAURI__.tauri;
const input = document.querySelector("#input-field");
const output = document.querySelector("#output-list");
const status = document.querySelector("#status");
const back = document.querySelector("#back");
const loading = document.querySelector("#loading");
const toast = document.querySelector("#toast");
let loadingTimer = null, toastTimer = null;
let previousItems = null, previousSelected = null, previousError = "";

const navigation = new Navigation(createTransport(invoke), state => {
    if (input.value !== state.query) input.value = state.query;
    input.placeholder = state.page?.prompt ?? "Search";
    input.setAttribute("aria-busy", String(state.loading));
    back.hidden = !state.canBack;
    status.textContent = !state.loading && !state.error && !state.items.length ? "No results" : "";
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
    const changedItems = state.items !== previousItems;
    if (changedItems) {
        output.replaceChildren();
        state.items.forEach((item, index) => {
        const element = document.createElement("div");
        element.className = "item";
        element.setAttribute("role", "option");
        element.textContent = item.title;
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
    if (changedItems || previousSelected !== state.selected) {
        output.children[state.selected]?.scrollIntoView({ block: "nearest" });
        previousSelected = state.selected;
    }
});

input.addEventListener("input", () => navigation.search(input.value));
window.addEventListener("keydown", event => {
    if (event.isComposing) return;
    if (event.key === "Escape") { event.preventDefault(); navigation.back(); }
    else if (event.key === "Enter") { event.preventDefault(); navigation.activate(); }
    else if (event.key === "ArrowDown" || (event.ctrlKey && event.key === "n")) {
        event.preventDefault(); navigation.select((navigation.current?.selected ?? 0) + 1);
    } else if (event.key === "ArrowUp" || (event.ctrlKey && event.key === "p")) {
        event.preventDefault(); navigation.select((navigation.current?.selected ?? 0) - 1);
    }
});
back.addEventListener("click", () => { navigation.back(); input.focus(); });
window.addEventListener("focus", () => {
    input.focus();
});
window.addEventListener("blur", () => {
    navigation.suspend();
    invoke("on_blur").catch(console.error);
});
const openPalette = createOpening(navigation, () => invoke("show"));
await window.__TAURI__.event.listen("toggle-palette", () => {
    if (navigation.visible) navigation.close();
    else openPalette().catch(console.error);
});
openPalette().catch(console.error);
