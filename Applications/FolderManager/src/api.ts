// Single IPC surface: the only module that calls invoke.
import { invoke } from "@tauri-apps/api/core";
import { isFmError } from "./types";
import type {
  DeletePreview,
  EditableType,
  FmError,
  MoveDirection,
  NewKind,
  Mutation,
  StorageInfo,
  TreeNode,
  TreeSnapshot,
  WkbStatus,
} from "./types";

async function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(cmd, args);
  } catch (e) {
    if (isFmError(e)) throw e;
    throw { kind: "internal", message: String(e) } satisfies FmError;
  }
}

export const getStorage = () => call<StorageInfo>("get_storage");
export const loadTree = () => call<TreeSnapshot>("load_tree");
export const refresh = () => call<TreeSnapshot>("refresh");
export const listChildren = (relParts: string[]) => call<TreeNode[]>("list_children", { relParts });
export const renameNode = (relParts: string[], newEditable: string) =>
  call<Mutation>("rename_node", { relParts, newEditable });
export const setNodeType = (relParts: string[], typeBase: EditableType, status: WkbStatus | null) =>
  call<Mutation>("set_node_type", { relParts, typeBase, status });
export const setNodeDetails = (relParts: string[], status: WkbStatus | null, owner: string | null) =>
  call<Mutation>("set_node_details", { relParts, status, owner });
export const addNode = (relParts: string[], kind: NewKind) => call<Mutation>("add_node", { relParts, kind });
export const moveNode = (relParts: string[], direction: MoveDirection) =>
  call<Mutation>("move_node", { relParts, direction });
export const inspectDelete = (relParts: string[]) => call<DeletePreview>("inspect_delete", { relParts });
export const deleteNode = (relParts: string[], expectGhost: boolean, confirmNonEmpty: boolean) =>
  call<Mutation>("delete_node", { relParts, expectGhost, confirmNonEmpty });
export const searchNodes = (query: string) => call<string[][]>("search_nodes", { query });
export interface Suggestion {
  value: string;
  label: string;
  hint: string;
}
export const suggestSearch = (key: string, prefix: string) => call<Suggestion[]>("suggest_search", { key, prefix });
export interface UpdateInfo {
  current: string;
  latest: string;
}
export const checkUpdate = () => call<UpdateInfo | null>("check_update");
export const startUpdate = () => call<void>("start_update");
export const getSkippedVersion = () => call<string>("get_skipped_version");
export const skipVersion = (version: string) => call<void>("skip_version", { version });

const path = (parts: string[]) => parts.join("/") || "root";

export function describeError(e: unknown): string {
  if (!isFmError(e)) return `Unexpected error: ${String(e)}`;
  switch (e.kind) {
    case "storageNotFound":
      return `No ANT storage found from ${e.start}`;
    case "invalidName":
      return `Invalid name "${e.name}": ${e.reason}`;
    case "invalidPath":
      return `Invalid path ${path(e.relParts)}: ${e.reason}`;
    case "notFound":
      return `${path(e.relParts)} no longer exists`;
    case "stale":
      return `${path(e.relParts)} changed on disk, the tree was reloaded`;
    case "alreadyExists":
      return `A folder named "${e.name}" already exists`;
    case "locked":
      return `${path(e.relParts)} is a locked ${e.nodeType} folder`;
    case "reserved":
      return `${path(e.relParts)} is reserved and cannot be changed`;
    case "ghostNotAllowed":
      return `Not allowed on a missing folder: ${e.op}`;
    case "depthLimit":
      return "Maximum depth reached";
    case "notMovable":
      return "This folder cannot be moved";
    case "needsConfirmation":
      return `Folder is not empty (${e.files} files, ${e.dirs} folders), confirm to delete`;
    case "io":
      return `${e.op} failed on ${e.path}: ${e.message}`;
    case "rolledBack":
      return `${e.op} failed and was rolled back: ${e.cause}`;
    case "inconsistent":
      return `${e.op} failed and the rollback was incomplete: ${e.cause}. Check ${e.rollbackErrors.length} affected paths`;
    case "internal":
      return `Internal error: ${e.message}`;
  }
}
