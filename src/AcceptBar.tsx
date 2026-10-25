import { useEffect, useState } from "react";

export function AcceptBar({ directApply, expiresAtMs, canCopy, busy, onAccept, onCopy, canRunApi, onRunApi, compact = false }: {
  directApply: boolean; expiresAtMs?: number; canCopy: boolean; busy: boolean;
  onAccept: () => void; onCopy: () => void;
  canRunApi?: boolean; onRunApi?: () => void;
  compact?: boolean;
}) {
  // Each capture mounts a new bar. Renders/edits/results never renew its lease.
  const [deadline] = useState(() => performance.now() + Math.max(0, Math.min(30_000, (expiresAtMs ?? 0) - Date.now())));
  const remaining = () => Math.max(0, Math.min(deadline - performance.now(), (expiresAtMs ?? 0) - Date.now()));
  const [ms, setMs] = useState(remaining);
  useEffect(() => {
    const timer = window.setInterval(() => setMs(remaining()), 200);
    return () => window.clearInterval(timer);
  }, [deadline, expiresAtMs]);
  const expired = ms <= 0;
  return <footer className="accept-bar" aria-label="Review actions">
    {onRunApi && <button type="button" className="secondary-button api-run-button" data-testid="run-api" disabled={!canRunApi || busy} onClick={onRunApi}>Run API Deep</button>}
    <p className={directApply && expired ? "expired" : ""} title="Accept expires 30 seconds after capture, including model wait and review. Copy remains available.">
      {directApply ? expired
        ? "Accept expired — select and capture again, or Copy."
        : compact ? `Accept: ${Math.ceil(ms / 1000)}s left · 30s from capture`
        : `Accept: ${Math.ceil(ms / 1000)}s remaining · maximum 30s from capture, including model wait and review.`
        : "Copy-only — paste manually into the intended field."}
    </p>
    {directApply && <span className="sr-only" role="status">{expired ? "Accept expired" : "Accept available; target is checked again on use"}</span>}
    <div className="accept-bar-actions">
      {directApply && <button type="button" className="primary-button" data-testid="apply-result"
        aria-label="Accept replacement in captured field" disabled={!canCopy || busy || expired}
        onClick={() => { if (remaining() > 0) onAccept(); else setMs(0); }}>Accept</button>}
      <button type="button" className="primary-button" data-testid="copy-result" aria-label="Copy result"
        disabled={!canCopy || busy} onClick={onCopy}>Copy</button>
    </div>
  </footer>;
}
