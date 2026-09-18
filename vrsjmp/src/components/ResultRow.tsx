import type { Item } from "../protocol";
export function ResultRow({
  item,
  index,
  selected,
  disabled,
  onSelect,
  onActivate,
  onActions,
}: {
  item: Item;
  index: number;
  selected: boolean;
  disabled: boolean;
  onSelect: () => void;
  onActivate: () => void;
  onActions: () => void;
}) {
  const hasActions = item.actions.some((action) => !action.primary);
  return (
    <div
      id={`result-${index}`}
      className="result-row relative flex w-full items-center text-left"
      role="option"
      aria-selected={selected}
      aria-disabled={disabled}
      tabIndex={-1}
      data-rich={Boolean(item.subtitle)}
      data-multiline={item.subtitle?.includes("\n") || undefined}
      onPointerMove={() => { if (!disabled) onSelect(); }}
      onMouseDown={(event) => event.preventDefault()}
      onClick={() => { if (!disabled) onActivate(); }}
      onContextMenu={(event) => {
        if (!hasActions) return;
        event.preventDefault();
        if (!disabled) onActions();
      }}
    >
      <span className="min-w-0 flex-1">
        <span className="block truncate">{item.title}</span>
        {item.subtitle && (
          <span className="row-subtitle block truncate">
            {item.subtitle_spans?.length
              ? item.subtitle_spans.map((part, i) =>
                  part.matched ? <mark key={i}>{part.text}</mark> : part.text,
                )
              : item.subtitle}
          </span>
        )}
      </span>
      {item.aside && (
        <span className="row-aside max-w-[27%] shrink-0 truncate text-right">
          {item.aside}
        </span>
      )}
      {hasActions && (
        <button
          type="button"
          className="row-actions"
          disabled={disabled}
          tabIndex={-1}
          aria-label={`Actions for ${item.title}`}
          aria-haspopup="listbox"
          title="Actions (⌘K)"
          onClick={(event) => {
            event.stopPropagation();
            onActions();
          }}
        >
          <svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor" aria-hidden="true">
            <circle cx="12" cy="7" r="1.6" /><circle cx="12" cy="12" r="1.6" /><circle cx="12" cy="17" r="1.6" />
          </svg>
        </button>
      )}
    </div>
  );
}
