// Single IPC surface: the only module that calls invoke.
import { invoke } from "@tauri-apps/api/core";
import { isFmError } from "./types";
import type {
  CopyAnswer,
  DeletePreview,
  DropOutcome,
  EditableType,
  FmError,
  IntakeReport,
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
export const runIntake = () => call<IntakeReport>("run_intake");
export const listChildren = (relParts: string[]) => call<TreeNode[]>("list_children", { relParts });
export const renameNode = (relParts: string[], newEditable: string) =>
  call<Mutation>("rename_node", { relParts, newEditable });
export const setNodeType = (relParts: string[], typeBase: EditableType, status: WkbStatus | null) =>
  call<Mutation>("set_node_type", { relParts, typeBase, status });
export const openNode = (relParts: string[]) => call<void>("open_node", { relParts });
export const setFavorite = (relParts: string[], on: boolean) => call<Mutation>("set_favorite", { relParts, on });
export const setUserFlag = (relParts: string[], on: boolean) => call<Mutation>("set_user_flag", { relParts, on });
export const answerCopy = (relParts: string[], answer: CopyAnswer) => call<void>("answer_copy", { relParts, answer });
export const getSettings = () => call<Record<string, string>>("get_settings");
export const setSetting = (key: string, value: string) => call<void>("set_setting", { key, value });
export const openLink = (link: "issues" | "mail" | "changelog" | "docs") => call<void>("open_link", { link });
export interface BackupEntry {
  name: string;
  path: string;
}
export type StorageHealth =
  | { state: "ok"; path: string }
  | { state: "noDatabase"; path: string; backup: { name: string; label: string } | null }
  | { state: "notFound"; path: string };
export const storageHealth = () => call<StorageHealth>("storage_health");
export const useLatestBackup = (stamp: string) => call<void>("use_latest_backup", { stamp });
export const prepareScan = (stamp: string) => call<void>("prepare_scan", { stamp });
export const changeStoragePath = (path: string) => call<string>("change_storage_path", { path });
export interface RenameStatus {
  old: string;
  new: string;
  revertible: boolean;
}
export const renameStorage = (newName: string, stamp: string) => call<string>("rename_storage", { newName, stamp });
export const revertStorageRename = (stamp: string) => call<string>("revert_storage_rename", { stamp });
export const storageRenameStatus = () => call<RenameStatus | null>("storage_rename_status");
export const syncSchedule = (on: boolean) => call<void>("sync_schedule", { on });
export const backupNow = (stamp: string) => call<string>("backup_now", { stamp });
export const backupIfDue = (stamp: string) => call<string | null>("backup_if_due", { stamp });
export const listBackups = () => call<BackupEntry[]>("list_backups");
export const restoreBackup = (name: string, stamp: string) => call<void>("restore_backup", { name, stamp });
export const backupSchedule = (on: boolean) => call<void>("backup_schedule", { on });
export const resetSettings = () => call<void>("reset_settings");
export const startDebugLog = (stamp: string) => call<string>("start_debug_log", { stamp });
export const stopDebugLog = () => call<void>("stop_debug_log");
export const logDebug = (level: "info" | "warning" | "error", message: string) => call<void>("log_debug", { level, message });
export const openLogsFolder = () => call<void>("open_logs_folder");
export const openDevtools = () => call<void>("open_devtools");
export const setNodeDetails = (relParts: string[], status: WkbStatus | null, owner: string | null) =>
  call<Mutation>("set_node_details", { relParts, status, owner });
export const addNode = (relParts: string[], kind: NewKind) => call<Mutation>("add_node", { relParts, kind });
export const moveNode = (relParts: string[], direction: MoveDirection) =>
  call<Mutation>("move_node", { relParts, direction });
export const dropFiles = (target: string[], paths: string[]) => call<DropOutcome>("drop_files", { target, paths });
export const pasteFiles = (target: string[]) => call<DropOutcome>("paste_files", { target });
export const moveFolder = (source: string[], destination: string[], copy: boolean) =>
  call<Mutation>("move_folder", { source, destination, copy });
export const startDragOut =(target: string[]) => call<void>("start_drag_out", { target });
export const inspectDelete =(relParts: string[]) => call<DeletePreview>("inspect_delete", { relParts });
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
export const checkUpdate = (manual: boolean) => call<UpdateInfo | null>("check_update", { manual });
export const startUpdate = () => call<void>("start_update");
export const skipVersion = (version: string) => call<void>("skip_version", { version });
export type ExtensionPermission = "tree:read" | "menu:context" | "panel:own" | "details:field";
export interface ExtensionInfo {
  id: string;
  name: string;
  version: string;
  description: string;
  author: string;
  permissions: string[];
  trust: "validated" | "unvalidated";
  enabled: boolean;
  status: "ok" | "rejected";
  reason: string | null;
}
export interface RegistryEntry {
  id: string;
  name: string;
  version: string;
  description: string;
  author: string;
  permissions: string[];
  installed: boolean;
}
export interface ExtensionPreview {
  token: string;
  id: string;
  name: string;
  version: string;
  description: string;
  author: string;
  permissions: string[];
  size: number;
  alreadyInstalled: boolean;
}
export const listExtensions = () => call<ExtensionInfo[]>("list_extensions");
export const setExtensionEnabled = (id: string, enabled: boolean) => call<void>("set_extension_enabled", { id, enabled });
export const readExtensionEntry = (id: string) => call<string>("read_extension_entry", { id });
export const removeExtension = (id: string) => call<void>("remove_extension", { id });
export const listRegistry = () => call<RegistryEntry[]>("list_registry");
export const installRegistryExtension = (id: string) => call<ExtensionInfo>("install_registry_extension", { id });
export const inspectExtensionRepo = (url: string) => call<ExtensionPreview>("inspect_extension_repo", { url });
export const installInspectedExtension = (token: string) => call<ExtensionInfo>("install_inspected_extension", { token });

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
    case "clipboardEmpty":
      return "The clipboard has no files to paste";
  }
}
