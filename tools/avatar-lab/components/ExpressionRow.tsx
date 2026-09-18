'use client';
import { Check, Pin } from 'lucide-react';
export function ExpressionRow({
  name,
  description,
  selected,
  assigned,
  pinned,
  onSelect,
  onPin,
}: {
  name: string;
  description?: string;
  selected: boolean;
  assigned: boolean;
  pinned?: boolean;
  onSelect: () => void;
  onPin?: () => void;
}) {
  return (
    <div className="expression-row" data-selected={selected}>
      <button
        className="expression-row-select"
        aria-pressed={selected}
        onClick={onSelect}
      >
        <span>
          {name}
          {assigned && <Check size={13} aria-label="Assigned" />}
        </span>
        {description && <small>{description}</small>}
      </button>
      {onPin && (
        <button
          className="expression-pin"
          aria-label={`${pinned ? 'Unpin' : 'Pin'} ${name}`}
          aria-pressed={pinned}
          title={pinned ? 'Unpin' : 'Pin'}
          onClick={onPin}
        >
          <Pin size={15} />
        </button>
      )}
    </div>
  );
}
