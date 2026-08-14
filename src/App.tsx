import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { openUrl } from "@tauri-apps/plugin-opener";
import {
  Check,
  ClipboardCheck,
  Languages,
  Loader2,
  LogIn,
  Keyboard,
  RefreshCcw,
  RotateCcw,
  Settings as SettingsIcon,
  ShieldCheck,
  Sparkles,
  AlertTriangle,
  X,
} from "lucide-react";
import {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
  type KeyboardEvent as ReactKeyboardEvent,
} from "react";
import {
  parseApplyOutcome,
  parseSelectionCaptured,
  sameCaptureToken,
  type CaptureToken,
} from "./captureContract";
import {
  parseAppSettings,
  parseShortcutUpdateResponse,
  sameRewriteIntent,
  shortcutCandidateFromKeyEvent,
  type AppSettings,
  type RewriteIntentToken,
  type RewriteMode,
  type ShortcutCandidate,
  type TranslationApplyFormat,
  type TranslationTargetLanguage,
} from "./promptlessContract";

type AuthStatus = {
  loggedIn: boolean;
  accountLabel: string | null;
  authMode: string | null;
  requiresOpenaiAuth: boolean;
};

type DeviceLogin = {
  loginId: string;
  verificationUrl: string;
  userCode: string;
};

type ActiveSelection = {
  token: CaptureToken;
  charCount: number;
};

type CaptureError = {
  message: string;
};

type CommandCheck = {
  name: string;
  available: boolean;
  version: string | null;
  path: string | null;
  requiredAtRuntime: boolean;
  requiredForBuild: boolean;
  error: string | null;
};

type PrerequisiteReport = {
  commands: CommandCheck[];
};

type RewriteEdit = {
  before: string;
  after: string;
  reason: string;
};

type RewriteResult = {
  replacement: string;
  changed: boolean;
  summary: string;
  edits: RewriteEdit[];
  confidence: number;
  mode: RewriteMode;
};

type LoginState = "signed_out" | "starting" | "waiting" | "signed_in" | "failed" | "cancelled";

type LoginCompleted = {
  loginId: string | null;
  success: boolean;
  error: string | null;
};

type CodexProcessExited = {
  message: string;
};

const MODES: Array<{ id: RewriteMode; label: string; icon: "sparkles" | "languages" }> = [
  { id: "grammar", label: "Grammar", icon: "sparkles" },
  { id: "natural", label: "Natural", icon: "sparkles" },
  { id: "concise", label: "Concise", icon: "sparkles" },
  { id: "polite", label: "Polite", icon: "sparkles" },
  { id: "translate", label: "Translate", icon: "languages" },
];

const TARGET_LANGUAGES: Array<{ id: TranslationTargetLanguage; label: string }> = [
  { id: "ko", label: "Korean" },
  { id: "en", label: "English" },
  { id: "ja", label: "Japanese" },
  { id: "zh-Hans", label: "Simplified Chinese" },
  { id: "zh-Hant", label: "Traditional Chinese" },
];

const DEFAULT_SETTINGS: AppSettings = {
  schemaVersion: 2,
  mode: "grammar",
  restoreClipboard: true,
  autoRewrite: true,
  shortcut: {
    primary: {
      modifiers: ["CTRL", "SHIFT"],
      key: "G",
      display: "Ctrl+Shift+G",
    },
  },
  translation: {
    sourceLanguage: "auto",
    targetLanguage: "en",
    applyFormat: "translation_only",
  },
};

function toErrorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

export default function App() {
  const [settings, setSettings] = useState<AppSettings>(DEFAULT_SETTINGS);
  const [auth, setAuth] = useState<AuthStatus | null>(null);
  const [prerequisites, setPrerequisites] = useState<PrerequisiteReport | null>(null);
  const [deviceLogin, setDeviceLogin] = useState<DeviceLogin | null>(null);
  const [selection, setSelection] = useState<ActiveSelection | null>(null);
  const [result, setResult] = useState<RewriteResult | null>(null);
  const [draft, setDraft] = useState("");
  const [status, setStatus] = useState("Ready");
  const [loginState, setLoginState] = useState<LoginState>("signed_out");
  const [error, setError] = useState<string | null>(null);
  const [isRewriting, setIsRewriting] = useState(false);
  const [isApplying, setIsApplying] = useState(false);
  const [isStartingLogin, setIsStartingLogin] = useState(false);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [recordingShortcut, setRecordingShortcut] = useState(false);
  const [shortcutCandidateLabel, setShortcutCandidateLabel] = useState<string | null>(null);
  const modeRef = useRef<RewriteMode>(DEFAULT_SETTINGS.mode);
  const targetLanguageRef = useRef<TranslationTargetLanguage>(
    DEFAULT_SETTINGS.translation.targetLanguage,
  );
  const autoRewriteRef = useRef(DEFAULT_SETTINGS.autoRewrite);
  const currentTokenRef = useRef<CaptureToken | null>(null);
  const currentIntentRef = useRef<RewriteIntentToken | null>(null);
  const rewritingIntentRef = useRef<RewriteIntentToken | null>(null);
  const resultIntentRef = useRef<RewriteIntentToken | null>(null);
  const applyingTokenRef = useRef<CaptureToken | null>(null);

  useEffect(() => {
    modeRef.current = settings.mode;
    targetLanguageRef.current = settings.translation.targetLanguage;
    autoRewriteRef.current = settings.autoRewrite;
  }, [settings]);

  const refreshAuth = useCallback(async () => {
    try {
      const next = await invoke<AuthStatus>("auth_status");
      setAuth(next);
      if (next.loggedIn) {
        setDeviceLogin(null);
        setLoginState("signed_in");
        setStatus("Signed in");
      } else {
        setLoginState((current) =>
          current === "waiting" || current === "starting" ? current : "signed_out",
        );
      }
    } catch (nextError) {
      setLoginState("failed");
      setError(toErrorMessage(nextError));
    }
  }, []);

  const saveSettings = useCallback(async (next: AppSettings): Promise<AppSettings | null> => {
    try {
      const saved = parseAppSettings(await invoke<unknown>("save_settings", { settings: next }));
      if (!saved) {
        setError("Settings returned an invalid response. Your previous settings remain active.");
        return null;
      }
      setSettings(saved);
      return saved;
    } catch (nextError) {
      setError(toErrorMessage(nextError));
      return null;
    }
  }, []);

  const rewrite = useCallback(async (
    mode: RewriteMode = modeRef.current,
    requestedToken: CaptureToken | null = currentTokenRef.current,
    targetLanguage: TranslationTargetLanguage = targetLanguageRef.current,
  ) => {
    if (!requestedToken) {
      return;
    }
    const requestedIntent: RewriteIntentToken = {
      ...requestedToken,
      mode,
      targetLanguage: mode === "translate" ? targetLanguage : null,
    };
    if (sameRewriteIntent(rewritingIntentRef.current, requestedIntent)) {
      return;
    }
    rewritingIntentRef.current = requestedIntent;
    setIsRewriting(true);
    setError(null);
    setStatus("Rewriting with Codex");
    try {
      const next = await invoke<RewriteResult>("rewrite_selected_text", {
        sessionId: requestedToken.sessionId,
        generation: requestedToken.generation,
        mode,
        targetLanguage: requestedIntent.targetLanguage,
      });
      if (!sameRewriteIntent(currentIntentRef.current, requestedIntent)) {
        return;
      }
      setResult(next);
      resultIntentRef.current = requestedIntent;
      setDraft(next.replacement);
      setStatus("Replacement ready");
    } catch (nextError) {
      if (!sameRewriteIntent(currentIntentRef.current, requestedIntent)) {
        return;
      }
      setError(toErrorMessage(nextError));
      setStatus("Rewrite failed");
    } finally {
      if (sameRewriteIntent(rewritingIntentRef.current, requestedIntent)) {
        rewritingIntentRef.current = null;
        setIsRewriting(false);
      }
    }
  }, []);

  useEffect(() => {
    let unlistenSelection: UnlistenFn | undefined;
    let unlistenError: UnlistenFn | undefined;
    let unlistenLogin: UnlistenFn | undefined;
    let unlistenAuthChanged: UnlistenFn | undefined;
    let unlistenProcessExited: UnlistenFn | undefined;
    let unlistenOpenSettings: UnlistenFn | undefined;

    void invoke<unknown>("load_settings")
      .then((value) => {
        const next = parseAppSettings(value);
        if (!next) {
          throw new Error("Settings response was invalid.");
        }
        setSettings(next);
        modeRef.current = next.mode;
        targetLanguageRef.current = next.translation.targetLanguage;
        autoRewriteRef.current = next.autoRewrite;
      })
      .catch((nextError) => setError(toErrorMessage(nextError)));

    void invoke<PrerequisiteReport>("check_prerequisites")
      .then((report) => {
        setPrerequisites(report);
        const codex = report.commands.find((command) => command.name === "codex");
        if (codex && !codex.available) {
          setError(codex.error ?? "Codex CLI must be installed and available on PATH.");
          setStatus("Codex missing");
        }
      })
      .catch((nextError) => setError(toErrorMessage(nextError)));

    void invoke<string[]>("take_startup_notices")
      .then((notices) => {
        if (notices.length > 0) {
          setError(
            notices.includes("shortcut_startup_failed")
              ? "The saved shortcut could not be registered. Open Settings to recover it."
              : notices.includes("shortcut_startup_fallback")
                ? "The saved shortcut was unavailable. Ctrl+Shift+G is active."
                : "Settings were recovered safely. Review them before continuing.",
          );
          setStatus("Setup warning");
        }
      })
      .catch((nextError) => setError(toErrorMessage(nextError)));

    void refreshAuth();

    void listen<unknown>("selection-captured", (event) => {
      const payload = parseSelectionCaptured(event.payload);
      if (!payload) {
        currentTokenRef.current = null;
        currentIntentRef.current = null;
        rewritingIntentRef.current = null;
        resultIntentRef.current = null;
        applyingTokenRef.current = null;
        setSelection(null);
        setResult(null);
        setDraft("");
        setIsRewriting(false);
        setIsApplying(false);
        setError("The capture token was invalid. Capture the selection again.");
        setStatus("Capture rejected");
        return;
      }
      const token: CaptureToken = {
        sessionId: payload.sessionId,
        generation: payload.generation,
      };
      currentTokenRef.current = token;
      const intent: RewriteIntentToken = {
        ...token,
        mode: modeRef.current,
        targetLanguage: modeRef.current === "translate" ? targetLanguageRef.current : null,
      };
      currentIntentRef.current = intent;
      rewritingIntentRef.current = null;
      resultIntentRef.current = null;
      applyingTokenRef.current = null;
      setSelection({ token, charCount: Array.from(payload.selectedText).length });
      setResult(null);
      setDraft("");
      setIsRewriting(false);
      setIsApplying(false);
      setError(null);
      setStatus("Selection captured");
      if (autoRewriteRef.current) {
        void rewrite(modeRef.current, token, targetLanguageRef.current);
      }
    }).then((unlisten) => {
      unlistenSelection = unlisten;
    });

    void listen<CaptureError>("capture-error", (event) => {
      currentTokenRef.current = null;
      currentIntentRef.current = null;
      rewritingIntentRef.current = null;
      resultIntentRef.current = null;
      applyingTokenRef.current = null;
      setSelection(null);
      setResult(null);
      setDraft("");
      setIsRewriting(false);
      setIsApplying(false);
      setError(event.payload.message);
      setStatus("No selection");
    }).then((unlisten) => {
      unlistenError = unlisten;
    });

    void listen<LoginCompleted>("login-completed", (event) => {
      if (event.payload.success) {
        setLoginState("signed_in");
        setDeviceLogin(null);
        setError(null);
        setStatus("Signed in");
        void refreshAuth();
      } else {
        setLoginState("failed");
        setError(event.payload.error ?? "ChatGPT device-code login failed.");
        setStatus("Login failed");
      }
    }).then((unlisten) => {
      unlistenLogin = unlisten;
    });

    void listen("auth-changed", () => {
      void refreshAuth();
    }).then((unlisten) => {
      unlistenAuthChanged = unlisten;
    });

    void listen<CodexProcessExited>("codex-process-exited", (event) => {
      setError(event.payload.message);
      setStatus("Codex exited");
    }).then((unlisten) => {
      unlistenProcessExited = unlisten;
    });

    void listen("open-settings", () => {
      setSettingsOpen(true);
      setRecordingShortcut(false);
    }).then((unlisten) => {
      unlistenOpenSettings = unlisten;
    });

    return () => {
      unlistenSelection?.();
      unlistenError?.();
      unlistenLogin?.();
      unlistenAuthChanged?.();
      unlistenProcessExited?.();
      unlistenOpenSettings?.();
    };
  }, [refreshAuth, rewrite]);

  useEffect(() => {
    if (!deviceLogin) {
      return;
    }

    const timer = window.setInterval(() => {
      void refreshAuth();
    }, 3000);

    return () => window.clearInterval(timer);
  }, [deviceLogin, refreshAuth]);

  const selectedMode = useMemo(
    () => MODES.find((mode) => mode.id === settings.mode) ?? MODES[0],
    [settings.mode],
  );

  const codexCheck = useMemo(
    () => prerequisites?.commands.find((command) => command.name === "codex") ?? null,
    [prerequisites],
  );

  async function startDeviceLogin() {
    setIsStartingLogin(true);
    setError(null);
    setLoginState("starting");
    setStatus("Starting login");
    try {
      const next = await invoke<DeviceLogin>("start_device_login");
      setDeviceLogin(next);
      await openUrl(next.verificationUrl);
      setLoginState("waiting");
      setStatus("Waiting for ChatGPT login");
    } catch (nextError) {
      setLoginState("failed");
      setError(toErrorMessage(nextError));
      setStatus("Login failed");
    } finally {
      setIsStartingLogin(false);
    }
  }

  async function cancelDeviceLogin() {
    if (!deviceLogin) {
      return;
    }

    setError(null);
    try {
      await invoke("cancel_device_login", { loginId: deviceLogin.loginId });
      setDeviceLogin(null);
      setLoginState("cancelled");
      setStatus("Login cancelled");
    } catch (nextError) {
      setLoginState("failed");
      setError(toErrorMessage(nextError));
    }
  }

  async function chooseMode(mode: RewriteMode) {
    if (mode === settings.mode) {
      return;
    }
    const next = { ...settings, mode };
    const saved = await saveSettings(next);
    if (!saved) {
      return;
    }
    modeRef.current = saved.mode;
    targetLanguageRef.current = saved.translation.targetLanguage;
    if (selection) {
      const intent: RewriteIntentToken = {
        ...selection.token,
        mode: saved.mode,
        targetLanguage:
          saved.mode === "translate" ? saved.translation.targetLanguage : null,
      };
      currentIntentRef.current = intent;
      resultIntentRef.current = null;
      setResult(null);
      setDraft("");
      setStatus("Selection captured");
      if (saved.autoRewrite) {
        void rewrite(saved.mode, selection.token, saved.translation.targetLanguage);
      }
    }
  }

  async function chooseTargetLanguage(targetLanguage: TranslationTargetLanguage) {
    const next = {
      ...settings,
      translation: { ...settings.translation, targetLanguage },
    };
    const saved = await saveSettings(next);
    if (!saved) {
      return;
    }
    targetLanguageRef.current = saved.translation.targetLanguage;
    if (selection && saved.mode === "translate") {
      const intent: RewriteIntentToken = {
        ...selection.token,
        mode: "translate",
        targetLanguage: saved.translation.targetLanguage,
      };
      currentIntentRef.current = intent;
      resultIntentRef.current = null;
      setResult(null);
      setDraft("");
      setStatus("Translation target changed");
      if (saved.autoRewrite) {
        void rewrite("translate", selection.token, saved.translation.targetLanguage);
      }
    }
  }

  async function chooseApplyFormat(applyFormat: TranslationApplyFormat) {
    await saveSettings({
      ...settings,
      translation: { ...settings.translation, applyFormat },
    });
  }

  async function toggleRestoreClipboard() {
    await saveSettings({ ...settings, restoreClipboard: !settings.restoreClipboard });
  }

  async function toggleAutoRewrite() {
    await saveSettings({ ...settings, autoRewrite: !settings.autoRewrite });
  }

  async function submitShortcutCandidate(candidate: ShortcutCandidate) {
    try {
      const response = parseShortcutUpdateResponse(
        await invoke<unknown>("update_primary_shortcut", { candidate }),
      );
      if (!response) {
        setError("Shortcut update returned an invalid response.");
        return;
      }
      setSettings((current) => ({
        ...current,
        shortcut: { primary: response.active },
      }));
      if (response.status === "applied" || response.status === "unchanged") {
        setError(null);
        setStatus(response.status === "applied" ? "Shortcut updated" : "Shortcut unchanged");
      } else if (response.status === "conflict") {
        setError("That shortcut is unavailable. The previous shortcut remains active.");
        setStatus("Shortcut conflict");
      } else if (response.status === "persistence_failed_rolled_back") {
        setError("The shortcut could not be saved. The previous shortcut was restored.");
        setStatus("Shortcut rolled back");
      } else {
        setError("That shortcut is invalid or could not be registered. The previous shortcut remains active.");
        setStatus("Shortcut rejected");
      }
    } catch (nextError) {
      setError(toErrorMessage(nextError));
      setStatus("Shortcut update failed");
    } finally {
      setRecordingShortcut(false);
    }
  }

  function recordShortcut(event: ReactKeyboardEvent<HTMLButtonElement>) {
    if (!recordingShortcut) {
      return;
    }
    event.preventDefault();
    event.stopPropagation();
    const candidate = shortcutCandidateFromKeyEvent(event);
    if (!candidate) {
      return;
    }
    const label = [...candidate.modifiers, candidate.key].join("+");
    setShortcutCandidateLabel(label);
    void submitShortcutCandidate(candidate);
  }

  async function resetShortcut() {
    try {
      const response = parseShortcutUpdateResponse(
        await invoke<unknown>("reset_primary_shortcut"),
      );
      if (!response) {
        setError("Shortcut reset returned an invalid response.");
        return;
      }
      setSettings((current) => ({
        ...current,
        shortcut: { primary: response.active },
      }));
      setShortcutCandidateLabel(null);
      setError(
        response.status === "applied" || response.status === "unchanged"
          ? null
          : "The default shortcut could not be restored. The previous shortcut remains active.",
      );
      setStatus(
        response.status === "applied" || response.status === "unchanged"
          ? "Default shortcut active"
          : "Shortcut reset failed",
      );
    } catch (nextError) {
      setError(toErrorMessage(nextError));
    }
  }

  async function applyReplacement() {
    const token = currentTokenRef.current;
    const intent = currentIntentRef.current;
    if (!token || !intent || !sameRewriteIntent(resultIntentRef.current, intent) || draft.length === 0) {
      return;
    }

    applyingTokenRef.current = token;
    setIsApplying(true);
    setError(null);
    try {
      const rawOutcome = await invoke<unknown>("apply_replacement", {
        sessionId: token.sessionId,
        generation: token.generation,
        replacement: draft,
        mode: intent.mode,
        targetLanguage: intent.targetLanguage,
        restoreClipboard: settings.restoreClipboard,
      });
      if (!sameCaptureToken(currentTokenRef.current, token)) {
        return;
      }
      const outcome = parseApplyOutcome(rawOutcome);
      if (!outcome) {
        currentTokenRef.current = null;
        currentIntentRef.current = null;
        resultIntentRef.current = null;
        setSelection(null);
        setError("Apply returned an invalid response. Capture the selection again.");
        setStatus("Apply rejected");
        return;
      }
      if (outcome.status === "applied") {
        currentTokenRef.current = null;
        currentIntentRef.current = null;
        resultIntentRef.current = null;
        setSelection(null);
        setStatus("Applied");
        return;
      }
      if (outcome.status === "copied_fallback") {
        currentTokenRef.current = null;
        currentIntentRef.current = null;
        resultIntentRef.current = null;
        setSelection(null);
        setStatus("Copied — paste manually");
        setError(
          "The captured target could not be proven safe. The approved replacement is on the clipboard; paste it manually.",
        );
        return;
      }
      if (outcome.status === "rejected_stale") {
        currentTokenRef.current = null;
        currentIntentRef.current = null;
        resultIntentRef.current = null;
        setSelection(null);
        setResult(null);
        setDraft("");
        setStatus("Stale result rejected");
        setError("This result no longer belongs to the current capture. Capture the selection again.");
        return;
      }

      setStatus("Apply failed safely");
      if (
        outcome.reason === "input_injection_failed" ||
        outcome.reason === "invalid_session_state"
      ) {
        currentTokenRef.current = null;
        setSelection(null);
      }
      setError(
        outcome.reason === "clipboard_ownership_lost"
          ? "The clipboard changed before paste. Nothing was pasted; review and try again."
          : outcome.reason === "input_injection_failed"
            ? "Windows did not confirm the complete paste input. Automatic retry is disabled; capture again."
            : "Nothing was pasted. Review the target and try again.",
      );
    } catch (nextError) {
      if (!sameCaptureToken(currentTokenRef.current, token)) {
        return;
      }
      currentTokenRef.current = null;
      setSelection(null);
      setError(toErrorMessage(nextError));
      setStatus("Apply failed safely");
    } finally {
      if (sameCaptureToken(applyingTokenRef.current, token)) {
        applyingTokenRef.current = null;
        setIsApplying(false);
      }
    }
  }

  async function dismiss() {
    const token = currentTokenRef.current;
    currentTokenRef.current = null;
    currentIntentRef.current = null;
    rewritingIntentRef.current = null;
    resultIntentRef.current = null;
    applyingTokenRef.current = null;
    setSelection(null);
    setResult(null);
    setDraft("");
    setIsRewriting(false);
    setIsApplying(false);
    try {
      await invoke("dismiss_window", {
        sessionId: token?.sessionId ?? null,
        generation: token?.generation ?? null,
      });
    } catch (nextError) {
      if (!currentTokenRef.current) {
        setError(toErrorMessage(nextError));
        setStatus("Dismiss failed");
      }
    }
  }

  const canRewrite = Boolean(selection && auth?.loggedIn && !isRewriting && !result);
  const canApply = Boolean(
    selection &&
      result &&
      sameRewriteIntent(resultIntentRef.current, currentIntentRef.current) &&
      draft.length > 0 &&
      !isApplying &&
      !isRewriting,
  );
  const loginStatusLabel =
    loginState === "starting"
      ? "Starting login"
      : loginState === "waiting"
        ? "Waiting for device code"
        : loginState === "signed_in"
          ? "Signed in"
          : loginState === "failed"
            ? "Login failed"
            : loginState === "cancelled"
              ? "Login cancelled"
              : "Signed out";

  return (
    <main className="shell">
      <section className="panel">
        <header className="topbar">
          <div className="brand">
            <div className="brand-mark">
              <Sparkles size={16} strokeWidth={2.2} />
            </div>
            <div>
              <h1>Codex Pencil</h1>
              <p>{auth?.loggedIn ? auth.accountLabel : "ChatGPT Codex login required"}</p>
            </div>
          </div>
          <div className="topbar-actions">
            <button
              className="icon-button"
              type="button"
              onClick={() => setSettingsOpen((current) => !current)}
              aria-label="Settings"
            >
              <SettingsIcon size={16} />
            </button>
            <button className="icon-button" type="button" onClick={dismiss} aria-label="Close">
              <X size={16} />
            </button>
          </div>
        </header>

        {settingsOpen ? (
          <div className="settings-pane" aria-label="Settings">
            <div className="settings-heading">
              <div>
                <h2>Settings</h2>
                <p>One primary shortcut opens Codex Pencil.</p>
              </div>
              <Keyboard size={20} />
            </div>
            <label className="settings-field">
              <span>Primary shortcut</span>
              <button
                className={recordingShortcut ? "shortcut-recorder recording" : "shortcut-recorder"}
                type="button"
                onClick={() => {
                  setShortcutCandidateLabel(null);
                  setRecordingShortcut(true);
                }}
                onBlur={() => setRecordingShortcut(false)}
                onKeyDown={recordShortcut}
              >
                {recordingShortcut
                  ? shortcutCandidateLabel ?? "Press a shortcut…"
                  : settings.shortcut.primary.display}
              </button>
            </label>
            <div className="settings-actions">
              <button className="secondary-button" type="button" onClick={resetShortcut}>
                <RotateCcw size={15} />
                Reset Ctrl+Shift+G
              </button>
              <button className="primary-button" type="button" onClick={() => setSettingsOpen(false)}>
                Done
              </button>
            </div>
          </div>
        ) : !auth?.loggedIn ? (
          <div className="login-pane">
            <div className="login-copy">
              {codexCheck && !codexCheck.available ? <AlertTriangle size={22} /> : <LogIn size={22} />}
              <div>
                <h2>Sign in with ChatGPT</h2>
                <p>{loginStatusLabel}</p>
              </div>
            </div>

            {codexCheck && !codexCheck.available ? (
              <div className="missing-codex">
                Codex CLI must be installed and available on PATH.
              </div>
            ) : null}

            {deviceLogin ? (
              <div className="device-code">
                <span>Device code</span>
                <strong>{deviceLogin.userCode}</strong>
                <div className="device-actions">
                  <button type="button" onClick={() => openUrl(deviceLogin.verificationUrl)}>
                    Open login page
                  </button>
                  <button type="button" onClick={cancelDeviceLogin}>
                    Cancel
                  </button>
                </div>
              </div>
            ) : null}

            <button
              className="primary-button"
              type="button"
              onClick={startDeviceLogin}
              disabled={isStartingLogin || Boolean(codexCheck && !codexCheck.available)}
            >
              {isStartingLogin ? <Loader2 className="spin" size={16} /> : <LogIn size={16} />}
              Start device login
            </button>
          </div>
        ) : (
          <>
            <div className="status-row">
              <div className="status-copy">
                <ClipboardCheck size={16} />
                <span>
                  {selection
                    ? `${selection.charCount.toLocaleString()} characters selected`
                    : `Press ${settings.shortcut.primary.display}`}
                </span>
              </div>
              <span className="status-pill">{status}</span>
            </div>

            {settings.mode === "translate" ? (
              <div className="translation-controls">
                <label className="translation-field">
                  <span>Target language</span>
                  <select
                    value={settings.translation.targetLanguage}
                    onChange={(event) =>
                      chooseTargetLanguage(event.target.value as TranslationTargetLanguage)
                    }
                  >
                    {TARGET_LANGUAGES.map((language) => (
                      <option value={language.id} key={language.id}>
                        {language.label}
                      </option>
                    ))}
                  </select>
                </label>
                <fieldset className="format-options">
                  <legend>Apply format</legend>
                  <label>
                    <input
                      type="radio"
                      name="translation-format"
                      checked={settings.translation.applyFormat === "translation_only"}
                      onChange={() => chooseApplyFormat("translation_only")}
                    />
                    번역문만
                  </label>
                  <label>
                    <input
                      type="radio"
                      name="translation-format"
                      checked={settings.translation.applyFormat === "source_with_translation"}
                      onChange={() => chooseApplyFormat("source_with_translation")}
                    />
                    원문 (번역문)
                  </label>
                </fieldset>
                {settings.translation.applyFormat === "source_with_translation" ? (
                  <p>The exact captured source is combined locally when you Apply.</p>
                ) : null}
              </div>
            ) : null}

            <div className="mode-grid" aria-label="Rewrite mode">
              {MODES.map((mode) => {
                const isActive = mode.id === selectedMode.id;
                const Icon = mode.icon === "languages" ? Languages : Sparkles;
                return (
                  <button
                    className={isActive ? "mode-button active" : "mode-button"}
                    type="button"
                    key={mode.id}
                    onClick={() => chooseMode(mode.id)}
                  >
                    <Icon size={14} />
                    {mode.label}
                  </button>
                );
              })}
            </div>

            <div className="toggles">
              <label>
                <input
                  type="checkbox"
                  checked={settings.autoRewrite}
                  onChange={toggleAutoRewrite}
                />
                Auto rewrite
              </label>
              <label>
                <input
                  type="checkbox"
                  checked={settings.restoreClipboard}
                  onChange={toggleRestoreClipboard}
                />
                Restore clipboard
              </label>
            </div>

            <div className="result-area">
              {isRewriting ? (
                <div className="loading-state">
                  <Loader2 className="spin" size={18} />
                  <span>Working locally through Codex</span>
                </div>
              ) : (
                <textarea
                  value={draft}
                  onChange={(event) => setDraft(event.target.value)}
                  placeholder="Replacement will appear here"
                  spellCheck={false}
                />
              )}
            </div>

            {result ? (
              <div className="rewrite-details">
                <div className="rewrite-summary">
                  <span>{result.summary}</span>
                  <strong>{Math.round(result.confidence * 100)}%</strong>
                </div>
                {result.edits.length ? (
                  <ul className="edit-list">
                    {result.edits.map((item, index) => (
                      <li key={`${item.before}-${item.after}-${index}`}>
                        <span>{item.reason}</span>
                      </li>
                    ))}
                  </ul>
                ) : null}
              </div>
            ) : null}

            <footer className="actions">
              <button className="secondary-button" type="button" onClick={() => rewrite()} disabled={!canRewrite}>
                <RefreshCcw size={16} />
                Rewrite
              </button>
              <button className="primary-button" type="button" onClick={applyReplacement} disabled={!canApply}>
                {isApplying ? <Loader2 className="spin" size={16} /> : <Check size={16} />}
                Apply
              </button>
            </footer>
          </>
        )}

        {error ? (
          <div className="error-row">
            <ShieldCheck size={14} />
            <span>{error}</span>
          </div>
        ) : null}
      </section>
    </main>
  );
}
