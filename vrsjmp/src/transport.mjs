// `callback` is reserved by Tauri's IPC envelope. Page data calls it get_items;
// use renderer at the bridge so it cannot overwrite the response callback ID.
export const createTransport = invoke => ({
    begin: () => invoke("root_page"),
    query: (page, query) => invoke("set_query", { renderer: page.get_items, args: page.args, query }),
    dispatch: form => invoke("dispatch", { form }),
    close: () => invoke("hide").catch(console.error),
});

// Resolve the initial page before showing. A cancelled or superseded opening
// must not show the window when its delayed response finally arrives.
export function createOpening(navigation, show) {
    let latest;
    return async options => {
        const opening = {};
        latest = opening;
        await navigation.begin(options);
        if (latest === opening && navigation.visible) await show();
    };
}

// A wakeup focuses an existing input flow, and bursts share one opening.
// An explicit close followed by an open starts a new, cancellable operation.
export function createShowHandler(navigation, show) {
    const open = createOpening(navigation, show);
    let running;
    return (options = {}) => {
        if (running && (navigation.visible || options.background)) return running;
        if (!options.background && navigation.visible && navigation.current?.page.on_cancel) {
            return Promise.resolve(show());
        }
        const operation = open(options);
        running = operation;
        const done = () => { if (running === operation) running = null; };
        operation.then(done, done);
        return operation;
    };
}
