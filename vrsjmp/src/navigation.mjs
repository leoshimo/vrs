// The GUI owns history, not page behavior. Callbacks/arguments stay opaque here.
export const rootPage = () => ({ get_items: "root_items", args: "()", title: "Home", prompt: "Search…", debounce_ms: 0 });
export const retentionMs = 30 * 1000;

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
        this.lastInteractionAt = null;
        this.idleTimer = null;
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
    now() { return this.timers.now?.() ?? Date.now(); }
    interact() {
        this.lastInteractionAt = this.now();
        this.scheduleIdleReset();
    }
    scheduleIdleReset() {
        if (this.idleTimer !== null) this.timers.clearTimeout(this.idleTimer);
        this.idleTimer = null;
        if (this.lastInteractionAt === null) return;
        const remaining = retentionMs - (this.now() - this.lastInteractionAt);
        this.idleTimer = this.timers.setTimeout(() => {
            this.idleTimer = null;
            // Let an action finish before discarding its navigation state.
            if (this.action) return;
            this.invalidate();
            this.opening = null;
            this.frames = [];
            this.lastInteractionAt = null;
            if (this.visible) this.begin({ idleReset: true }).catch(console.error);
            else this.changed();
        }, Math.max(0, remaining));
    }
    resumeIdleReset() {
        if (this.idleTimer === null) this.scheduleIdleReset();
    }
    invalidate(keepAction = false) {
        if (this.timer !== null) this.timers.clearTimeout(this.timer);
        this.timer = null;
        this.pending = null;
        if (this.flight) this.flight.obsolete = true;
        if (!keepAction) {
            if (this.action) this.action.obsolete = true;
            this.action = null;
        }
    }
    open(page = rootPage(), query = "", recordInteraction = true) {
        this.invalidate();
        this.frames = [];
        this.visible = true;
        if (recordInteraction) this.interact();
        this.push(page, query);
    }
    async begin({ background = false, idleReset = false } = {}) {
        if (!this.visible) this.invalidate();
        // Reconnect checks may recover pending input without opening an idle
        // palette. The service still chooses the page through the same hook.
        if (background && !this.visible) {
            const opening = {};
            this.opening = opening;
            const result = await this.transport.begin();
            if (this.opening !== opening || this.visible) return;
            if (result.type !== "push_page") throw new Error("Expected an initial page");
            if (result.page.on_cancel) this.open(result.page);
            return;
        }
        const retain = this.frames.length && this.lastInteractionAt !== null
            && this.now() - this.lastInteractionAt < retentionMs;
        if (!idleReset) this.interact();
        if (retain) {
            this.visible = true;
            const frames = this.frames;
            const root = frames[0];
            const opening = {};
            this.opening = opening;
            const request = { frame: root, obsolete: false, bootstrap: true };
            // Keep navigation while checking for a pending input request.
            if (this.current === root) { this.action = request; root.loading = true; }
            this.changed();
            try {
                const result = await this.transport.begin();
                if (this.opening !== opening || this.frames !== frames || !this.visible) return;
                if (result.type !== "push_page") throw new Error("Expected an initial page");
                // A pending input request takes precedence over retained
                // ordinary navigation. The same request keeps its subpages.
                if ((root.page.on_cancel ?? null) !== (result.page.on_cancel ?? null)) {
                    this.open(result.page, "", false);
                    return;
                }
                root.page = result.page;
                root.loaded = false;
            } catch (error) {
                if (this.opening !== opening || this.frames !== frames || !this.visible) return;
                root.error = String(error);
                // Clear the old page if the initial-page request fails.
                root.page = rootPage();
                root.loaded = false;
                root.items = [];
                root.loading = false;
                if (this.action === request) this.action = null;
                if (this.current === root) { this.changed(); return; }
            } finally {
                if (this.action === request) this.action = null;
                this.resumeIdleReset();
            }
            // A request abandoned on hide must be retried, not left spinning.
            if (!this.current.loaded || this.current.loading) this.search(this.current.query, true, true);
            else this.changed();
            return;
        }
        this.invalidate();
        this.frames = [{ page: rootPage(), query: "", items: [], selected: 0, loading: false, loaded: false, error: "" }];
        this.visible = true;
        const request = { frame: this.current, obsolete: false, bootstrap: true };
        this.action = request;
        this.changed();
        try {
            const result = await this.transport.begin();
            if (request.obsolete || !this.visible) return;
            if (result.type !== "push_page") throw new Error("Expected an initial page");
            this.open(result.page, request.frame.query, false);
        } catch (error) {
            if (!request.obsolete && this.visible) {
                this.current.error = String(error);
                this.action = null;
                this.changed();
            }
        } finally {
            if (this.action === request) this.action = null;
            this.resumeIdleReset();
        }
    }
    push(page, query = "") {
        this.invalidate();
        if (!page.on_cancel && this.current?.page.on_cancel) {
            page = { ...page, on_cancel: this.current.page.on_cancel };
        }
        this.frames.push({ page, query: "", items: [], selected: 0, loading: false, loaded: false, error: "" });
        this.search(query, true);
    }
    search(text, immediate = false, preserveSelection = false) {
        if (!this.visible || !this.current) return;
        if (!immediate) this.interact();
        // Keep fast typing while the service chooses the initial page, without
        // rendering a page that is missing its server-supplied arguments.
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
            this.pending = { frame, text, obsolete: false,
                selectedId: preserveSelection ? frame.items[frame.selected]?.id : undefined };
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
                const selected = request.selectedId === undefined ? 0
                    : items.findIndex(item => item.id === request.selectedId);
                request.frame.selected = Math.max(0, selected);
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
        this.interact();
        const count = this.current.items.length;
        this.current.selected = count ? (index + count) % count : 0;
        this.changed();
    }
    async activate(index = this.current?.selected, actionIndex = null) {
        const frame = this.current;
        const primary = frame?.items[index];
        const item = actionIndex === null ? primary : primary?.actions?.[actionIndex];
        if (!item || frame.loading || this.action || !this.visible) return;
        this.interact();
        frame.selected = index;
        const request = { frame, obsolete: false };
        this.action = request;
        frame.error = "";
        this.changed();
        try {
            const result = await this.transport.dispatch(item.on_click);
            if (request.obsolete || this.current !== frame) return;
            this.action = null;
            if (result.type === "close") {
                frame.query = "";
                frame.items = [];
                frame.selected = 0;
                frame.loaded = false;
                if (this.visible) this.close(false);
            } else if (result.type === "refresh") {
                frame.loaded = false;
                if (this.visible) this.search(frame.query, true, true);
            } else if (this.visible) {
                if (result.type === "push_page") this.push(result.page);
                else throw new Error("Unrecognized navigation response");
            }
        } catch (error) {
            if (!request.obsolete && this.visible && this.current === frame) frame.error = String(error);
        } finally {
            if (this.action === request) this.action = null;
            this.resumeIdleReset();
            this.changed();
        }
    }
    back() {
        this.interact();
        if (this.frames.length <= 1) { this.close(); return; }
        this.invalidate();
        const leaving = this.frames.pop();
        if (leaving.page.on_cancel !== this.current.page.on_cancel) this.cancelPage(leaving.page);
        this.current.loading = false;
        if (!this.current.loaded) this.search(this.current.query, true);
        else this.changed();
    }
    suspend() {
        if (!this.visible) return;
        // An action can move focus before its reply arrives. Let completion
        // clear its query while hidden; reopening invalidates the old action.
        this.invalidate(true);
        this.visible = false;
        this.interact();
        this.opening = null;
        this.changed();
    }
    dispose() {
        this.invalidate();
        if (this.idleTimer !== null) this.timers.clearTimeout(this.idleTimer);
        this.idleTimer = null;
        this.lastInteractionAt = null;
        this.opening = null;
        this.visible = false;
        this.render = () => {};
    }
    cancelPage(page) {
        if (page?.on_cancel) Promise.resolve(this.transport.dispatch(page.on_cancel)).catch(console.error);
    }
    close(cancel = true) {
        if (cancel) this.cancelPage(this.current?.page);
        this.suspend();
        this.transport.close();
    }
}
