'use client';
import { CircleCheck, Check, Pin } from 'lucide-react';

export function ExpressionActions({
  name,
  assignmentLabel,
  matches,
  pinned,
  onApply,
  onPin,
}: {
  name: string;
  assignmentLabel: string;
  matches: boolean;
  pinned: boolean;
  onApply: () => void;
  onPin: () => void;
}) {
  const applyLabel = matches
    ? `${name} is assigned to ${assignmentLabel}`
    : `Apply ${name} to ${assignmentLabel}`;
  return (
    <div className="expression-actions">
      <button
        className="expression-apply"
        disabled={matches}
        data-matches={matches}
        aria-label={applyLabel}
        title={applyLabel}
        onClick={onApply}
      >
        {matches ? <Check size={15} /> : <CircleCheck size={15} />}
      </button>
      <button
        className="expression-pin"
        aria-label={`${pinned ? 'Unpin' : 'Pin'} ${name}`}
        title={`${pinned ? 'Unpin' : 'Pin'} ${name}`}
        aria-pressed={pinned}
        onClick={onPin}
      >
        <Pin size={15} />
      </button>
    </div>
  );
}
