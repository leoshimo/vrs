export function Toast({ message, dismiss }: { message: string; dismiss: () => void }) {
  if (!message) return null;
  return (
    <div
      className="toast surface flex items-start gap-3 rounded-[16px] px-[18px] py-3"
      role="alert"
    >
      <span className="max-h-20 min-w-0 flex-1 overflow-auto whitespace-pre-wrap break-words select-text">
        {message}
      </span>
      <button
        type="button"
        className="shrink-0 rounded px-1"
        onClick={dismiss}
        aria-label="Dismiss message"
      >
        ×
      </button>
    </div>
  );
}
