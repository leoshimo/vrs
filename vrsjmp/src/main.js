const { invoke } = window.__TAURI__.tauri;

let inputEl;
let outputListEl;

async function dispatch(form) {
    await invoke("dispatch", { form: form });
}

async function setQuery(query) {
    const items = await invoke("set_query", { query: query });

    outputListEl.replaceChildren();
    for (const item of items) {
        const itemButton = document.createElement('button')
        itemButton.classList = ['output-item'];
        itemButton.innerText = item['title'];
        itemButton.addEventListener('click', (e) => {
            dispatch(item['on_click']);
        });
        outputListEl.appendChild(itemButton);
    }
}

window.addEventListener("DOMContentLoaded", () => {
    inputEl = document.querySelector("#input-field");
    outputListEl = document.querySelector("#output-list");
    inputEl.addEventListener("input", (e) => {
        e.preventDefault();
        setQuery(inputEl.value);
    });
});
