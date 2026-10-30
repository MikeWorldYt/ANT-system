// IPC contract shared with the Rust backend; mirror of the serde types.
export type TypeBase = "NODE" | "WKB" | "PORTF" | "REV" | "RES" | "TPL" | "CFG" | "UNKNOWN";
export type EditableType = "NODE" | "WKB" | "PORTF";
export type NewKind = "node" | "workboard";
export type WkbStatus = "in-progress" | "for-review" | "hold" | "done";
export type MoveDirection = "up" | "down";

export interface NodeType {
  raw: string;
  base: TypeBase;
  level: number | null;
}

export interface NodeInfo {
  fields: { key: string; value: string }[];
  error: string | null;
}

export interface NodeActions {
  rename: boolean;
  changeType: boolean;
  addNext: boolean;
  moveUp: boolean;
  moveDown: boolean;
  delete: boolean;
  copyPath: boolean;
  open: boolean;
  favorite: boolean;
  editDetails: boolean;
  editFlags: boolean;
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
  flags: string[];
  hasChildren: boolean;
  isGit: boolean;
  fixedPrefix: string;
  editableName: string;
  favorite: boolean;
  info: NodeInfo | null;
  children: TreeNode[] | null;
  absPath: string;
  actions: NodeActions;
}

export interface TreeSnapshot {
  root: string;
  nodes: TreeNode[];
  yamlChanged: boolean;
  copies: Copied[];
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
export interface Mutation {
  tree: TreeSnapshot | null;
  levels: LevelPatch[];
  select: string[] | null;
  warnings: string[];
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

export interface IntakeReport {
  moved: Moved[];
  loose: LooseFile[];
  warnings: string[];
}

export interface ReviewIndexEvent {
  phase: "started" | "done";
  failures: string[];
}

export interface StorageChangedEvent {
  paths: number;
  full: boolean;
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
  | { kind: "internal"; message: string };

export function isFmError(e: unknown): e is FmError {
  return typeof e === "object" && e !== null && typeof (e as { kind?: unknown }).kind === "string";
}
