import { invoke } from "@tauri-apps/api/core";
import { useEffect, useRef, useState } from "react";

type ApiStatus = { epoch: string; configured: boolean; endpoint: string | null; model: string | null; consent: boolean; requestEnabled: boolean };
const empty: ApiStatus = { epoch: "" , configured: false, endpoint: null, model: null, consent: false, requestEnabled: false };

export function ApiSettings() {
  const [models, setModels] = useState<{id: string; name: string; selectable: boolean}[]>([]);
  const [catalogMessage, setCatalogMessage] = useState("Model list not loaded. Actual invocation: not verified.");
  const [status, setStatus] = useState<ApiStatus>(empty);
  const [endpoint, setEndpoint] = useState("");
  const [model, setModel] = useState("");
  const [consent, setConsent] = useState(false);
  const [pending, setPending] = useState(false);
  const [message, setMessage] = useState("");
  const version = useRef(0);
  const keyInput = useRef<HTMLInputElement>(null);
  useEffect(() => {
    let mounted = true; const initialVersion = version.current;
    void invoke<ApiStatus>("api_session_status").then(value => {
      if (!mounted || version.current !== initialVersion) return;
      setStatus(value); setEndpoint(value.endpoint ?? ""); setModel(value.model ?? ""); setConsent(value.consent);
    }).catch(() => { if (mounted) setMessage("API setup is unavailable."); });
    const clear = () => {
      const action = ++version.current; setModels([]); setCatalogMessage("Model list not loaded. Actual invocation: not verified."); setPending(false); setStatus(empty); setConsent(false);
      void invoke<ApiStatus>("api_session_status").then(value => { if (mounted && version.current === action) setStatus(value); }).catch(() => {});
      if (keyInput.current) keyInput.current.value = "";
    };
    window.addEventListener("grammar-api-cleared", clear);
    return () => { mounted = false; version.current++; window.removeEventListener("grammar-api-cleared", clear); if (keyInput.current) keyInput.current.value = ""; };
  }, []);
  function resetCatalog() { setModels([]); setCatalogMessage("Model list not loaded. Actual invocation: not verified."); }
  async function loadModels() {
    if (pending) return;
    const action = ++version.current; setPending(true); resetCatalog();
    try {
      const rows = await invoke<{id: string; name: string; selectable: boolean}[]>("list_api_models", {lookup: {endpoint, key: keyInput.current?.value ?? ""}});
      if (version.current !== action) return;
      setModels(rows); setCatalogMessage(`Model list loaded: ${rows.length}. Actual invocation: not verified. Listing does not verify billing, quota or writing-response compatibility.`);
    } catch (error) {
      if (version.current !== action) return;
      const messages: Record<string,string> = {api_invalid_key:"Enter an API key first.", api_catalog_auth_denied:"Provider denied catalog access (key or permission).",api_catalog_rate_limited:"Provider rate limit reached. No retry was sent.",api_catalog_unsupported_endpoint:"Model lookup supports the four official provider presets. Enter a custom model manually.",api_catalog_timeout:"Model lookup timed out.",api_catalog_limit:"Catalog exceeded bounded limits; no partial list accepted."};
      setCatalogMessage((messages[String(error)] ?? "Model lookup failed. No partial list accepted.") + " Actual invocation: not verified.");
    } finally { if (version.current === action) setPending(false); }
  }
  async function save() {
    if (pending) return;
    const action = ++version.current; setPending(true); setMessage("");
    const key = keyInput.current?.value ?? "";
    if (keyInput.current) keyInput.current.value = "";
    try {
      const next = await invoke<ApiStatus>("configure_api_session", { setup: { expectedEpoch: status.epoch, endpoint, model, key, consent } });
      if (version.current !== action) return;
      setStatus(next); setMessage("Setup kept for this app session. No connection or text request was made.");
    } catch {
      const latest = await invoke<ApiStatus>("api_session_status").catch(() => empty);
      if (version.current !== action) return;
      setStatus(latest); setConsent(false);
      setMessage("Setup was not saved. Reload the current state and check the HTTPS endpoint, model and key.");
    } finally { if (version.current === action) setPending(false); }
  }
  async function clear() {
    resetCatalog();
    const action = ++version.current; setPending(true); setMessage("");
    if (keyInput.current) keyInput.current.value = "";
    try { const next = await invoke<ApiStatus>("clear_api_session"); if (version.current !== action) return; setStatus(next); setConsent(false); setMessage("API key and permission removed from this session."); }
    catch { if (version.current !== action) return; setMessage("Could not confirm removal. Close the app to end the session."); }
    finally { if (version.current === action) setPending(false); }
  }
  return <section className="provider-settings" aria-label="Optional API setup">
    <h3>Optional API setup</h3>
    <p>Claude Messages and OpenAI-compatible Chat Completions. Use Run API Deep on a captured selection to send one explicit request. Automatic rewriting does not use this API.</p>
    <p>The key stays in app memory for this session and is not saved to settings. Re-enter it after restarting. Saving does not contact the provider.</p>
    <label className="settings-field">API provider preset
      <select disabled={pending} value={[
        "https://api.anthropic.com/v1/messages", "https://generativelanguage.googleapis.com/v1beta/openai/chat/completions",
        "https://openrouter.ai/api/v1/chat/completions", "https://api.x.ai/v1/chat/completions"
      ].includes(endpoint) ? endpoint : ""} onChange={event => {
        resetCatalog(); setEndpoint(event.target.value); setModel(""); setConsent(false);
        if (keyInput.current) keyInput.current.value = "";
      }}>
        <option value="">Custom HTTPS / Chat Completions</option>
        <option value="https://api.anthropic.com/v1/messages">Claude (Anthropic API)</option>
        <option value="https://generativelanguage.googleapis.com/v1beta/openai/chat/completions">Google Gemini API</option>
        <option value="https://openrouter.ai/api/v1/chat/completions">OpenRouter API</option>
        <option value="https://api.x.ai/v1/chat/completions">xAI API</option>
      </select>
    </label>
    <p>Enter an exact model ID available to your API account. CLI subscriptions do not configure API access. OpenRouter requires an explicit model (no auto/router variants), disables fallback, and requires zero data retention; unsupported routes fail.</p>
    <label className="settings-field">HTTPS endpoint
      <input type="url" value={endpoint} autoComplete="off" spellCheck={false} disabled={pending} onChange={event => { resetCatalog(); setEndpoint(event.target.value); setConsent(false); if (keyInput.current) keyInput.current.value = ""; }} placeholder="https://api.example.com/v1/chat/completions" />
    </label>
    <label className="settings-field">Model
      <input value={model} autoComplete="off" spellCheck={false} disabled={pending} onChange={event => { setModel(event.target.value); setConsent(false); }} />
    </label>
    <label className="settings-field">API key (session only)
      <input ref={keyInput} type="password" autoComplete="off" spellCheck={false} disabled={pending} maxLength={4096} onChange={() => { resetCatalog(); setConsent(false); }} />
    </label>
    <button type="button" disabled={pending} onClick={() => void loadModels()}>Load model list</button>
    <p>Loading sends the entered API key to the selected provider for catalog lookup only. No document text or generation request is sent. The key remains in this form until setup is saved, removed or closed.</p>
    <label className="settings-field">Available models
      <select disabled={pending || !models.length} value={models.some(row => row.id === model) ? model : ""} onChange={event => {setModel(event.target.value); setConsent(false);}}>
        <option value="">Select a listed model (or enter a model ID above)</option>
        {models.map(row => <option key={row.id} value={row.id} disabled={!row.selectable}>{row.name} — {row.id}{row.selectable ? " — invocation unverified" : " — unsupported by this writing connection"}</option>)}
      </select>
    </label>
    <p role="status">{catalogMessage}</p>
    <label><input type="checkbox" checked={consent} disabled={pending} onChange={event => setConsent(event.target.checked)} />Allow explicit API requests to this endpoint and model. Requests may incur charges; provider retention policies apply.</label>
    <p>Saved endpoint: {status.endpoint ?? "None"} 쨌 Model: {status.model ?? "None"} 쨌 Permission: {status.consent ? "allowed for explicit requests" : "not granted"}</p>
    <button type="button" disabled={pending || !status.epoch} onClick={() => void save()}>Keep setup for this session</button>
    <button type="button" disabled={pending} onClick={() => void clear()}>Remove API key and permission</button>
    {message && <p role="status">{message}</p>}
  </section>;
}
