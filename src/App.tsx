import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import {
  Check,
  BookOpen,
  ClipboardCheck,
  Download,
  Edit3,
  Languages,
  Loader2,
  LogIn,
  Keyboard,
  Plus,
  RefreshCcw,
  RotateCcw,
  Search,
  Settings as SettingsIcon,
  ShieldCheck,
  Sparkles,
  AlertTriangle,
  Trash2,
  Upload,
  X,
} from "lucide-react";
import {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
  type KeyboardEvent as ReactKeyboardEvent,
  type MouseEvent as ReactMouseEvent,
} from "react";
import {
  parseApplyOutcome,
  parseSelectionCaptured,
  sameCaptureToken,
  type CaptureToken,
} from "./captureContract";
import {
  cancelSwitch,
  captureReset,
  confirmSwitch,
  editDraft,
  EMPTY_INSTANT_RUNTIME_STATE,
  lateCandidate,
  requestSwitch,
  visibleChoices,
  type InstantRuntimeState,
  type RuntimeCandidate,
} from "./instantSelectionRuntime";
import {
  parseAppSettings,
  parseShortcutUpdateResponse,
  sameRewriteIntent,
  shortcutCandidateFromKeyEvent,
  type AppSettings,
  type ProviderKind,
  type RewriteIntentToken,
  type RewriteMode,
  type ShortcutCandidate,
  type TranslationApplyFormat,
  type TranslationReferenceLanguage,
  type TranslationTargetLanguage,
} from "./promptlessContract";
import {
  parseProviderSnapshot,
  providerDisplayName,
  type ProviderSnapshot,
  type ProviderStatus,
} from "./providerContract";
import {
  CLOUD_PROCESSING_DISCLOSURE_VERSION,
  contentLimitMessage,
  isBackendContentLimitError,
  userFacingRuntimeErrorMessage,
  validateFrontendTextLimit,
} from "./mvpContract";
import {
  entryAffectsRequests,
  parseImportPlanPreview,
  parseImportReport,
  parseTerminologyRewriteResult,
  parseTerminologyRuntimeSnapshot,
  type ImportPlanPreview,
  type TerminologyEntry,
  type TerminologyEntryDraft,
  type TerminologyEntryStatus,
  type TerminologyEntryType,
  type TerminologyEntrySort,
  type TerminologyLanguage,
  type TerminologyRewriteResult,
  type TerminologyRuntimeSnapshot,
  type TerminologySuggestion,
} from "./terminologyContract";
import { startWindowDrag } from "./windowChromeContract";

type AuthStatus = {
  loggedIn: boolean;
  accountLabel: string | null;
  authMode: string | null;
  requiresOpenaiAuth: boolean;
};

type DeviceLogin = {
  loginId: string;
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

type RewriteResult = TerminologyRewriteResult;

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
  { id: "auto", label: "Auto" },
  { id: "ko", label: "Korean" },
  { id: "en", label: "English" },
  { id: "ja", label: "Japanese" },
  { id: "zh-Hans", label: "Simplified Chinese" },
  { id: "zh-Hant", label: "Traditional Chinese" },
];

const REFERENCE_LANGUAGES: Array<{ id: TranslationReferenceLanguage; label: string }> =
  TARGET_LANGUAGES.filter(
    (language): language is { id: TranslationReferenceLanguage; label: string } =>
      language.id !== "auto",
  );

function autoReferenceForIntent(
  mode: RewriteMode,
  targetLanguage: TranslationTargetLanguage,
  referenceLanguage: TranslationReferenceLanguage,
): TranslationReferenceLanguage | null {
  return mode === "translate" && targetLanguage === "auto" ? referenceLanguage : null;
}

function referenceLanguageLabel(language: TranslationReferenceLanguage): string {
  return REFERENCE_LANGUAGES.find((candidate) => candidate.id === language)?.label ?? language;
}

function automaticFallbackLanguage(
  referenceLanguage: TranslationReferenceLanguage,
): TranslationReferenceLanguage {
  return referenceLanguage === "en" ? "ko" : "en";
}

function cloudAckFor(settings: AppSettings, kind: ProviderKind): number {
  if (kind === "codex") return settings.cloudProcessingAcknowledgementVersion;
  if (kind === "antigravity") return settings.antigravityCloudAcknowledgementVersion;
  return settings.claudeCloudAcknowledgementVersion;
}

const DEFAULT_SETTINGS: AppSettings = {
  schemaVersion: 6,
  cloudProcessingAcknowledgementVersion: 0,
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
    autoReferenceLanguage: "ko",
    applyFormat: "translation_only",
  },
  terminology: {
    enabled: true,
    activeProfileId: "general",
    useApprovedTerminology: true,
    suggestTerminology: true,
    autoSaveSuggestions: false,
  },
  activeProvider: "codex",
  antigravityCloudAcknowledgementVersion: 0,
  claudeCloudAcknowledgementVersion: 0,
};

const EMPTY_ENTRY_DRAFT: TerminologyEntryDraft = {
  profileId: "general",
  type: "preferred",
  status: "approved",
  sourceText: "",
  preferredText: "",
  sourceLanguage: "any",
  targetLanguage: "any",
  aliases: [],
  matchMode: "whole_phrase",
  caseSensitive: false,
  priority: 100,
  usageCount: 0,
  occurrenceCount: 0,
  note: null,
};

function toErrorMessage(error: unknown): string {
  return userFacingRuntimeErrorMessage(error);
}

function requireTerminologySnapshot(value: unknown): TerminologyRuntimeSnapshot {
  const snapshot = parseTerminologyRuntimeSnapshot(value);
  if (!snapshot) {
    throw new Error("Terminology response was invalid. No local change was accepted.");
  }
  return snapshot;
}

function draftFromEntry(entry: TerminologyEntry): TerminologyEntryDraft {
  return {
    profileId: entry.profileId,
    type: entry.type,
    status: entry.status,
    sourceText: entry.sourceText,
    preferredText: entry.preferredText,
    sourceLanguage: entry.sourceLanguage,
    targetLanguage: entry.targetLanguage,
    aliases: [...entry.aliases],
    matchMode: "whole_phrase",
    caseSensitive: entry.caseSensitive,
    priority: entry.priority,
    usageCount: entry.usageCount,
    occurrenceCount: entry.occurrenceCount,
    note: entry.note,
  };
}

export default function App() {
  const [settings, setSettings] = useState<AppSettings>(DEFAULT_SETTINGS);
  const [terminology, setTerminology] = useState<TerminologyRuntimeSnapshot | null>(null);
  const [terminologyQuery, setTerminologyQuery] = useState("");
  const [terminologyTypeFilter, setTerminologyTypeFilter] = useState<TerminologyEntryType | "all">("all");
  const [terminologyStatusFilter, setTerminologyStatusFilter] = useState<TerminologyEntryStatus | "all">("all");
  const [terminologyProfileFilter, setTerminologyProfileFilter] = useState("all");
  const [terminologySourceFilter, setTerminologySourceFilter] = useState<TerminologyLanguage | "all">("all");
  const [terminologyTargetFilter, setTerminologyTargetFilter] = useState<TerminologyLanguage | "all">("all");
  const [terminologySort, setTerminologySort] = useState<TerminologyEntrySort>("source_text");
  const [profileNameDraft, setProfileNameDraft] = useState("");
  const [entryDraft, setEntryDraft] = useState<TerminologyEntryDraft>(EMPTY_ENTRY_DRAFT);
  const [editingEntryId, setEditingEntryId] = useState<string | null>(null);
  const [importText, setImportText] = useState("");
  const [importFormat, setImportFormat] = useState<"json" | "csv">("json");
  const [importPreview, setImportPreview] = useState<ImportPlanPreview | null>(null);
  const [auth, setAuth] = useState<AuthStatus | null>(null);
  const [providerSnapshot, setProviderSnapshot] = useState<ProviderSnapshot | null>(null);
  const [prerequisites, setPrerequisites] = useState<PrerequisiteReport | null>(null);
  const [deviceLogin, setDeviceLogin] = useState<DeviceLogin | null>(null);
  const [selection, setSelection] = useState<ActiveSelection | null>(null);
  const [result, setResult] = useState<RewriteResult | null>(null);
  const [draft, setDraft] = useState("");
  const [instantRuntime, setInstantRuntime] = useState<InstantRuntimeState>(EMPTY_INSTANT_RUNTIME_STATE);
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
  const providerRef = useRef<ProviderKind>(DEFAULT_SETTINGS.activeProvider);
  const providerAckRef = useRef(0);
  const targetLanguageRef = useRef<TranslationTargetLanguage>(
    DEFAULT_SETTINGS.translation.targetLanguage,
  );
  const autoReferenceLanguageRef = useRef<TranslationReferenceLanguage>(
    DEFAULT_SETTINGS.translation.autoReferenceLanguage,
  );
  const autoRewriteRef = useRef(DEFAULT_SETTINGS.autoRewrite);
  const cloudAcknowledgementRef = useRef(
    DEFAULT_SETTINGS.cloudProcessingAcknowledgementVersion,
  );
  const currentTokenRef = useRef<CaptureToken | null>(null);
  const currentIntentRef = useRef<RewriteIntentToken | null>(null);
  const rewritingIntentRef = useRef<RewriteIntentToken | null>(null);
  const resultIntentRef = useRef<RewriteIntentToken | null>(null);
  const applyingTokenRef = useRef<CaptureToken | null>(null);
  const terminologyEpochRef = useRef(0);

  useEffect(() => {
    modeRef.current = settings.mode;
    providerRef.current = settings.activeProvider;
    providerAckRef.current = cloudAckFor(settings, settings.activeProvider);
    targetLanguageRef.current = settings.translation.targetLanguage;
    autoReferenceLanguageRef.current = settings.translation.autoReferenceLanguage;
    autoRewriteRef.current = settings.autoRewrite;
    cloudAcknowledgementRef.current = settings.cloudProcessingAcknowledgementVersion;
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

  const refreshProviders = useCallback(async () => {
    try {
      const next = parseProviderSnapshot(await invoke<unknown>("refresh_providers"));
      if (next) {
        setProviderSnapshot(next);
      }
    } catch (nextError) {
      setError(toErrorMessage(nextError));
    }
  }, []);

  const refreshTerminology = useCallback(async () => {
    try {
      setTerminology(requireTerminologySnapshot(await invoke<unknown>("terminology_state")));
    } catch (nextError) {
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
    autoReferenceLanguage: TranslationReferenceLanguage = autoReferenceLanguageRef.current,
  ) => {
    if (!requestedToken) {
      return;
    }
    if (cloudAcknowledgementRef.current < CLOUD_PROCESSING_DISCLOSURE_VERSION) {
      setStatus("Cloud processing review required");
      return;
    }
    const requestedIntent: RewriteIntentToken = {
      ...requestedToken,
      mode,
      targetLanguage: mode === "translate" ? targetLanguage : null,
      autoReferenceLanguage: autoReferenceForIntent(
        mode,
        targetLanguage,
        autoReferenceLanguage,
      ),
    };
    const requestedTerminologyEpoch = terminologyEpochRef.current;
    if (sameRewriteIntent(rewritingIntentRef.current, requestedIntent)) {
      return;
    }
    rewritingIntentRef.current = requestedIntent;
    setIsRewriting(true);
    setError(null);
    setStatus(`Rewriting with ${providerDisplayName(providerRef.current)}`);
    try {
      const next = parseTerminologyRewriteResult(await invoke<unknown>("rewrite_selected_text", {
        sessionId: requestedToken.sessionId,
        generation: requestedToken.generation,
        mode,
        targetLanguage: requestedIntent.targetLanguage,
        autoReferenceLanguage: requestedIntent.autoReferenceLanguage,
      }));
      if (!next) {
        throw new Error("Rewrite returned an invalid terminology contract.");
      }
      const replacementLimit = validateFrontendTextLimit("model_replacement", next.replacement);
      if (replacementLimit) {
        throw new Error(contentLimitMessage(replacementLimit));
      }
      if (
        terminologyEpochRef.current !== requestedTerminologyEpoch ||
        !sameRewriteIntent(currentIntentRef.current, requestedIntent)
      ) {
        return;
      }
      setResult(next);
      resultIntentRef.current = requestedIntent;
      const deepCandidate: RuntimeCandidate = {
        kind: "deep",
        text: next.replacement,
        sessionId: requestedToken.sessionId,
        generation: requestedToken.generation,
      };
      setInstantRuntime((current) => {
        const nextState = lateCandidate(current, deepCandidate);
        if (!current.dirty && current.draft.length === 0) {
          setDraft(nextState.draft);
        }
        return nextState;
      });
      setStatus("Replacement ready");
    } catch (nextError) {
      if (
        terminologyEpochRef.current !== requestedTerminologyEpoch ||
        !sameRewriteIntent(currentIntentRef.current, requestedIntent)
      ) {
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
    let unlistenInstant: UnlistenFn | undefined;

    void invoke<unknown>("load_settings")
      .then((value) => {
        const next = parseAppSettings(value);
        if (!next) {
          throw new Error("Settings response was invalid.");
        }
        setSettings(next);
        modeRef.current = next.mode;
        targetLanguageRef.current = next.translation.targetLanguage;
        autoReferenceLanguageRef.current = next.translation.autoReferenceLanguage;
        autoRewriteRef.current = next.autoRewrite;
        cloudAcknowledgementRef.current = next.cloudProcessingAcknowledgementVersion;
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
            notices.includes("terminology_store_unrecoverable")
              ? "The local terminology store and backup are invalid. Open Settings to reset it explicitly."
              : notices.includes("terminology_backup_recovered")
                ? "The local terminology store was recovered from its valid backup."
                : notices.includes("shortcut_startup_failed")
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
    void refreshProviders();
    void refreshTerminology();

    void listen<unknown>("instant-candidate", (event) => {
      const payload = event.payload as {
        sessionId?: string;
        generation?: number;
        noChange?: boolean;
        candidateText?: string | null;
      };
      if (!payload.sessionId || typeof payload.generation !== "number" || payload.noChange || !payload.candidateText) {
        return;
      }
      const candidate: RuntimeCandidate = {
        kind: "instant",
        text: payload.candidateText,
        sessionId: payload.sessionId,
        generation: payload.generation,
      };
      setInstantRuntime((current) => {
        const nextState = lateCandidate(current, candidate);
        if (!current.dirty && current.draft.length === 0) {
          setDraft(nextState.draft);
          setStatus("Instant candidate ready");
        }
        return nextState;
      });
    }).then((unlisten) => {
      unlistenInstant = unlisten;
    });

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
        autoReferenceLanguage: autoReferenceForIntent(
          modeRef.current,
          targetLanguageRef.current,
          autoReferenceLanguageRef.current,
        ),
      };
      currentIntentRef.current = intent;
      rewritingIntentRef.current = null;
      resultIntentRef.current = null;
      applyingTokenRef.current = null;
      setSelection({ token, charCount: Array.from(payload.selectedText).length });
      setInstantRuntime(captureReset(EMPTY_INSTANT_RUNTIME_STATE, token.sessionId, token.generation));
      setResult(null);
      setDraft("");
      setIsRewriting(false);
      setIsApplying(false);
      setError(null);
      setStatus("Selection captured");
      const sourceLimit = validateFrontendTextLimit("source", payload.selectedText);
      if (sourceLimit) {
        setError(contentLimitMessage(sourceLimit));
        setStatus("Selection too large");
      } else if (
        autoRewriteRef.current &&
        providerAckRef.current >= CLOUD_PROCESSING_DISCLOSURE_VERSION
      ) {
        void rewrite(modeRef.current, token, targetLanguageRef.current);
      } else if (cloudAcknowledgementRef.current < CLOUD_PROCESSING_DISCLOSURE_VERSION) {
        setStatus("Cloud processing review required");
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
      void refreshTerminology();
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
      unlistenInstant?.();
    };
  }, [refreshAuth, refreshProviders, refreshTerminology, rewrite]);

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

  const terminologyStore = terminology?.status === "ready" ? terminology.store : null;
  const filteredTerminologyEntries = useMemo(() => {
    if (!terminologyStore) return [];
    const query = terminologyQuery.normalize("NFC").trim().toLocaleLowerCase();
    return terminologyStore.entries
      .filter((entry) =>
        (terminologyProfileFilter === "all" || entry.profileId === terminologyProfileFilter) &&
        (terminologyTypeFilter === "all" || entry.type === terminologyTypeFilter) &&
        (terminologyStatusFilter === "all" || entry.status === terminologyStatusFilter) &&
        (terminologySourceFilter === "all" || entry.sourceLanguage === terminologySourceFilter) &&
        (terminologyTargetFilter === "all" || entry.targetLanguage === terminologyTargetFilter) &&
        (!query || [entry.sourceText, entry.preferredText ?? "", ...entry.aliases, entry.note ?? ""]
          .some((value) => value.normalize("NFC").toLocaleLowerCase().includes(query))))
      .sort((left, right) => {
        const stable = left.id.localeCompare(right.id);
        if (terminologySort === "priority") return right.priority - left.priority || stable;
        if (terminologySort === "recently_updated") return right.updatedAtMs - left.updatedAtMs || stable;
        if (terminologySort === "usage_count") return right.usageCount - left.usageCount || stable;
        return left.sourceText.localeCompare(right.sourceText) || stable;
      });
  }, [
    terminologyProfileFilter,
    terminologyQuery,
    terminologySort,
    terminologySourceFilter,
    terminologyStatusFilter,
    terminologyStore,
    terminologyTargetFilter,
    terminologyTypeFilter,
  ]);

  async function startDeviceLogin() {
    setIsStartingLogin(true);
    setError(null);
    setLoginState("starting");
    setStatus("Starting login");
    try {
      const next = await invoke<DeviceLogin>("start_device_login");
      setDeviceLogin(next);
      await invoke("open_device_login_page", { loginId: next.loginId });
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

  async function openDeviceLoginPage() {
    if (!deviceLogin) return;
    try {
      await invoke("open_device_login_page", { loginId: deviceLogin.loginId });
    } catch (nextError) {
      setError(toErrorMessage(nextError));
      setStatus("Login page unavailable");
    }
  }

  async function acknowledgeCloudProcessing() {
    const kind = settings.activeProvider;
    const saved = await saveSettings({
      ...settings,
      cloudProcessingAcknowledgementVersion:
        kind === "codex" ? CLOUD_PROCESSING_DISCLOSURE_VERSION : settings.cloudProcessingAcknowledgementVersion,
      antigravityCloudAcknowledgementVersion:
        kind === "antigravity" ? CLOUD_PROCESSING_DISCLOSURE_VERSION : settings.antigravityCloudAcknowledgementVersion,
      claudeCloudAcknowledgementVersion:
        kind === "claude" ? CLOUD_PROCESSING_DISCLOSURE_VERSION : settings.claudeCloudAcknowledgementVersion,
    });
    if (!saved) return;
    cloudAcknowledgementRef.current = saved.cloudProcessingAcknowledgementVersion;
    providerAckRef.current = cloudAckFor(saved, saved.activeProvider);
    const token = currentTokenRef.current;
    if (token && saved.autoRewrite) {
      await rewrite(saved.mode, token, saved.translation.targetLanguage);
    } else {
      setStatus("Selection captured");
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
    autoReferenceLanguageRef.current = saved.translation.autoReferenceLanguage;
    if (selection) {
      const intent: RewriteIntentToken = {
        ...selection.token,
        mode: saved.mode,
        targetLanguage:
          saved.mode === "translate" ? saved.translation.targetLanguage : null,
        autoReferenceLanguage: autoReferenceForIntent(
          saved.mode,
          saved.translation.targetLanguage,
          saved.translation.autoReferenceLanguage,
        ),
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
    autoReferenceLanguageRef.current = saved.translation.autoReferenceLanguage;
    if (selection && saved.mode === "translate") {
      const intent: RewriteIntentToken = {
        ...selection.token,
        mode: "translate",
        targetLanguage: saved.translation.targetLanguage,
        autoReferenceLanguage: autoReferenceForIntent(
          "translate",
          saved.translation.targetLanguage,
          saved.translation.autoReferenceLanguage,
        ),
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

  async function chooseAutoReferenceLanguage(
    autoReferenceLanguage: TranslationReferenceLanguage,
  ) {
    const saved = await saveSettings({
      ...settings,
      translation: { ...settings.translation, autoReferenceLanguage },
    });
    if (!saved) {
      return;
    }
    autoReferenceLanguageRef.current = saved.translation.autoReferenceLanguage;
    if (
      selection &&
      saved.mode === "translate" &&
      saved.translation.targetLanguage === "auto"
    ) {
      const intent: RewriteIntentToken = {
        ...selection.token,
        mode: "translate",
        targetLanguage: "auto",
        autoReferenceLanguage: saved.translation.autoReferenceLanguage,
      };
      currentIntentRef.current = intent;
      resultIntentRef.current = null;
      setResult(null);
      setDraft("");
      setStatus("Auto reference language changed");
      if (saved.autoRewrite) {
        void rewrite(
          "translate",
          selection.token,
          "auto",
          saved.translation.autoReferenceLanguage,
        );
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

  function invalidateTerminologyResult(message: string) {
    terminologyEpochRef.current += 1;
    resultIntentRef.current = null;
    setResult(null);
    setDraft("");
    setImportPreview(null);
    setStatus(message);
  }

  async function runTerminologyMutation(
    command: string,
    args: Record<string, unknown>,
    message: string,
  ): Promise<boolean> {
    try {
      const snapshot = requireTerminologySnapshot(await invoke<unknown>(command, args));
      setTerminology(snapshot);
      setError(null);
      invalidateTerminologyResult(message);
      return true;
    } catch (nextError) {
      setError(toErrorMessage(nextError));
      setStatus("Terminology change rejected");
      return false;
    }
  }

  async function toggleTerminologySetting(
    field: "enabled" | "useApprovedTerminology" | "suggestTerminology",
  ) {
    const saved = await saveSettings({
      ...settings,
      terminology: {
        ...settings.terminology,
        [field]: !settings.terminology[field],
        autoSaveSuggestions: false,
      },
    });
    if (saved) invalidateTerminologyResult("Terminology settings changed — rewrite again");
  }

  async function addProfile() {
    if (!profileNameDraft.trim()) return;
    if (await runTerminologyMutation(
      "add_terminology_profile",
      { name: profileNameDraft },
      "Terminology profile added",
    )) setProfileNameDraft("");
  }

  async function renameProfile(profileId: string, currentName: string) {
    const name = window.prompt("Rename this local profile", currentName);
    if (name === null || name === currentName) return;
    await runTerminologyMutation(
      "rename_terminology_profile",
      { profileId, name },
      "Terminology profile renamed",
    );
  }

  async function toggleProfile(profileId: string, enabled: boolean) {
    await runTerminologyMutation(
      "set_terminology_profile_enabled",
      { profileId, enabled: !enabled },
      enabled ? "Terminology profile disabled" : "Terminology profile enabled",
    );
  }

  async function chooseTerminologyProfile(profileId: string) {
    try {
      const saved = parseAppSettings(await invoke<unknown>("set_active_terminology_profile", { profileId }));
      if (!saved) throw new Error("Active-profile response was invalid.");
      setSettings(saved);
      setEntryDraft((current) => ({ ...current, profileId }));
      setError(null);
      invalidateTerminologyResult("Active terminology profile changed — rewrite again");
    } catch (nextError) {
      setError(toErrorMessage(nextError));
    }
  }

  async function submitTerminologyEntry() {
    const draft: TerminologyEntryDraft = {
      ...entryDraft,
      preferredText: entryDraft.type === "protected" ? null : entryDraft.preferredText,
      aliases: entryDraft.aliases.map((alias) => alias.trim()).filter(Boolean),
    };
    const command = editingEntryId ? "update_terminology_entry" : "add_terminology_entry";
    const args = editingEntryId ? { entryId: editingEntryId, draft } : { draft };
    if (await runTerminologyMutation(command, args, editingEntryId ? "Terminology entry updated" : "Terminology entry added")) {
      setEditingEntryId(null);
      setEntryDraft({ ...EMPTY_ENTRY_DRAFT, profileId: settings.terminology.activeProfileId });
    }
  }

  async function setEntryStatus(entryId: string, status: TerminologyEntryStatus) {
    await runTerminologyMutation(
      status === "approved" ? "approve_terminology_suggestion" : "set_terminology_entry_status",
      status === "approved" ? { entryId } : { entryId, status },
      status === "approved" ? "Terminology entry approved" : "Terminology status changed",
    );
  }

  async function deleteEntry(entryId: string) {
    if (!window.confirm("Delete this local terminology entry?")) return;
    await runTerminologyMutation(
      "delete_terminology_entry",
      { entryId },
      "Terminology entry deleted",
    );
  }

  async function saveSuggestion(suggestion: TerminologySuggestion) {
    await runTerminologyMutation(
      "save_terminology_suggestion",
      { profileId: settings.terminology.activeProfileId, suggestion },
      "Suggestion saved as suggested — approve it explicitly to activate",
    );
  }

  function dismissSuggestion(index: number) {
    setResult((current) => current ? {
      ...current,
      terminologySuggestions: current.terminologySuggestions.filter((_, itemIndex) => itemIndex !== index),
    } : current);
  }

  function openSuggestionForm(suggestion: TerminologySuggestion) {
    setEntryDraft({
      ...EMPTY_ENTRY_DRAFT,
      profileId: settings.terminology.activeProfileId,
      type: suggestion.type,
      status: "suggested",
      sourceText: suggestion.sourceText,
      preferredText: suggestion.preferredText,
      sourceLanguage: suggestion.sourceLanguage,
      targetLanguage: suggestion.targetLanguage,
      occurrenceCount: 1,
    });
    setEditingEntryId(null);
    setSettingsOpen(true);
  }

  async function exportTerminology(format: "json" | "csv") {
    try {
      const content = await invoke<unknown>("export_terminology", { format });
      if (typeof content !== "string") throw new Error("Terminology export response was invalid.");
      const blob = new Blob([content], { type: format === "json" ? "application/json" : "text/csv" });
      const url = URL.createObjectURL(blob);
      const anchor = document.createElement("a");
      anchor.href = url;
      anchor.download = `codex-pencil-terminology.${format}`;
      anchor.click();
      URL.revokeObjectURL(url);
      setStatus(`Terminology ${format.toUpperCase()} exported locally`);
    } catch (nextError) {
      setError(toErrorMessage(nextError));
    }
  }

  async function dryRunImport() {
    try {
      const preview = parseImportPlanPreview(await invoke<unknown>("dry_run_terminology_import", {
        format: importFormat,
        text: importText,
      }));
      if (!preview) throw new Error("Import dry-run response was invalid.");
      setImportPreview(preview);
      setError(null);
      setStatus("Import dry run ready — no data changed");
    } catch (nextError) {
      setError(toErrorMessage(nextError));
    }
  }

  async function selectImportFile(file: File | undefined) {
    setImportPreview(null);
    if (!file) return;
    if (file.size > 2 * 1024 * 1024) {
      setError("Terminology import exceeds the 2 MiB local limit.");
      return;
    }
    try {
      const bytes = await file.arrayBuffer();
      setImportText(new TextDecoder("utf-8", { fatal: true }).decode(bytes));
      setImportFormat(file.name.toLocaleLowerCase().endsWith(".csv") ? "csv" : "json");
      setError(null);
      setStatus("Terminology import loaded locally — run dry run before apply");
    } catch {
      setError("The terminology import could not be read as UTF-8 text.");
    }
  }

  async function applyImport() {
    if (!importPreview) return;
    try {
      const report = parseImportReport(await invoke<unknown>("apply_terminology_import", {
        planId: importPreview.planId,
      }));
      if (!report) throw new Error("Import apply response was invalid.");
      await refreshTerminology();
      setImportPreview(null);
      setImportText("");
      invalidateTerminologyResult("Terminology import applied — conflicts remained skipped");
    } catch (nextError) {
      setError(toErrorMessage(nextError));
    }
  }

  async function resetTerminologyStore() {
    if (!window.confirm("Reset the unrecoverable local terminology store?")) return;
    if (await runTerminologyMutation("reset_terminology_store", {}, "Terminology store reset")) {
      setSettings((current) => ({
        ...current,
        terminology: { ...current.terminology, activeProfileId: "general" },
      }));
    }
  }

  async function applyReplacement() {
    const token = currentTokenRef.current;
    const intent = currentIntentRef.current;
    if (!token || !intent || !sameRewriteIntent(resultIntentRef.current, intent) || draft.length === 0) {
      return;
    }
    const draftLimit = validateFrontendTextLimit("final_apply", draft);
    if (draftLimit) {
      setError(contentLimitMessage(draftLimit));
      setStatus("Replacement too large");
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
        autoReferenceLanguage: intent.autoReferenceLanguage,
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
        void refreshTerminology();
        await dismiss();
        return;
      }
      if (outcome.status === "copied_fallback") {
        currentTokenRef.current = null;
        currentIntentRef.current = null;
        resultIntentRef.current = null;
        setSelection(null);
        setStatus("Copied — paste manually");
        void refreshTerminology();
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
      if (!isBackendContentLimitError(nextError)) {
        currentTokenRef.current = null;
        setSelection(null);
      }
      setError(toErrorMessage(nextError));
      setStatus("Apply failed safely");
    } finally {
      if (sameCaptureToken(applyingTokenRef.current, token)) {
        applyingTokenRef.current = null;
        setIsApplying(false);
      }
    }
  }

  function beginWindowDrag(event: ReactMouseEvent<HTMLElement>) {
    void startWindowDrag(event, getCurrentWindow()).catch(() => undefined);
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

  const activeProviderStatus = providerSnapshot?.statuses.find((item) => item.kind === settings.activeProvider) ?? null;
  const providerReady = activeProviderStatus?.state === "ready" || (settings.activeProvider === "codex" && auth?.loggedIn === true);
  const canRewrite = Boolean(selection && providerReady && !isRewriting && !result);
  const canApply = Boolean(
    selection &&
      (result || instantRuntime.activeKind === "instant") &&
      draft.length > 0 &&
      !isApplying &&
      !isRewriting &&
      (result ? sameRewriteIntent(resultIntentRef.current, currentIntentRef.current) : true),
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
        <header className="topbar" onMouseDown={beginWindowDrag}>
          <div className="brand">
            <div className="brand-mark">
              <Sparkles size={16} strokeWidth={2.2} />
            </div>
            <div>
              <h1>Codex Pencil</h1>
              <p>
                {providerDisplayName(settings.activeProvider)}
                {activeProviderStatus ? ` · ${activeProviderStatus.state.replace(/_/g, " ")}` : ""}
              </p>
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
            <section className="provider-settings" aria-label="Providers">
              <div className="translation-settings-heading">
                <div>
                  <h3>Providers</h3>
                  <p>One Provider is used for each cloud request. Instant stays local.</p>
                </div>
              </div>
              {(providerSnapshot?.statuses ?? [
                { kind: "codex", displayName: "Codex", state: "signed_out" },
                { kind: "antigravity", displayName: "Google Antigravity", state: "unavailable" },
                { kind: "claude", displayName: "Claude", state: "signed_out" },
              ] as Pick<ProviderStatus, "kind" | "displayName" | "state">[]).map((item) => {
                const status = providerSnapshot?.statuses.find((entry) => entry.kind === item.kind);
                const selected = settings.activeProvider === item.kind;
                const busy = Boolean(providerSnapshot?.busyKind);
                return (
                  <article className="provider-card" key={item.kind}>
                    <label className="provider-select">
                      <input
                        type="radio"
                        name="active-provider"
                        checked={selected}
                        disabled={busy}
                        onChange={() => {
                          void saveSettings({ ...settings, activeProvider: item.kind });
                        }}
                      />
                      <strong>{item.displayName}</strong>
                      <small>{(status?.state ?? item.state).replace(/_/g, " ")}</small>
                    </label>
                    {status?.reason ? <p>{status.reason}</p> : null}
                    {status?.setupRequirement ? <p>{status.setupRequirement}</p> : null}
                    <p className="provider-disclosure">
                      {item.kind === "codex"
                        ? "Codex sends selected text and matched approved terms through the official Codex app-server."
                        : item.kind === "antigravity"
                          ? "Google Antigravity sends selected text through the official `agy` CLI using your local Google session."
                          : "Claude sends selected text through official Claude Code CLI print mode using your Claude account."}
                    </p>
                    <div className="row-actions">
                      {status?.capabilities.officialSignIn || item.kind === "codex" ? (
                        <button
                          type="button"
                          disabled={busy}
                          onClick={() => {
                            if (item.kind === "codex") {
                              void startDeviceLogin();
                            } else {
                              void invoke("start_provider_login", { kind: item.kind })
                                .then(() => refreshProviders())
                                .catch((nextError) => setError(toErrorMessage(nextError)));
                            }
                          }}
                        >
                          {item.kind === "codex" ? "Connect Codex" : "Sign in"}
                        </button>
                      ) : null}
                      {status?.capabilities.officialSignOut ? (
                        <button
                          type="button"
                          disabled={busy}
                          onClick={() => {
                            void invoke("sign_out_provider", { kind: item.kind })
                              .then(() => refreshProviders())
                              .catch((nextError) => setError(toErrorMessage(nextError)));
                          }}
                        >
                          Sign out
                        </button>
                      ) : null}
                      <button type="button" onClick={() => void refreshProviders()}>
                        Refresh status
                      </button>
                    </div>
                  </article>
                );
              })}
            </section>
            <section className="translation-settings" aria-label="Automatic translation settings">
              <div className="translation-settings-heading">
                <div>
                  <h3>Translation</h3>
                  <p>Choose the language Auto treats as your primary reading language.</p>
                </div>
                <Languages size={18} />
              </div>
              <label className="settings-field">
                <span>Auto reference language</span>
                <select
                  value={settings.translation.autoReferenceLanguage}
                  onChange={(event) =>
                    chooseAutoReferenceLanguage(
                      event.target.value as TranslationReferenceLanguage,
                    )
                  }
                >
                  {REFERENCE_LANGUAGES.map((language) => (
                    <option key={language.id} value={language.id}>
                      {language.label}
                    </option>
                  ))}
                </select>
              </label>
              <p className="translation-direction">
                Other languages → {referenceLanguageLabel(settings.translation.autoReferenceLanguage)}.
                {" "}{referenceLanguageLabel(settings.translation.autoReferenceLanguage)} text → {referenceLanguageLabel(automaticFallbackLanguage(settings.translation.autoReferenceLanguage))}.
                Mixed or unclear text uses the reference language.
              </p>
            </section>
            <section className="terminology-settings" aria-label="개인 사전">
              <div className="terminology-heading">
                <div>
                  <h3>개인 사전</h3>
                  <p>로컬에만 저장됩니다. 선택문과 함께 모델로 전송되는 것은 일치한 approved 항목뿐입니다.</p>
                </div>
                <BookOpen size={18} />
              </div>
              <div className="terminology-toggles">
                <label>
                  <input
                    type="checkbox"
                    checked={settings.terminology.enabled}
                    onChange={() => toggleTerminologySetting("enabled")}
                  />
                  Enable
                </label>
                <label>
                  <input
                    type="checkbox"
                    checked={settings.terminology.useApprovedTerminology}
                    onChange={() => toggleTerminologySetting("useApprovedTerminology")}
                  />
                  Use approved
                </label>
                <label>
                  <input
                    type="checkbox"
                    checked={settings.terminology.suggestTerminology}
                    onChange={() => toggleTerminologySetting("suggestTerminology")}
                  />
                  Show suggestions
                </label>
              </div>

              {terminology?.status === "uninitialized" ? (
                <div className="terminology-recovery">
                  <span>{terminology.reason}</span>
                  <p>The local app-data path is unavailable. Reset and import stay disabled until the app can initialize that path.</p>
                </div>
              ) : terminology?.status === "unrecoverable" ? (
                <div className="terminology-recovery">
                  <span>{terminology.reason}</span>
                  <div className="export-actions">
                    <button className="secondary-button" type="button" onClick={resetTerminologyStore}>
                      <RotateCcw size={14} /> Reset local store
                    </button>
                    <label className="file-button"><Upload size={14} /> Import recovery<input type="file" accept=".json,.csv,application/json,text/csv" onChange={(event) => void selectImportFile(event.target.files?.[0])} /></label>
                  </div>
                  {importText ? (
                    <div className="import-plan">
                      <span>{importFormat.toUpperCase()} loaded locally ({new Blob([importText]).size.toLocaleString()} bytes)</span>
                      <button className="secondary-button" type="button" onClick={dryRunImport}>Dry run</button>
                      {importPreview ? (
                        <div>
                          <span>{importPreview.report.newProfiles} profiles · {importPreview.report.newEntries} entries · {importPreview.report.idConflicts + importPreview.report.semanticKeyConflicts} conflict</span>
                          <button className="primary-button" type="button" onClick={applyImport}>Recover from non-conflicting data</button>
                        </div>
                      ) : null}
                    </div>
                  ) : null}
                </div>
              ) : terminologyStore ? (
                <>
                  <label className="settings-field">
                    <span>Active non-global profile</span>
                    <select
                      value={settings.terminology.activeProfileId}
                      onChange={(event) => chooseTerminologyProfile(event.target.value)}
                    >
                      {terminologyStore.profiles
                        .filter((profile) => profile.id !== "global" && profile.enabled)
                        .map((profile) => <option key={profile.id} value={profile.id}>{profile.name}</option>)}
                    </select>
                  </label>
                  <div className="inline-form">
                    <input
                      value={profileNameDraft}
                      onChange={(event) => setProfileNameDraft(event.target.value)}
                      placeholder="New profile name"
                      maxLength={128}
                    />
                    <button className="secondary-button" type="button" onClick={addProfile}>
                      <Plus size={14} /> Profile
                    </button>
                  </div>
                  <div className="profile-list">
                    {terminologyStore.profiles.map((profile) => (
                      <div className="profile-row" key={profile.id}>
                        <span>{profile.name}</span>
                        <div className="badge-row">
                          {profile.id === "global" ? <small>global · always active</small> : null}
                          {profile.id === settings.terminology.activeProfileId ? <small>active</small> : null}
                        </div>
                        {profile.id !== "global" ? (
                          <div className="row-actions">
                            <button type="button" onClick={() => renameProfile(profile.id, profile.name)} aria-label="Rename profile"><Edit3 size={13} /></button>
                            <button type="button" onClick={() => toggleProfile(profile.id, profile.enabled)} disabled={profile.id === settings.terminology.activeProfileId}>
                              {profile.enabled ? "Disable" : "Enable"}
                            </button>
                          </div>
                        ) : null}
                      </div>
                    ))}
                  </div>

                  <div className="terminology-search">
                    <Search size={14} />
                    <input
                      aria-label="Terminology search"
                      value={terminologyQuery}
                      onChange={(event) => setTerminologyQuery(event.target.value)}
                      placeholder="Search source, preferred, alias, note"
                    />
                    <select aria-label="Terminology type filter" value={terminologyTypeFilter} onChange={(event) => setTerminologyTypeFilter(event.target.value as TerminologyEntryType | "all")}>
                      <option value="all">All types</option>
                      <option value="translation">translation</option>
                      <option value="preferred">preferred</option>
                      <option value="protected">protected</option>
                    </select>
                    <select aria-label="Terminology status filter" value={terminologyStatusFilter} onChange={(event) => setTerminologyStatusFilter(event.target.value as TerminologyEntryStatus | "all")}>
                      <option value="all">All states</option>
                      <option value="approved">approved</option>
                      <option value="suggested">suggested</option>
                      <option value="disabled">disabled</option>
                    </select>
                    <select aria-label="Terminology profile filter" value={terminologyProfileFilter} onChange={(event) => setTerminologyProfileFilter(event.target.value)}>
                      <option value="all">All profiles</option>
                      {terminologyStore.profiles.map((profile) => <option key={profile.id} value={profile.id}>{profile.name}</option>)}
                    </select>
                    <select aria-label="Terminology source-language filter" value={terminologySourceFilter} onChange={(event) => setTerminologySourceFilter(event.target.value as TerminologyLanguage | "all")}>
                      <option value="all">All source languages</option>
                      {(["any", "ko", "en", "ja", "zh-Hans", "zh-Hant"] as TerminologyLanguage[]).map((language) => <option key={language} value={language}>source: {language}</option>)}
                    </select>
                    <select aria-label="Terminology target-language filter" value={terminologyTargetFilter} onChange={(event) => setTerminologyTargetFilter(event.target.value as TerminologyLanguage | "all")}>
                      <option value="all">All target languages</option>
                      {(["any", "ko", "en", "ja", "zh-Hans", "zh-Hant"] as TerminologyLanguage[]).map((language) => <option key={language} value={language}>target: {language}</option>)}
                    </select>
                    <select aria-label="Terminology sort order" value={terminologySort} onChange={(event) => setTerminologySort(event.target.value as TerminologyEntrySort)}>
                      <option value="source_text">Sort: source</option>
                      <option value="priority">Sort: priority</option>
                      <option value="recently_updated">Sort: updated</option>
                      <option value="usage_count">Sort: usage</option>
                    </select>
                  </div>

                  <div className="entry-list-heading">
                    <strong>Saved entries ({filteredTerminologyEntries.length})</strong>
                    {filteredTerminologyEntries.length !== terminologyStore.entries.length ? (
                      <small>{terminologyStore.entries.length} total · reset filters to show hidden entries</small>
                    ) : null}
                  </div>
                  <div className="entry-list">
                    {filteredTerminologyEntries.map((entry) => {
                      const profileEnabled = terminologyStore.profiles
                        .some((profile) => profile.id === entry.profileId && profile.enabled);
                      const requestEligible = settings.terminology.enabled &&
                        settings.terminology.useApprovedTerminology &&
                        entryAffectsRequests(entry) && profileEnabled &&
                        (entry.profileId === "global" || entry.profileId === settings.terminology.activeProfileId);
                      return <article className="entry-row" key={entry.id}>
                        <div className="entry-copy">
                          <strong>{entry.sourceText}</strong>
                          <span>{entry.preferredText ?? "Protected exactly"}</span>
                          <div className="badge-row">
                            <small>{entry.type}</small><small>{entry.status}</small>
                            {!requestEligible ? <small>inert</small> : null}
                          </div>
                        </div>
                        <div className="row-actions">
                          <button type="button" onClick={() => { setEditingEntryId(entry.id); setEntryDraft(draftFromEntry(entry)); }} aria-label="Edit entry"><Edit3 size={13} /></button>
                          {entry.status === "approved" ? <button type="button" onClick={() => setEntryStatus(entry.id, "disabled")}>Disable</button> : null}
                          {entry.status === "suggested" ? <><button type="button" onClick={() => setEntryStatus(entry.id, "approved")}>Approve</button><button type="button" onClick={() => setEntryStatus(entry.id, "disabled")}>Disable</button></> : null}
                          {entry.status === "disabled" ? <button type="button" onClick={() => setEntryStatus(entry.id, "approved")}>Re-enable</button> : null}
                          <button type="button" onClick={() => deleteEntry(entry.id)} aria-label="Delete entry"><Trash2 size={13} /></button>
                        </div>
                      </article>;
                    })}
                  </div>

                  <div className="entry-form">
                    <div className="entry-form-heading">
                      <strong>{editingEntryId ? "Edit entry" : "Add entry"}</strong>
                      {editingEntryId ? <button type="button" onClick={() => { setEditingEntryId(null); setEntryDraft({ ...EMPTY_ENTRY_DRAFT, profileId: settings.terminology.activeProfileId }); }}>Cancel</button> : null}
                    </div>
                    <div className="entry-form-grid">
                      <select aria-label="Terminology entry profile" value={entryDraft.profileId} onChange={(event) => setEntryDraft((current) => ({ ...current, profileId: event.target.value }))}>
                        {terminologyStore.profiles.filter((profile) => profile.enabled).map((profile) => <option key={profile.id} value={profile.id}>{profile.name}</option>)}
                      </select>
                      <select aria-label="Terminology entry type" value={entryDraft.type} onChange={(event) => setEntryDraft((current) => ({ ...current, type: event.target.value as TerminologyEntryType }))}>
                        <option value="translation">translation</option>
                        <option value="preferred">preferred</option>
                        <option value="protected">protected</option>
                      </select>
                      <select aria-label="Terminology entry status" value={entryDraft.status} onChange={(event) => setEntryDraft((current) => ({ ...current, status: event.target.value as TerminologyEntryStatus }))}>
                        <option value="approved">approved</option>
                        <option value="suggested">suggested</option>
                        <option value="disabled">disabled</option>
                      </select>
                      <input value={entryDraft.sourceText} maxLength={256} onChange={(event) => setEntryDraft((current) => ({ ...current, sourceText: event.target.value }))} placeholder="Source term" />
                      {entryDraft.type !== "protected" ? <input value={entryDraft.preferredText ?? ""} maxLength={512} onChange={(event) => setEntryDraft((current) => ({ ...current, preferredText: event.target.value }))} placeholder="Preferred term" /> : null}
                      <textarea value={entryDraft.aliases.join("\n")} onChange={(event) => setEntryDraft((current) => ({ ...current, aliases: event.target.value.split(/\r?\n/) }))} placeholder="Aliases, one per line" rows={2} />
                      <select aria-label="Terminology entry source language" value={entryDraft.sourceLanguage} onChange={(event) => setEntryDraft((current) => ({ ...current, sourceLanguage: event.target.value as TerminologyLanguage }))}>
                        {(["any", "ko", "en", "ja", "zh-Hans", "zh-Hant"] as TerminologyLanguage[]).map((language) => <option key={language} value={language}>source: {language}</option>)}
                      </select>
                      <select aria-label="Terminology entry target language" value={entryDraft.targetLanguage} onChange={(event) => setEntryDraft((current) => ({ ...current, targetLanguage: event.target.value as TerminologyLanguage }))}>
                        {(["any", "ko", "en", "ja", "zh-Hans", "zh-Hant"] as TerminologyLanguage[]).map((language) => <option key={language} value={language}>target: {language}</option>)}
                      </select>
                      <input type="number" min={0} max={1000} value={entryDraft.priority} onChange={(event) => setEntryDraft((current) => ({ ...current, priority: Number(event.target.value) }))} aria-label="Priority" />
                      <input value={entryDraft.note ?? ""} maxLength={1024} onChange={(event) => setEntryDraft((current) => ({ ...current, note: event.target.value || null }))} placeholder="Local note (optional)" />
                    </div>
                    <label className="case-toggle"><input type="checkbox" checked={entryDraft.caseSensitive} onChange={(event) => setEntryDraft((current) => ({ ...current, caseSensitive: event.target.checked }))} /> Case sensitive</label>
                    <button className="primary-button" type="button" onClick={submitTerminologyEntry}>
                      <Check size={14} /> {editingEntryId ? "Save entry" : "Add entry"}
                    </button>
                  </div>

                  <div className="import-export">
                    <div className="export-actions">
                      <button className="secondary-button" type="button" onClick={() => exportTerminology("json")}><Download size={14} /> JSON</button>
                      <button className="secondary-button" type="button" onClick={() => exportTerminology("csv")}><Download size={14} /> CSV</button>
                      <label className="file-button"><Upload size={14} /> Import<input type="file" accept=".json,.csv,application/json,text/csv" onChange={(event) => void selectImportFile(event.target.files?.[0])} /></label>
                    </div>
                    {importText ? (
                      <div className="import-plan">
                        <span>{importFormat.toUpperCase()} loaded locally ({new Blob([importText]).size.toLocaleString()} bytes)</span>
                        <button className="secondary-button" type="button" onClick={dryRunImport}>Dry run</button>
                        {importPreview ? (
                          <div>
                            <span>{importPreview.report.newProfiles} profiles · {importPreview.report.newEntries} entries · {importPreview.report.identicalDuplicates} duplicate · {importPreview.report.idConflicts + importPreview.report.semanticKeyConflicts} conflict · {importPreview.report.invalidRows} invalid · {importPreview.report.skippedRows} skipped</span>
                            {importPreview.report.conflicts.length ? (
                              <ul className="import-conflict-list">
                                {importPreview.report.conflicts.slice(0, 50).map((conflict, index) => (
                                  <li key={`${conflict.kind}-${conflict.incomingId ?? "none"}-${index}`}>
                                    {conflict.kind} · incoming {conflict.incomingId ?? "n/a"} · existing {conflict.existingId ?? "n/a"}{conflict.rowNumber === null ? "" : ` · row ${conflict.rowNumber}`}
                                  </li>
                                ))}
                                {importPreview.report.conflicts.length > 50 ? <li>{importPreview.report.conflicts.length - 50} more conflicts</li> : null}
                              </ul>
                            ) : null}
                            <button className="primary-button" type="button" onClick={applyImport}>Apply non-conflicting</button>
                          </div>
                        ) : null}
                      </div>
                    ) : null}
                  </div>
                </>
              ) : <div className="loading-state"><Loader2 className="spin" size={16} /> Loading local terminology</div>}
            </section>
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
        ) : settings.activeProvider === "codex" && !auth?.loggedIn && !selection ? (
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
                  <button type="button" onClick={openDeviceLoginPage}>
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
        ) : selection &&
          cloudAckFor(settings, settings.activeProvider) < CLOUD_PROCESSING_DISCLOSURE_VERSION ? (
          <div className="disclosure-pane" role="dialog" aria-labelledby="cloud-disclosure-title">
            <div className="login-copy">
              <ShieldCheck size={22} />
              <div>
                <h2 id="cloud-disclosure-title">AI 클라우드 처리 안내</h2>
                <p>전송 전에 내용을 확인해 주세요.</p>
              </div>
            </div>
            <ul className="disclosure-list">
              <li>선택한 텍스트와 현재 선택에 일치한 승인 용어만 {providerDisplayName(settings.activeProvider)}로 전송되어 AI 처리될 수 있습니다. 다른 Provider로 자동 전환되지 않습니다.</li>
              <li>전체 용어 사전, 이전 문서, 화면 이미지, 문서 기록은 이 앱이 전송하지 않습니다.</li>
              <li>용어 관리는 로컬에서 동작하지만 AI 교정·번역은 오프라인 모델이 아닙니다.</li>
              <li>취소하면 현재 선택은 전송되지 않습니다.</li>
            </ul>
            <div className="device-actions">
              <button className="primary-button" type="button" onClick={acknowledgeCloudProcessing}>
                동의하고 계속
              </button>
              <button className="secondary-button" type="button" onClick={dismiss}>
                취소
              </button>
            </div>
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
                        {language.id === "auto"
                          ? `Auto · ${referenceLanguageLabel(settings.translation.autoReferenceLanguage)} reference`
                          : language.label}
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
                {settings.translation.targetLanguage === "auto" ? (
                  <p>
                    Auto direction: other languages → {referenceLanguageLabel(settings.translation.autoReferenceLanguage)};
                    {" "}{referenceLanguageLabel(settings.translation.autoReferenceLanguage)} → {referenceLanguageLabel(automaticFallbackLanguage(settings.translation.autoReferenceLanguage))}.
                    Change the reference language in Settings.
                  </p>
                ) : null}
              </div>
            ) : null}

            <div className="mode-grid" aria-label="Rewrite mode">
              {MODES.map((mode) => {
                const isActive = mode.id === selectedMode.id;
                const Icon = mode.icon === "languages" ? Languages : Sparkles;
                return (
                  <button
                    className={`${isActive ? "mode-button active" : "mode-button"}${
                      mode.id === "translate" ? " translate-mode" : ""
                    }`}
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

            <section className="result-card" aria-label="Editable rewrite result">
              <div className="result-heading">
                <div>
                  <strong>{settings.mode === "translate" ? "Translation" : "Replacement"}</strong>
                  <span>
                    {instantRuntime.activeKind === "instant"
                      ? "Instant local candidate"
                      : result
                        ? `Deep · ${providerDisplayName(result.providerUsed)}`
                        : "Edit the text before applying it."}
                  </span>
                </div>
                {result ? (
                  <strong className="confidence-badge">
                    {Math.round(result.confidence * 100)}% · {result.terminologyMatchCount} terms
                  </strong>
                ) : null}
              </div>

              <div className="result-area">
                {isRewriting ? (
                  <div className="loading-state">
                    <Loader2 className="spin" size={18} />
                    <span>Working through {providerDisplayName(settings.activeProvider)}</span>
                  </div>
                ) : (
                  <textarea
                    value={draft}
                    onChange={(event) => {
                      const text = event.target.value;
                      setDraft(text);
                      setInstantRuntime((current) => editDraft(current, text));
                    }}
                    placeholder={
                      settings.mode === "translate"
                        ? "Translation will appear here"
                        : "Replacement will appear here"
                    }
                    aria-label="Editable rewrite result text"
                    spellCheck={false}
                  />
                )}
              </div>

              {settings.mode === "grammar" && visibleChoices(instantRuntime).length > 0 ? (
                <div className="candidate-choices" role="list" aria-label="Instant and Deep candidates">
                  {visibleChoices(instantRuntime).map((choice) => (
                    <button
                      key={`${choice.kind}-${choice.generation}`}
                      type="button"
                      role="listitem"
                      aria-pressed={instantRuntime.activeKind === choice.kind}
                      onClick={() => {
                        setInstantRuntime((current) => {
                          const nextState = requestSwitch(current, choice);
                          if (!current.dirty) {
                            setDraft(choice.text);
                          }
                          return nextState;
                        });
                      }}
                    >
                      {choice.kind === "instant" ? "Instant" : "Deep"}
                    </button>
                  ))}
                </div>
              ) : null}
              {instantRuntime.pendingSwitch ? (
                <div className="candidate-confirm" role="dialog" aria-label="Discard edited draft">
                  <p>Replace the edited draft with the selected candidate?</p>
                  <button
                    type="button"
                    onClick={() => {
                      setInstantRuntime((current) => {
                        const nextState = confirmSwitch(current);
                        setDraft(nextState.draft);
                        return nextState;
                      });
                    }}
                  >
                    Replace draft
                  </button>
                  <button
                    type="button"
                    onClick={() => setInstantRuntime((current) => cancelSwitch(current))}
                  >
                    Keep edit
                  </button>
                </div>
              ) : null}
              {result ? (
                <div className="rewrite-details">
                  <div className="rewrite-summary">
                    <span>{result.summary}</span>
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
                  {result.terminologyWarnings.length ? (
                    <div className="terminology-warnings" role="status">
                      {result.terminologyWarnings.map((warning, index) => (
                        <span key={`${warning.code}-${index}`}>{warning.code.replace(/_/g, " ")}</span>
                      ))}
                      <small>Warnings never auto-correct or auto-Apply the result.</small>
                    </div>
                  ) : null}
                  {result.terminologySuggestions.length ? (
                    <div className="suggestion-list">
                      {result.terminologySuggestions.map((suggestion, index) => (
                        <article key={`${suggestion.sourceText}-${index}`}>
                          <div><strong>{suggestion.sourceText}</strong><span>{suggestion.preferredText}</span><small>{suggestion.reason.replace(/_/g, " ")} · ephemeral</small></div>
                          <div className="row-actions">
                            <button type="button" onClick={() => dismissSuggestion(index)}>Dismiss</button>
                            <button type="button" onClick={() => openSuggestionForm(suggestion)}>Review</button>
                            <button type="button" onClick={() => saveSuggestion(suggestion)}>Save suggested</button>
                          </div>
                        </article>
                      ))}
                    </div>
                  ) : null}
                </div>
              ) : null}

              <div className="result-actions">
                <button className="secondary-button" type="button" onClick={() => rewrite()} disabled={!canRewrite}>
                  <RefreshCcw size={16} />
                  Rewrite
                </button>
                <button className="primary-button" type="button" onClick={applyReplacement} disabled={!canApply}>
                  {isApplying ? <Loader2 className="spin" size={16} /> : <Check size={16} />}
                  Apply
                </button>
              </div>
            </section>
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
