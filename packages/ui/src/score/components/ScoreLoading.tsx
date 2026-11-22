import { useEffect, useState } from 'react';

/** Shows the indeterminate indicator only when a loading stage lasts at least 200 ms. */
export function ScoreLoading({ label }: { label: string }) {
  const [visible, setVisible] = useState(false);
  useEffect(() => {
    const timer = setTimeout(() => setVisible(true), 200);
    return () => clearTimeout(timer);
  }, []);
  if (!visible) return null;
  return (
    <div className="score-loading-state">
      <div className="score-loading-melody" aria-hidden="true">
        <span className="score-loading-note">♪</span>
        <span className="score-loading-note">♫</span>
        <span className="score-loading-note">♪</span>
      </div>
      <span className="score-loading-label">{label}</span>
      <progress className="score-loading-progress" aria-label={label} />
    </div>
  );
}
