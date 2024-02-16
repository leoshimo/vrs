const { invoke } = window.__TAURI__.tauri;

let rootEl;
let inputEl;
let outputListEl;

async function dispatch(form) {
    await invoke("dispatch", { form: form });
}

async function setQuery(query) {
    const items = await invoke("set_query", { query: query });
    outputListEl.replaceChildren();
    for (const item of items) {
        outputListEl.appendChild(itemElement(item));
    }
}

/// Given an query item, return HTML element for rendering query
function itemElement(query_item) {
    const itemEl = document.createElement('div')

    itemEl.classList = ['item'];
    itemEl.textContent = query_item['title'];
    itemEl.addEventListener('click', (e) => {
        dispatch(query_item['on_click']);
    });

    const itemMeta = document.createElement("item__meta");
    itemMeta.classList = ['item__meta'];
    itemMeta.textContent = "Meta";
    itemEl.appendChild(itemMeta);

    return itemEl;
}

window.addEventListener("DOMContentLoaded", () => {
    rootEl = document.querySelector(".root");

    inputEl = document.querySelector("#input-field");
    inputEl.addEventListener("input", (e) => {
        e.preventDefault();
        setQuery(inputEl.value);
    });

    outputListEl = document.querySelector("#output-list");
});

window.onfocus = function() {
    inputEl.focus();
};
