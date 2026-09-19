'use client';
export function ResetTweaks({
  onClick,
  label = 'Modified',
  title = 'Reset tweaks',
  action = 'Reset',
}: {
  onClick: () => void;
  label?: string;
  title?: string;
  action?: string;
}) {
  return (
    <button
      className="reset-tweaks"
      onClick={onClick}
      aria-label={title}
      title={title}
    >
      <span aria-hidden="true">{label}</span>
      <span aria-hidden="true">{action}</span>
    </button>
  );
}
