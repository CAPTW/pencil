export type ProviderKind = "codex" | "antigravity" | "claude";

export type ProviderLifecycleState =
  | "unavailable"
  | "signed_out"
  | "authenticating"
  | "ready"
  | "busy"
  | "cancelling"
  | "faulted"
  | "signed_out_pending_cleanup";

export type ProviderStatus = Readonly<{
  kind: ProviderKind;
  displayName: string;
  state: ProviderLifecycleState;
  available: boolean;
  executablePath: string | null;
  version: string | null;
  accountLabel: string | null;
  reason: string | null;
  setupRequirement: string | null;
  capabilities: Readonly<{
    writing: boolean;
    officialSignIn: boolean;
    officialSignOut: boolean;
    cancellation: boolean;
  }>;
}>;

export type SelfTestRecord = Readonly<{
  kind: ProviderKind;
  classification: string;
  startedUtc: string;
  durationMs: number;
  errorCode: string | null;
}>;

export type ProviderSnapshot = Readonly<{
  active: ProviderKind;
  busyKind: ProviderKind | null;
  statuses: ProviderStatus[];
  lastSelfTests?: SelfTestRecord[];
}>;

const KINDS = new Set<ProviderKind>(["codex", "antigravity", "claude"]);
const STATES = new Set<ProviderLifecycleState>([
  "unavailable",
  "signed_out",
  "authenticating",
  "ready",
  "busy",
  "cancelling",
  "faulted",
  "signed_out_pending_cleanup",
]);

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

export function providerDisplayName(kind: ProviderKind): string {
  switch (kind) {
    case "codex":
      return "Codex";
    case "antigravity":
      return "Google Antigravity";
    case "claude":
      return "Claude";
  }
}

export function parseProviderSnapshot(value: unknown): ProviderSnapshot | null {
  if (!isRecord(value) || typeof value.active !== "string" || !KINDS.has(value.active as ProviderKind)) {
    return null;
  }
  const busyKind =
    value.busyKind === null || value.busyKind === undefined
      ? null
      : typeof value.busyKind === "string" && KINDS.has(value.busyKind as ProviderKind)
        ? (value.busyKind as ProviderKind)
        : null;
  if (value.busyKind !== null && value.busyKind !== undefined && busyKind === null) {
    return null;
  }
  if (!Array.isArray(value.statuses) || value.statuses.length !== 3) {
    return null;
  }
  const statuses: ProviderStatus[] = [];
  for (const item of value.statuses) {
    if (!isRecord(item) || typeof item.kind !== "string" || !KINDS.has(item.kind as ProviderKind)) {
      return null;
    }
    if (typeof item.state !== "string" || !STATES.has(item.state as ProviderLifecycleState)) {
      return null;
    }
    if (!isRecord(item.capabilities)) {
      return null;
    }
    statuses.push({
      kind: item.kind as ProviderKind,
      displayName: typeof item.displayName === "string" ? item.displayName : providerDisplayName(item.kind as ProviderKind),
      state: item.state as ProviderLifecycleState,
      available: item.available === true,
      executablePath: typeof item.executablePath === "string" ? item.executablePath : null,
      version: typeof item.version === "string" ? item.version : null,
      accountLabel: typeof item.accountLabel === "string" ? item.accountLabel : null,
      reason: typeof item.reason === "string" ? item.reason : null,
      setupRequirement: typeof item.setupRequirement === "string" ? item.setupRequirement : null,
      capabilities: {
        writing: item.capabilities.writing === true,
        officialSignIn: item.capabilities.officialSignIn === true,
        officialSignOut: item.capabilities.officialSignOut === true,
        cancellation: item.capabilities.cancellation === true,
      },
    });
  }
  return {
    active: value.active as ProviderKind,
    busyKind,
    statuses,
    lastSelfTests: Array.isArray(value.lastSelfTests)
      ? value.lastSelfTests.flatMap((item) => {
          if (!isRecord(item) || typeof item.kind !== "string" || !KINDS.has(item.kind as ProviderKind)) {
            return [];
          }
          return [{
            kind: item.kind as ProviderKind,
            classification: typeof item.classification === "string" ? item.classification : "product_failure",
            startedUtc: typeof item.startedUtc === "string" ? item.startedUtc : "",
            durationMs: typeof item.durationMs === "number" ? item.durationMs : 0,
            errorCode: typeof item.errorCode === "string" ? item.errorCode : null,
          }];
        })
      : [],
  };
}
