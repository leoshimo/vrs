import type { Item } from "../protocol";
export function ResultRow({
  item,
  index,
  selected,
  disabled,
  onSelect,
  onActivate,
}: {
  item: Item;
  index: number;
  selected: boolean;
  disabled: boolean;
  onSelect: () => void;
  onActivate: () => void;
}) {
  return (
    <button
      type="button"
      id={`result-${index}`}
      className="result-row relative flex w-full items-center gap-3 rounded-[14px] px-3 py-[6px] text-left"
      role="option"
      aria-selected={selected}
      disabled={disabled}
      tabIndex={-1}
      data-rich={Boolean(item.subtitle)}
      data-multiline={item.subtitle?.includes("\n") || undefined}
      onPointerMove={onSelect}
      onMouseDown={(event) => event.preventDefault()}
      onClick={onActivate}
    >
      <span className="min-w-0 flex-1">
        <span className="block truncate text-sm leading-5">{item.title}</span>
        {item.subtitle && (
          <span className="row-subtitle block truncate text-[11px] leading-4">
            {item.subtitle_spans?.length
              ? item.subtitle_spans.map((part, i) =>
                  part.matched ? <mark key={i}>{part.text}</mark> : part.text,
                )
              : item.subtitle}
          </span>
        )}
      </span>
      {item.aside && (
        <span className="row-aside max-w-[27%] shrink-0 truncate text-right text-[11px] leading-4">
          {item.aside}
        </span>
      )}
    </button>
  );
}
