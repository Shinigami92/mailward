import type { DecisionKind } from "@/lib/decisions";
import type { DeepReadonly, Ref } from "vue";
import { ref } from "vue";

export interface Decision {
  account: string;
  folder: string;
  uid: string;
  subject: string;
  from: string;
  decision: DecisionKind;
  reason: string;
}
export interface RunResult {
  runId: string;
  scanned: number;
  actioned: number;
  applied: boolean;
  decisions: Decision[];
}
export interface ProgressEvent {
  runId: string;
  kind: string;
  folder: string | null;
  index: number | null;
  total: number | null;
  fetched: number | null;
  fetchTotal: number | null;
  decision: Decision | null;
}
export interface Phase {
  folder: string;
  index: number;
  total: number;
  fetched: number;
  fetchTotal: number;
}

export type RunVariables = {
  account: string | null;
  mode: string;
  dryRun: boolean;
  runId: string;
};
export type RunExecutor = (
  variables: Readonly<RunVariables>,
) => Promise<{ data?: { triggerRun?: RunResult } | null; error?: { message: string } | null }>;

export interface RunController {
  account: Ref<string>;
  mode: Ref<"CLASSIFY" | "CLEANUP">;
  apply: Ref<boolean>;
  result: Ref<RunResult | null>;
  live: Ref<Decision[]>;
  phase: Ref<Phase | null>;
  errorMessage: Ref<string>;
  fetching: Ref<boolean>;
  setExecutor: (fn: RunExecutor) => void;
  applyEvent: (event: DeepReadonly<ProgressEvent>) => void;
  execute: (dryRun: boolean) => Promise<void>;
  ensureLoaded: () => void;
}

// Module-level singletons: this state lives ABOVE any component, so navigating away from the
// Run view and back (or remounting it) reattaches to the same run instead of starting a fresh
// one - the in-flight mutation keeps filling `result`, and on remount the subscription rebinds
// and resumes streaming for the still-active run. (Each browser tab has its own module instance,
// so a second tab is independent; the server `run_lock` serializes them and `runId` keeps their
// event streams from bleeding into each other.)
const account = ref("all");
const mode = ref<"CLASSIFY" | "CLEANUP">("CLASSIFY");
const apply = ref(false);
const result = ref<RunResult | null>(null);
const live = ref<Decision[]>([]);
const phase = ref<Phase | null>(null);
const errorMessage = ref("");
const fetching = ref(false);

// Non-reactive run bookkeeping.
let executor: RunExecutor | null = null;
let runToken = 0;
let inFlight = false;
let queuedDry: boolean | null = null;
// The id of the run whose events we currently render. Empty between runs / right after a
// supersede, so stray events (a superseded run still draining, or another tab) are ignored.
let currentRunId = "";
let runSeq = 0;

function newRunId(): string {
  if (typeof crypto !== "undefined" && typeof crypto.randomUUID === "function") {
    return crypto.randomUUID();
  }
  runSeq += 1;
  return `web-${runSeq}`;
}

// The component registers urql's mutation executor here. It stays valid after the component
// unmounts (it is bound to the app-level client), so an in-flight run finishes regardless.
function setExecutor(fn: RunExecutor): void {
  executor = fn;
}

// Route a subscription event into the live view, but only if it belongs to the run we started.
function applyEvent(event: DeepReadonly<ProgressEvent>): void {
  if (event.runId !== currentRunId) return;
  if (
    event.kind === "folder" &&
    event.folder != null &&
    event.index != null &&
    event.total != null
  ) {
    phase.value = {
      folder: event.folder,
      index: event.index,
      total: event.total,
      fetched: event.fetched ?? 0,
      fetchTotal: event.fetchTotal ?? 0,
    };
  } else if (event.kind === "decision" && event.decision != null) {
    live.value.push({ ...event.decision });
  }
}

// Clear the rendered run (kept separate so it is reused at start and on each queued restart).
function clearView(): void {
  result.value = null;
  live.value = [];
  phase.value = null;
}

// One mutation round trip, tagged with a fresh runId so its broadcast events are attributable.
async function runPass(useDry: boolean): Promise<void> {
  const token = runToken;
  const runId = newRunId();
  currentRunId = runId;
  if (executor === null) {
    errorMessage.value = "run executor not ready";
    return;
  }
  const response = await executor({
    account: account.value === "all" ? null : account.value,
    mode: mode.value,
    dryRun: useDry,
    runId,
  });
  // Apply the result only if this run wasn't superseded mid-flight.
  if (token === runToken) {
    if (response.error == null) {
      result.value = response.data?.triggerRun ?? null;
    } else {
      errorMessage.value = response.error.message;
    }
  }
}

async function execute(dryRun: boolean): Promise<void> {
  // Bumping the token + clearing `currentRunId` immediately stops any in-flight run from
  // updating the view, and resets it for the new request.
  runToken += 1;
  currentRunId = "";
  errorMessage.value = "";
  clearView();
  if (inFlight) {
    // A run is already going; queue this request - the loop below picks it up next.
    queuedDry = dryRun;
    return;
  }

  inFlight = true;
  fetching.value = true;
  let pending: boolean | null = dryRun;
  while (pending !== null) {
    const useDry = pending;
    pending = null;
    // Sequential by design: runs must not overlap (concurrency is the bug we're fixing),
    // so Promise.all / parallelism is explicitly not wanted here.
    // oxlint-disable-next-line eslint/no-await-in-loop
    await runPass(useDry);
    if (queuedDry !== null) {
      pending = queuedDry;
      queuedDry = null;
      clearView();
    }
  }
  inFlight = false;
  fetching.value = false;
}

// Auto-run a dry run when the view opens IF there is nothing to show yet and no run is
// already going - so returning to the view reattaches to an existing result/run instead of
// kicking off a redundant scan.
function ensureLoaded(): void {
  if (result.value !== null || inFlight) return;
  void execute(true);
}

export function useRunController(): RunController {
  return {
    account,
    mode,
    apply,
    result,
    live,
    phase,
    errorMessage,
    fetching,
    setExecutor,
    applyEvent,
    execute,
    ensureLoaded,
  };
}
