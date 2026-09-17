import type { ReactNode, Ref } from "react";
export function SearchBar({
  avatar,
  actions,
  value,
  placeholder,
  activeId,
  loading,
  onChange,
  inputRef,
}: {
  avatar: ReactNode;
  actions?: ReactNode;
  value: string;
  placeholder: string;
  activeId?: string;
  loading: boolean;
  onChange: (value: string) => void;
  inputRef: Ref<HTMLInputElement>;
}) {
  return (
    <div
      className="search-bar surface flex h-12 items-center gap-2 rounded-[18px] py-[3px] pl-2 pr-3"
      data-tauri-drag-region
    >
      {avatar}
      <input
        ref={inputRef}
        className="min-w-0 flex-1 border-0 bg-transparent p-0 text-base outline-none"
        value={value}
        onChange={(event) => onChange(event.target.value)}
        placeholder={placeholder}
        aria-label={placeholder}
        role="combobox"
        aria-autocomplete="list"
        aria-controls="activity-list"
        aria-expanded="true"
        aria-activedescendant={activeId}
        aria-busy={loading}
        autoComplete="off"
        autoCorrect="off"
        autoCapitalize="off"
        spellCheck={false}
      />
      {actions}
    </div>
  );
}
