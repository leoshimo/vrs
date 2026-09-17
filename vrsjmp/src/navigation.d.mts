import type { Page, Snapshot, Transport } from "./protocol";
export function rootPage(): Page;
export const retentionMs: number;
export class Navigation {
  constructor(transport: Transport, render: (state: Snapshot) => void, timers?: unknown);
  visible: boolean;
  frames: {
    page: Page;
    query: string;
    items: Snapshot["items"];
    selected: number;
    loading: boolean;
    loaded: boolean;
    error: string;
  }[];
  get current(): Navigation["frames"][number] | undefined;
  snapshot(): Snapshot;
  interact(): void;
  begin(options?: { background?: boolean; idleReset?: boolean }): Promise<void>;
  open(page?: Page, query?: string): void;
  push(page: Page, query?: string): void;
  search(text: string, immediate?: boolean, preserveSelection?: boolean): void;
  select(index: number): void;
  activate(index?: number, actionIndex?: number | null): Promise<void>;
  back(): void;
  suspend(): void;
  dispose(): void;
  close(cancel?: boolean): void;
}
