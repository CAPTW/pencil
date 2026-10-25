import { useEffect, useRef } from "react";

export function ApiRequestDialog({ endpoint, model, onClose }: {
  endpoint: string; model: string; onClose: (accepted: boolean) => void;
}) {
  const dialog = useRef<HTMLDialogElement>(null);
  useEffect(() => { dialog.current?.showModal(); }, []);
  return <dialog ref={dialog} className="api-request-dialog" aria-labelledby="api-request-title"
    onCancel={event => { event.preventDefault(); onClose(false); }}>
    <h2 id="api-request-title">Send to API?</h2>
    <p>Send this selection and matched approved terms using <strong>{model}</strong>?</p>
    <p className="api-request-endpoint">{endpoint}</p>
    <p>Charges and the provider's retention policy may apply.</p>
    <div className="row-actions">
      <button type="button" autoFocus onClick={() => onClose(false)}>Cancel</button>
      <button type="button" className="primary-button" onClick={() => onClose(true)}>Send request</button>
    </div>
  </dialog>;
}
