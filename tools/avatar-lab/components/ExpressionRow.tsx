'use client';
import { ExpressionActions } from './ExpressionActions';
export function ExpressionRow({
  name,
  description,
  selected,
  matches,
  pinned,
  onSelect,
  assignmentLabel,
  onPin,
  onApply,
}: {
  name: string;
  description?: string;
  selected: boolean;
  matches: boolean;
  pinned: boolean;
  onSelect: () => void;
  assignmentLabel: string;
  onPin: () => void;
  onApply: () => void;
}) {
  return (
    <div className="expression-row" data-selected={selected}>
      <button
        className="expression-row-select"
        aria-pressed={selected}
        onClick={onSelect}
      >
        <span>{name}</span>
        {description && <small>{description}</small>}
      </button>
      <ExpressionActions
        {...{ name, assignmentLabel, matches, pinned, onApply, onPin }}
      />
    </div>
  );
}
