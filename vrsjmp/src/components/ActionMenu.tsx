import type { Item, ItemCommand } from "../protocol";
export type MenuEntry = { command: ItemCommand; index: number };
export function ActionMenu({
  item,
  entries,
  selected,
  onSelect,
  onActivate,
}: {
  item: Item;
  entries: MenuEntry[];
  selected: number;
  onSelect: (index: number) => void;
  onActivate: (index: number) => void;
}) {
  return (
    <div
      id="activity-list"
      role="listbox"
      aria-label={`Actions for ${item.title}`}
      className="activity-list"
    >
      {entries.length ? (
        entries.map(({ command, index }, i) => (
          <button
            type="button"
            role="option"
            aria-selected={i === selected}
            id={`action-${index}`}
            key={index}
            tabIndex={-1}
            className="result-row flex w-full items-center text-left"
            data-separator={
              (i > 0 && !command.primary && entries[i - 1].command.primary) || undefined
            }
            onMouseDown={(event) => event.preventDefault()}
            onPointerMove={() => onSelect(i)}
            onClick={() => onActivate(index)}
          >
            <span className="min-w-0 flex-1 break-words">{command.title}</span>
            {command.primary && <kbd>↵</kbd>}
          </button>
        ))
      ) : (
        <div className="empty-state" role="status">
          No matching actions
        </div>
      )}
    </div>
  );
}
