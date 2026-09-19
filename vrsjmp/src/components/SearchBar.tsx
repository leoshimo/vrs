import type { ReactNode, Ref } from "react";
export function SearchBar({
  avatar,
  pageTitle,
  onBack,
  value,
  placeholder,
  activeId,
  loading,
  onChange,
  inputRef,
}: {
  avatar: ReactNode;
  pageTitle?: string;
  onBack?: () => void;
  value: string;
  placeholder: string;
  activeId?: string;
  loading: boolean;
  onChange: (value: string) => void;
  inputRef: Ref<HTMLInputElement>;
}) {
  return (
    <div
      className="search-bar surface flex items-center"
      data-tauri-drag-region
    >
      {avatar}
      <input
        ref={inputRef}
        className="min-w-0 flex-1 border-0 bg-transparent p-0 outline-none"
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
      {onBack && (
        <button
          type="button"
          className="page-tag"
          onClick={onBack}
          aria-label={`Back from ${pageTitle ?? "page"} (Escape)`}
          title={pageTitle}
        >
          {!value && <span>{pageTitle}</span>}
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" aria-hidden="true">
            <path d="m12 5-7 7 7 7M5 12h14" />
          </svg>
        </button>
      )}
      {import.meta.env.DEV && <span className="dev-badge" title="Development build">dev</span>}
    </div>
  );
}
