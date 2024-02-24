// `callback` is reserved by Tauri's IPC envelope. Page data calls it get_items;
// use renderer at the bridge so it cannot overwrite the response callback ID.
export const createTransport = invoke => ({
    begin: () => invoke("begin_interaction"),
    query: (page, query) => invoke("set_query", { renderer: page.get_items, args: page.args, query }),
    dispatch: form => invoke("dispatch", { form }),
    close: () => invoke("hide").catch(console.error),
});

// Capture through the service before taking focus. A cancelled or superseded
// opening must not show the window when its delayed response finally arrives.
export function createOpening(navigation, show) {
    let latest;
    return async () => {
        const opening = {};
        latest = opening;
        await navigation.begin();
        if (latest === opening && navigation.visible) await show();
    };
}
