export type ItemCommand = { title: string; primary: boolean; on_click: string };
export type Item = {
  id: string;
  title: string;
  subtitle?: string | null;
  subtitle_spans?: { text: string; matched: boolean }[];
  aside?: string | null;
  actions: ItemCommand[];
  on_click: string;
};
export type Page = {
  title: string;
  prompt: string;
  get_items: string;
  args: string;
  debounce_ms: number;
  on_cancel?: string | null;
};
export type Action = { type: "close" | "refresh" } | { type: "push_page"; page: Page };
export type Snapshot = {
  page?: Page;
  query: string;
  items: Item[];
  selected: number;
  loading: boolean;
  error: string;
  canBack: boolean;
  visible: boolean;
};
export type UiConfig = {
  theme: "neutral" | "warm" | "cool";
};
export const defaultUiConfig: UiConfig = { theme: "neutral" };
export type Transport = {
  begin(): Promise<Action>;
  query(page: Page, query: string): Promise<Item[]>;
  dispatch(form: string): Promise<Action>;
  close(reason?: "dismiss" | "complete"): void;
};
export type Bridge = {
  native: boolean;
  transport: Transport;
  show(): Promise<void>;
  blur(): Promise<void>;
  config(): Promise<UiConfig>;
  listen(event: string, callback: () => void): Promise<() => void>;
};
