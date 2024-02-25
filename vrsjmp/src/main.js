const { invoke } = window.__TAURI__.tauri;

let rootEl;
let inputEl;
let outputListEl;
let focusedEl;

async function dispatch(form) {
    await invoke("dispatch", { form: form });
}

async function setQuery(query) {
    focusedEl = null;
    const items = await invoke("set_query", { query: query });
    outputListEl.replaceChildren();
    let isFirst = true;
    for (const item of items) {
        const itemEl = itemElement(item);
        outputListEl.appendChild(itemEl);
        if (isFirst) {
            focusItem(itemEl);
            isFirst = false;
        }
    }
}

/// Given an query item, return HTML element for rendering query
function itemElement(query_item) {
    const itemEl = document.createElement('div')

    itemEl.classList = ['item'];
    itemEl.textContent = query_item['title'];
    itemEl.addEventListener('click', (e) => {
        dispatch(query_item['on_click']);
        inputEl.value = '';
        setQuery('');
    });

    // const itemMeta = document.createElement("item__meta");
    // itemMeta.classList = ['item__meta'];
    // itemMeta.textContent = "Meta";
    // itemEl.appendChild(itemMeta);

    return itemEl;
}

function focusItem(newEl) {
    console.log(newEl);
    if (focusedEl) {
        focusedEl.classList.remove('item--focus');
        focusedEl = null;
    }
    focusedEl = newEl;
    if (focusedEl) {
        focusedEl.classList.add('item--focus');
        focusedEl.scrollIntoView({
            behavior: "auto",
            block: "nearest",
            inline: "nearest",
        });
    }
}

window.onkeyup = function(e){
    if (e.key === 'ArrowDown' || (e.key === 'n' && e.ctrlKey)) {
        focusItem(focusedEl.nextSibling || outputListEl.firstChild);
    }
    if (e.key === 'ArrowUp' || (e.key === 'p' && e.ctrlKey)) {
        focusItem(focusedEl.previousSibling || outputListEl.lastChild);
    }
}

window.addEventListener("DOMContentLoaded", () => {
    rootEl = document.querySelector(".root");

    setQuery("");

    inputEl = document.querySelector("#input-field");
    inputEl.addEventListener("input", (e) => {
        e.preventDefault();
        setQuery(inputEl.value);
    });
    inputEl.addEventListener("keydown", (e) => {
        if (e.key == 'Enter') {
            if (focusedEl != null) {
                focusedEl.click();
            }
        }
    });

    outputListEl = document.querySelector("#output-list");
});

window.onfocus = function() {
    inputEl.value = '';
    setQuery('');
    inputEl.focus();
    focusItem(outputListEl.firstChild);
};

window.onblur = function() {
    invoke("on_blur");
};
