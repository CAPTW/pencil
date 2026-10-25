import { invoke } from "@tauri-apps/api/core";
import { useEffect, useRef, useState } from "react";

type ApiStatus = { epoch: string; configured: boolean; endpoint: string | null; model: string | null; consent: boolean; requestEnabled: boolean; storageError?: string | null };
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
  const keyEdited = useRef(false);
  useEffect(() => {
    let mounted = true; const initialVersion = version.current;
    void invoke<ApiStatus>("api_session_status").then(value => {
      if (!mounted || version.current !== initialVersion) return;
      if (value.storageError) setMessage("Windows secure storage could not be read. API requests are disabled. Retry saving or deleting the saved key."); setStatus(value); setEndpoint(value.endpoint ?? ""); setModel(value.model ?? ""); setConsent(value.consent);
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
    keyEdited.current = false;
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
  function loadAfterKeyEntry(nextTarget: EventTarget | null) {
    // Buttons retain their explicit action; changing the endpoint must not send
    // the old draft key. Never fetch on mount, restore, or each keystroke.
    const next = nextTarget instanceof HTMLElement ? nextTarget : null;
    if (next?.closest("button") || next?.closest("label")?.textContent?.match(/API provider preset|HTTPS endpoint/)) return;
    const official = ["https://api.anthropic.com/v1/messages", "https://generativelanguage.googleapis.com/v1beta/openai/chat/completions", "https://openrouter.ai/api/v1/chat/completions", "https://api.x.ai/v1/chat/completions"].includes(endpoint);
    if (keyEdited.current && keyInput.current?.value.trim() && official) void loadModels();
  }
  async function save() {
    if (pending) return;
    const action = ++version.current; setPending(true); setMessage("");
    const key = keyInput.current?.value ?? "";
    if (keyInput.current) keyInput.current.value = "";
    try {
      const next = await invoke<ApiStatus>("configure_api_session", { setup: { expectedEpoch: status.epoch, endpoint, model, key, consent } });
      if (version.current !== action) return;
      setStatus(next); setMessage("Setup saved in Windows Credential Manager. No connection or text request was made.");
    } catch {
      const latest = await invoke<ApiStatus>("api_session_status").catch(() => empty);
      if (version.current !== action) return;
      setStatus(latest); setConsent(false);
      setMessage("Setup was not saved. Reload the current state and check the HTTPS endpoint, model and key.");
    } finally { if (version.current === action) setPending(false); }
  }
  async function revoke() {
    const action = ++version.current; setPending(true); resetCatalog();
    if (keyInput.current) keyInput.current.value = "";
    try {
      const next = await invoke<ApiStatus>("revoke_api_consent");
      if (version.current !== action) return;
      setStatus(next); setConsent(false); setMessage("API consent revoked. Active writing requests cancelled; saved key retained if secure storage succeeded.");
    } catch {
      const latest = await invoke<ApiStatus>("api_session_status").catch(() => empty);
      if (version.current !== action) return;
      setStatus(latest);
      setConsent(false); setMessage("Could not confirm durable revocation. Retry deletion before restarting or sending more requests.");
    } finally { if (version.current === action) setPending(false); }
  }
  async function clear() {
    resetCatalog();
    const action = ++version.current; setPending(true); setMessage("");
    if (keyInput.current) keyInput.current.value = "";
    try { const next = await invoke<ApiStatus>("clear_api_session"); if (version.current !== action) return; setStatus(next); setConsent(false); setMessage("Saved API key and permission deleted from Windows Credential Manager."); }
    catch { const latest = await invoke<ApiStatus>("api_session_status").catch(() => empty); if (version.current !== action) return; setStatus(latest); setConsent(false); setMessage("Could not confirm deletion from Windows Credential Manager. Retry deletion before restarting or sending more requests."); }
    finally { if (version.current === action) setPending(false); }
  }
  return <section className="provider-settings" aria-label="Optional API setup">
    <h3>Connect a model</h3>
    <p>Choose a provider, enter a key, then select a model. Saved keys stay in Windows Credential Manager.</p>
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
    <details className="connection-group"><summary>Endpoint and manual model ID</summary>
    <label className="settings-field">HTTPS endpoint
      <input type="url" value={endpoint} autoComplete="off" spellCheck={false} disabled={pending} onChange={event => { resetCatalog(); setEndpoint(event.target.value); setConsent(false); if (keyInput.current) keyInput.current.value = ""; }} placeholder="https://api.example.com/v1/chat/completions" />
    </label>
    <label className="settings-field">Model
      <input value={model} autoComplete="off" spellCheck={false} disabled={pending} onChange={event => { setModel(event.target.value); setConsent(false); }} />
    </label>
    </details>
    <label className="settings-field">API key (Windows secure storage)
      <input ref={keyInput} type="password" autoComplete="off" spellCheck={false} disabled={pending} placeholder={status.configured && endpoint === status.endpoint ? "Saved key — leave blank to keep" : "Enter API key"} maxLength={4096} onChange={() => { keyEdited.current = true; resetCatalog(); setConsent(false); }} onBlur={event => loadAfterKeyEntry(event.relatedTarget)} />
    </label>
    <p className="connection-hint">Leaving the key field loads the provider’s catalog once. No document text is sent.</p>
    <button type="button" disabled={pending} onClick={() => void loadModels()}>Load model list</button>
    <label className="settings-field">Available models
      <select disabled={pending || !models.length} value={models.some(row => row.id === model) ? model : ""} onChange={event => {setModel(event.target.value); setConsent(false);}}>
        <option value="">{model ? `Selected: ${model} · invocation unverified` : "Choose a model"}</option>
        {models.map(row => <option key={row.id} value={row.id} disabled={!row.selectable}>{row.name} — {row.id}{row.selectable ? " — invocation unverified" : " — unsupported by this writing connection"}</option>)}
      </select>
    </label>
    <p role="status">{catalogMessage}</p>
    <label><input type="checkbox" checked={consent} disabled={pending} onChange={event => setConsent(event.target.checked)} />Allow explicit API requests to this endpoint and model. Requests may incur charges; provider retention policies apply.</label>
    <p>Saved endpoint: {status.endpoint ?? "None"} 쨌 Model: {status.model ?? "None"} 쨌 Permission: {status.consent ? "allowed for explicit requests" : "not granted"}</p>
    <button className="primary-button" type="button" disabled={pending || !status.epoch} onClick={() => void save()}>Save API setup securely</button>
    <details className="connection-group"><summary>Privacy and connection management</summary>
    <p>Saving makes no network request. API requests require Run API Deep and confirmation. CLI subscriptions are separate. OpenRouter uses an explicit model with fallback disabled and zero data retention; unsupported routes fail.</p>
    <button type="button" disabled={pending || !status.configured} onClick={() => void revoke()}>Revoke API consent</button>
    <button type="button" disabled={pending} onClick={() => void clear()}>Remove API key and permission</button>
    </details>
    {message && <p role="status">{message}</p>}
  </section>;
}
