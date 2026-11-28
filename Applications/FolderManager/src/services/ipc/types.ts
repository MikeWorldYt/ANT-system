// IPC contract shared with the Rust backend; mirror of the serde types.
export type TypeBase = "NODE" | "WKB" | "OUT" | "REV" | "RES" | "TPL" | "CFG" | "SS" | "REC" | "UNKNOWN";
export type EditableType = "NODE" | "WKB";
export type NewKind = "node" | "worknode";
export type WkbStatus = "in-progress" | "for-review" | "hold" | "done";
export type MoveDirection = "up" | "down";
export type BoardColumn = "backlog" | "todo" | "progress" | "done";

export interface NodeType {
  raw: string;
  base: TypeBase;
  level: number | null;
}

export interface NodeInfo {
  fields: { key: string; value: string }[];
  error: string | null;
}

export interface HiddenChoice {
  node: TreeNode;
  hidden: boolean;
  fixed: boolean;
}

export interface NodeActions {
  rename: boolean;
  changeType: boolean;
  addNext: boolean;
  addChild: boolean;
  addFolder: boolean;
  moveUp: boolean;
  moveDown: boolean;
  delete: boolean;
  copyPath: boolean;
  open: boolean;
  favorite: boolean;
  important: boolean;
  editDetails: boolean;
  editFlags: boolean;
  hideLink: boolean;
  hide: boolean;
}

export interface TreeNode {
  name: string;
  relParts: string[];
  depth: number;
  ghost: boolean;
  inYaml: boolean;
  nodeType: NodeType | null;
  status: WkbStatus | null;
  statusRaw: string | null;
  locked: boolean;
  reserved: boolean;
  linked: boolean;
  linkPath: string | null;
  flags: string[];
  hasChildren: boolean;
  isGit: boolean;
  fixedPrefix: string;
  editableName: string;
  favorite: boolean;
  important: boolean;
  hidden: boolean;
  concealed: boolean;
  file: boolean;
  info: NodeInfo | null;
  children: TreeNode[] | null;
  absPath: string;
  actions: NodeActions;
  // Milliseconds since the epoch; size only for files.
  stamps: { modified: number | null; created: number | null; size: number | null };
}

// `name` is the file's name after the run; `project` the folder the sort would put it in.
// A review folder that runs its automations on its own files them on at once; `chained` says where they went next.
export interface ApplyOutcome {
  changed: number;
  chained: Chained[];
}

export interface Chained {
  name: string;
  from: string[];
  to: string[];
}

export interface PlannedFile {
  file: string;
  name: string;
  project: string | null;
  sure: boolean;
  folder: string | null;
  target: string[] | null;
  // The target's last folder does not exist yet; the run creates it.
  create: boolean;
  // The dictionary heading that placed the file, or null for a file no entry took.
  entry: string | null;
}

export interface DictionaryIssue {
  line: number;
  level: "error" | "warning";
  message: string;
}

export interface ItemTotals {
  size: number;
  onDisk: number;
  files: number;
  folders: number;
}

// Milliseconds since the epoch; size and onDisk only for files, whose totals are known at once.
export interface ItemFacts {
  name: string;
  folder: boolean;
  kind: string;
  location: string;
  size: number | null;
  onDisk: number | null;
  created: number | null;
  modified: number | null;
  accessed: number | null;
  readOnly: boolean;
  hidden: boolean;
}

// One support folder a node may hold: the one it has, or the name it would get; empty tells whether it can be quit.
export interface SupportRow {
  kind: string;
  name: string;
  exists: boolean;
  empty: boolean;
}

// The Outcomes folder a node's worknode defaults live in, each saved value as written (empty when unset).
export interface OutcomeDefaults {
  outcomes: string[];
  output: string;
  status: string;
  owner: string;
  template: string;
}

export interface Picks {
  reduce: boolean;
  sort: boolean;
  group: boolean;
  addDate: boolean;
  removeDate: boolean;
  removeSymbols: boolean;
}

export interface TreeSnapshot {
  root: string;
  nodes: TreeNode[];
  yamlChanged: boolean;
  copies: Copied[];
  warnings: string[];
}

export interface Copied {
  relParts: string[];
  original: string[];
}

export type CopyAnswer = "copy" | "independent";

export interface StorageInfo {
  root: string;
  appVersion: string;
  maxDepth: number;
  depthOptions: number[];
  // Set when the saved depth was unusable and the default is used instead.
  depthWarning: string | null;
}

// Touched children arrive with their subtree filled; the others with children null.
export interface LevelPatch {
  parent: string[];
  hasChildren: boolean;
  children: TreeNode[];
}

// A null tree means a local change: only the listed levels are patched.
// An id a folder already holds; node ids have an empty tracker, worknode numbers count per tracker ("PJ.").
export interface IdKey {
  tracker: string;
  number: number;
}

// The children a reorder by id may renumber, each list in id order, and the ids the other siblings hold.
export interface ReorderPlan {
  nodes: string[];
  worknodes: string[];
  taken: IdKey[];
  trackers: string[];
}

export interface IdTarget {
  name: string;
  newId: string;
  // Only for a worknode whose tracker changes.
  newTracker?: string;
}

export interface Mutation {
  tree: TreeSnapshot | null;
  levels: LevelPatch[];
  select: string[] | null;
  warnings: string[];
}

export interface BoardCard {
  relParts: string[];
  column: BoardColumn;
  order: number;
}

export interface PlacedFile {
  name: string;
  folder: string[];
  copied: boolean;
}

// A worknode already in the storage under the same id; `source` is what to send back to replace it.
export interface WorknodeConflict {
  name: string;
  source: string;
  existing: string[];
}

export interface DropOutcome {
  mutation: Mutation;
  placed: PlacedFile[];
  conflicts: WorknodeConflict[];
}

export interface DeletePreview {
  ghost: boolean;
  files: number;
  dirs: number;
}

export interface Moved {
  node: string;
  name: string;
  count: number;
  review: string;
}

export interface LooseFile {
  node: string;
  file: string;
}

export interface WaitingNode {
  node: string;
  relParts: string[];
  files: number;
}

export interface IntakeReport {
  moved: Moved[];
  loose: LooseFile[];
  waiting: WaitingNode[];
  warnings: string[];
}

export interface InvalidFolder {
  relParts: string[];
  review: string[][];
  worknode: boolean;
  tracker: string | null;
}

export interface SupportFolder {
  relParts: string[];
  name: string;
  notice: boolean;
  proposed: string;
}

export interface SupportCheck {
  needed: boolean;
  folders: SupportFolder[];
  root: boolean;
}

export interface ExtraReview {
  relParts: string[];
  name: string;
  keep: string;
}

export interface RootCheck {
  invalid: InvalidFolder[];
}

export type Resolution =
  | { kind: "review"; destination: string[] }
  | { kind: "node" }
  | { kind: "worknode"; tracker: string };

export interface ReviewIndexEvent {
  phase: "started" | "done";
  failures: string[];
}

export interface StorageChangedEvent {
  paths: number;
  folders: string[][];
  rescan: boolean;
  dictionary: boolean;
}

export type UiCall =
  | { action: "getState" }
  | { action: "reveal"; relParts: string[] }
  | { action: "openTab"; relParts: string[] | null }
  | { action: "split"; relParts: string[] | null }
  | { action: "setView"; view: string }
  | { action: "confirm"; title: string; items: string[]; seconds: number };

export interface UiRequest {
  id: number;
  call: UiCall;
}

export type FmError =
  | { kind: "storageNotFound"; start: string }
  | { kind: "invalidName"; name: string; reason: string }
  | { kind: "invalidPath"; relParts: string[]; reason: string }
  | { kind: "notFound"; relParts: string[] }
  | { kind: "stale"; relParts: string[] }
  | { kind: "alreadyExists"; name: string }
  | { kind: "locked"; relParts: string[]; nodeType: string }
  | { kind: "reserved"; relParts: string[] }
  | { kind: "ghostNotAllowed"; op: string }
  | { kind: "depthLimit" }
  | { kind: "notMovable" }
  | { kind: "needsConfirmation"; files: number; dirs: number }
  | { kind: "io"; op: string; path: string; message: string; osCode: number | null }
  | { kind: "rolledBack"; op: string; cause: string }
  | { kind: "inconsistent"; op: string; cause: string; rollbackErrors: string[] }
  | { kind: "internal"; message: string }
  | { kind: "clipboardEmpty" }
  | { kind: "newerDatabase"; path: string };

export function isFmError(e: unknown): e is FmError {
  return typeof e === "object" && e !== null && typeof (e as { kind?: unknown }).kind === "string";
}

export type AiClientId = "claudeDesktop" | "claudeCode" | "codex" | "opencode";

export interface AiClient {
  id: AiClientId;
  name: string;
  installed: boolean;
  connected: boolean;
  configPath: string | null;
  restartNeeded: boolean;
}

// The first part of every path inside the system Downloads folder; ':' cannot occur in a folder name, so it never matches a storage folder.
export const DOWNLOADS_ROOT = "::Downloads";

// The other native folders, in the order they close the root; they browse and change as Downloads does, without its automations.
export const SYSTEM_ROOTS = ["::Documents", "::Pictures", "::Music", "::Videos", "::Desktop"];

export const inDownloads = (parts: string[]): boolean => parts[0] === DOWNLOADS_ROOT;

// The native folder a path lies in, Downloads included; null for the storage.
export const systemRootOf = (parts: string[]): string | null => (parts[0] === DOWNLOADS_ROOT || SYSTEM_ROOTS.includes(parts[0] ?? "") ? (parts[0] ?? null) : null);

export const inSystem = (parts: string[]): boolean => systemRootOf(parts) !== null;

export const systemName = (parts: string[]): string => (systemRootOf(parts) ?? DOWNLOADS_ROOT).slice(2);

export interface TransferProgress {
  id: number;
  action: string;
  from: string;
  to: string;
  bytesTotal: number;
  bytesDone: number;
  filesTotal: number;
  filesDone: number;
  current: string;
  paused: boolean;
  finished: boolean;
}
