// The GUI owns history, not page behavior. Callbacks/arguments stay opaque here.
export const rootPage = () => ({ get_items: "root_items", args: "(())", prompt: "Search", debounce_ms: 0 });
export const retentionMs = 8 * 60 * 1000;

export class Navigation {
    constructor(transport, render, timers = globalThis) {
        this.transport = transport;
        this.render = render;
        this.timers = timers;
        this.frames = [];
        this.timer = null;
        this.pending = null;
        this.flight = null;
        this.action = null;
        this.visible = false;
        this.hiddenAt = null;
        this.opening = null;
    }
    get current() { return this.frames.at(-1); }
    snapshot() {
        const frame = this.current;
        return { page: frame?.page, query: frame?.query ?? "", items: frame?.items ?? [],
            selected: frame?.selected ?? 0, loading: Boolean(frame?.loading || this.action),
            error: frame?.error ?? "", canBack: this.frames.length > 1, visible: this.visible };
    }
    changed() { this.render(this.snapshot()); }
    invalidate() {
        if (this.timer !== null) this.timers.clearTimeout(this.timer);
        this.timer = null;
        this.pending = null;
        if (this.flight) this.flight.obsolete = true;
        if (this.action) this.action.obsolete = true;
        this.action = null;
    }
    open(page = rootPage(), query = "") {
        this.invalidate();
        this.frames = [];
        this.visible = true;
        this.push(page, query);
    }
    async begin() {
        if (this.frames.length && this.hiddenAt !== null && Date.now() - this.hiddenAt < retentionMs) {
            this.visible = true;
            this.hiddenAt = null;
            const frames = this.frames;
            const root = frames[0];
            const opening = {};
            this.opening = opening;
            const request = { frame: root, obsolete: false, bootstrap: true };
            // Keep navigation, but don't offer Save against yesterday's tab.
            if (this.current === root) { this.action = request; root.loading = true; }
            this.changed();
            try {
                const result = await this.transport.begin();
                if (this.opening !== opening || this.frames !== frames || !this.visible) return;
                if (result.type !== "push_page") throw new Error("Expected an initial page");
                root.page = result.page;
                root.loaded = false;
            } catch (error) {
                if (this.opening !== opening || this.frames !== frames || !this.visible) return;
                root.error = String(error);
                // Invalidate the old context on capture failure.
                root.page = rootPage();
                root.loaded = false;
                root.items = [];
                root.loading = false;
                if (this.action === request) this.action = null;
                if (this.current === root) { this.changed(); return; }
            }
            if (this.action === request) this.action = null;
            // A request abandoned on hide must be retried, not left spinning.
            if (!this.current.loaded || this.current.loading) this.search(this.current.query, true);
            else this.changed();
            return;
        }
        this.invalidate();
        this.frames = [{ page: rootPage(), query: "", items: [], selected: 0, loading: false, loaded: false, error: "" }];
        this.visible = true;
        this.hiddenAt = null;
        const request = { frame: this.current, obsolete: false, bootstrap: true };
        this.action = request;
        this.changed();
        try {
            const result = await this.transport.begin();
            if (request.obsolete || !this.visible) return;
            if (result.type !== "push_page") throw new Error("Expected an initial page");
            this.open(result.page, request.frame.query);
        } catch (error) {
            if (!request.obsolete && this.visible) {
                this.current.error = String(error);
                this.action = null;
                this.changed();
            }
        }
    }
    push(page, query = "") {
        this.invalidate();
        this.frames.push({ page, query: "", items: [], selected: 0, loading: false, loaded: false, error: "" });
        this.search(query, true);
    }
    search(text, immediate = false) {
        if (!this.visible || !this.current) return;
        // Keep fast typing while context is captured, without rendering a root
        // page that is missing its server-supplied arguments.
        if (this.action?.bootstrap) {
            this.current.query = text;
            this.changed();
            return;
        }
        this.invalidate();
        const frame = this.current;
        frame.query = text;
        frame.loading = true;
        frame.error = "";
        this.changed();
        const enqueue = () => {
            this.timer = null;
            this.pending = { frame, text, obsolete: false };
            this.pump();
        };
        const delay = immediate ? 0 : (frame.page.debounce_ms ?? 0);
        if (delay > 0) this.timer = this.timers.setTimeout(enqueue, delay);
        else enqueue();
    }
    async pump() {
        if (this.flight || !this.pending || !this.visible) return;
        const request = this.pending;
        this.pending = null;
        this.flight = request;
        try {
            const items = await this.transport.query(request.frame.page, request.text);
            if (!request.obsolete && this.visible && this.current === request.frame) {
                request.frame.items = items;
                request.frame.selected = 0;
                request.frame.loaded = true;
            }
        } catch (error) {
            if (!request.obsolete && this.visible && this.current === request.frame) {
                request.frame.error = String(error);
                request.frame.items = [];
                request.frame.selected = 0;
                request.frame.loaded = false;
            }
        } finally {
            if (!request.obsolete) request.frame.loading = false;
            this.flight = null;
            this.changed();
            this.pump();
        }
    }
    select(index) {
        if (!this.current || this.current.loading || this.action) return;
        const count = this.current.items.length;
        this.current.selected = count ? (index + count) % count : 0;
        this.changed();
    }
    async activate(index = this.current?.selected) {
        const frame = this.current;
        const item = frame?.items[index];
        if (!item || frame.loading || this.action || !this.visible) return;
        const request = { frame, obsolete: false };
        this.action = request;
        frame.error = "";
        this.changed();
        try {
            const result = await this.transport.dispatch(item.on_click);
            if (request.obsolete || !this.visible || this.current !== frame) return;
            this.action = null;
            if (result.type === "push_page") this.push(result.page);
            else if (result.type === "close") this.close();
            else throw new Error("Unrecognized navigation response");
        } catch (error) {
            if (!request.obsolete && this.visible && this.current === frame) frame.error = String(error);
        } finally {
            if (this.action === request) this.action = null;
            this.changed();
        }
    }
    back() {
        if (this.frames.length <= 1) { this.close(); return; }
        this.invalidate();
        this.frames.pop();
        this.current.loading = false;
        if (!this.current.loaded) this.search(this.current.query, true);
        else this.changed();
    }
    suspend() {
        if (!this.visible) return;
        this.invalidate();
        this.visible = false;
        this.hiddenAt = Date.now();
        this.opening = null;
        this.changed();
    }
    close() {
        // TODO: Propagate Escape cancellation through VRS/Lyric. Abandoning the
        // response does not interrupt a command or undo completed side effects.
        this.suspend();
        this.transport.close();
    }
}
