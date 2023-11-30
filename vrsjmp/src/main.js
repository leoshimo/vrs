const { invoke } = window.__TAURI__.tauri;

let inputEl;
let outputEl;

async function runEval(input) {
    outputEl.textContent = await invoke("eval", { input: input });
}

window.addEventListener("DOMContentLoaded", () => {
    inputEl = document.querySelector("#input-field");
    outputEl = document.querySelector("#output");
    document.querySelector("#input-form").addEventListener("submit", (e) => {
        e.preventDefault();
        runEval(inputEl.value);
    });
});
