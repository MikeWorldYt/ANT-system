// Single IPC surface: the only module that calls invoke.
import { invoke } from "@tauri-apps/api/core";
import { inSystem, isFmError } from "./types";
import { hiddenItemsShown } from "../store/view-prefs";
import type {
  AiClient,
  ApplyOutcome,
  BoardCard,
  CopyAnswer,
  HiddenChoice,
  DeletePreview,
  DropOutcome,
  EditableType,
  ExtraReview,
  FmError,
  IntakeReport,
  IdTarget,
  MoveDirection,
  NewKind,
  Mutation,
  ReorderPlan,
  Resolution,
  RootCheck,
  SupportCheck,
  StorageInfo,
  TreeNode,
  TreeSnapshot,
  WkbStatus,
  Picks,
  PlannedFile,
  DictionaryIssue,
  ItemFacts,
  ItemTotals,
  SupportRow,
} from "./types";

// The tutorial serves its in-memory Storage here, so while it runs no command reaches the backend.
type DemoHandler = (cmd: string, args?: Record<string, unknown>) => Promise<unknown>;
let demo: DemoHandler | null = null;
export function setDemoHandler(handler: DemoHandler | null): void {
  demo = handler;
}

async function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (demo) return (await demo(cmd, args)) as T;
  try {
    return await invoke<T>(cmd, args);
  } catch (e) {
    if (isFmError(e)) throw e;
    throw { kind: "internal", message: String(e) } satisfies FmError;
  }
}

export const getStorage = () => call<StorageInfo>("get_storage");
export const loadTree = () => call<TreeSnapshot>("load_tree");
export const refresh = (full: boolean) => call<TreeSnapshot>("refresh", { full });
export const runIntake = () => call<IntakeReport>("run_intake");
export const acknowledgeIntake = (relParts: string[]) => call<void>("acknowledge_intake", { relParts });
export const checkRoot = () => call<RootCheck>("check_root");
export const checkSupportUpgrade = () => call<SupportCheck>("check_support_upgrade");
export const applySupportUpgrade = (relParts: string[]) => call<Mutation>("apply_support_upgrade", { relParts });
export const finishSupportUpgrade = (stamp: string) => call<string[]>("finish_support_upgrade", { stamp });
export const checkExtraReviews = () => call<ExtraReview[]>("check_extra_reviews");
export const mergeExtraReview = (relParts: string[]) => call<Mutation>("merge_extra_review", { relParts });
export const previewInvalid = (rows: { relParts: string[]; resolution: Resolution }[]) => call<string[]>("preview_invalid", { rows });
export const resolveInvalid = (relParts: string[], resolution: Resolution) =>
  call<Mutation>("resolve_invalid", { relParts, resolution });
// Native folder paths start with a marker; the backend takes it apart as the folder to use and the names below it.
const native = (relParts: string[]) => ({ root: relParts[0], relParts: relParts.slice(1) });
// One call per native folder, as each call names a single one.
const byRoot = (targets: string[][]): string[][][] => [...new Set(targets.map((t) => t[0]))].map((root) => targets.filter((t) => t[0] === root));
// Hidden, system and dot files reach the views only when Hidden items is on.
export const listChildren = async (relParts: string[]): Promise<TreeNode[]> => {
  const nodes = await (inSystem(relParts) ? call<TreeNode[]>("list_downloads", native(relParts)) : call<TreeNode[]>("list_children", { relParts }));
  return hiddenItemsShown() ? nodes : nodes.filter((n) => !n.concealed);
};
// A reorder renames concealed folders too, so they are listed whatever Hidden items says.
export const listAllChildren = (relParts: string[]) => call<TreeNode[]>("list_children", { relParts });
// Answers arrive as folder-size events, one per folder, while the call runs.
export const folderSizes = (targets: string[][]) => call<void>("folder_sizes", { targets });
export const renameNode = (relParts: string[], newEditable: string) =>
  call<Mutation>("rename_node", { relParts, newEditable });
export const setNodeType = (relParts: string[], typeBase: EditableType, status: WkbStatus | null) =>
  call<Mutation>("set_node_type", { relParts, typeBase, status });
export const openNode = (relParts: string[]) =>
  inSystem(relParts) ? call<void>("open_download", native(relParts)) : call<void>("open_node", { relParts });
export const revealNode = (relParts: string[]) =>
  inSystem(relParts) ? call<void>("reveal_download", native(relParts)) : call<void>("reveal_node", { relParts });
export const renameDownload = (relParts: string[], newName: string) => call<void>("rename_download", { ...native(relParts), newName });
export const createDownloadFolder = (relParts: string[], name: string) => call<string>("create_download_folder", { ...native(relParts), name });
export const pasteIntoDownloads = (relParts: string[]) => call<string[]>("paste_into_downloads", native(relParts));
export const pinDictionaryFile = (folder: string[], entry: string | null, file: string) => call<void>("pin_dictionary_file", { folder, entry, file });
export const dropIntoDownloads = (relParts: string[], paths: string[]) => call<string[]>("drop_into_downloads", { ...native(relParts), paths });
export const deleteDownloads = async (targets: string[][]): Promise<void> => {
  for (const group of byRoot(targets)) await call<void>("delete_downloads", { root: group[0]?.[0], targets: group.map((t) => t.slice(1)) });
};
export const setFavorite = (relParts: string[], on: boolean) => call<Mutation>("set_favorite", { relParts, on });
export const setImportant = (relParts: string[], on: boolean) => call<Mutation>("set_important", { relParts, on });
export const fileImage = (relParts: string[], large: boolean) =>
  inSystem(relParts) ? call<string | null>("download_image", { ...native(relParts), large }) : call<string | null>("file_image", { relParts, large });
export const previewImage = (relParts: string[]) =>
  inSystem(relParts) ? call<string | null>("download_image", { ...native(relParts), large: true, preview: true }) : call<string | null>("file_image", { relParts, large: true, preview: true });
export const setLinkHidden =(relParts: string[], on: boolean) => call<Mutation>("set_link_hidden", { relParts, on });
export const setUserFlag = (relParts: string[], on: boolean, flag: "data" | "autom" = "data") => call<Mutation>("set_user_flag", { relParts, on, flag });
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
export interface KnownStorage {
  path: string;
  name: string;
  exists: boolean;
}
export const knownStorages = () => call<KnownStorage[]>("known_storages");
export const forgetStorage = (path: string) => call<void>("forget_storage", { path });
export const createStorage = (parent: string, name: string) => call<string>("create_storage", { parent, name });
export const renameListedStorage = (path: string, name: string) => call<string>("rename_listed_storage", { path, name });
export const moveListedStorage = (path: string, destination: string) => call<string>("move_listed_storage", { path, destination });
export const revealStorage = (path: string) => call<void>("reveal_storage", { path });
export interface StorageLocation {
  label: string;
  path: string;
}
export const storageLocations = () => call<StorageLocation[]>("storage_locations");
export interface RenameStatus {
  old: string;
  new: string;
  revertible: boolean;
}
export const renameStorage = (newName: string, stamp: string) => call<string>("rename_storage", { newName, stamp });
export const revertStorageRename = (stamp: string) => call<string>("revert_storage_rename", { stamp });
export const storageRenameStatus = () => call<RenameStatus | null>("storage_rename_status");
export const syncSchedule = (on: boolean) => call<void>("sync_schedule", { on });
export const scheduledTaskProblems = () => call<string[]>("scheduled_task_problems");
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
export const setNodeHidden = (relParts: string[], on: boolean) => call<Mutation>("set_node_hidden", { relParts, on });
export const setHiddenNodes = (parent: string[], hidden: string[]) => call<Mutation>("set_hidden_nodes", { parent, hidden });
export const listHiddenChoices = (parent: string[]) => call<HiddenChoice[]>("list_hidden_choices", { parent });
export const createTemplate = (identifier: string, name: string, description: string | null) => call<string[]>("create_template", { identifier, name, description });
export const setOutcomeDefault = (relParts: string[], key: string, value: string | null) => call<Mutation>("set_outcome_default", { relParts, key, value });
export const setNodeOutput = (relParts: string[], output: string | null) => call<Mutation>("set_node_output", { relParts, output });
export const deliverOutput = (relParts: string[], date: string, all: boolean) => call<Mutation>("deliver_output", { relParts, date, all });
export const restoreOutput = (relParts: string[]) => call<Mutation>("restore_output", { relParts });
export const addNode = (relParts: string[], kind: NewKind) => call<Mutation>("add_node", { relParts, kind });
export const addChild = (relParts: string[], kind: NewKind) => call<Mutation>("add_child", { relParts, kind });
export const moveNode = (relParts: string[], direction: MoveDirection) =>
  call<Mutation>("move_node", { relParts, direction });
export const reorderPlan = (parentRelParts: string[]) => call<ReorderPlan>("reorder_plan", { parentRelParts });
export const reorderIds = (parentRelParts: string[], targets: IdTarget[]) => call<Mutation>("reorder_ids", { parentRelParts, targets });
export const dropFiles = (target: string[], paths: string[], replace: string[] = []) => call<DropOutcome>("drop_files", { target, paths, replace });
export const pasteFiles = (target: string[]) => call<DropOutcome>("paste_files", { target });
export const moveFilesIn = (sources: string[][], target: string[], copy: boolean) =>
  call<DropOutcome>("move_files_in", { sources, target, copy });
export const moveFolder = (source: string[], destination: string[], copy: boolean) =>
  call<Mutation>("move_folder", { source, destination, copy });
// The drop effect Explorer chose: 2 when the items were moved out of the storage.
export const startDragOut = (targets: string[][]) => call<number>("start_drag_out", { targets });
export interface ShortcutCreated {
  name: string;
  folder: string;
}
export const createShortcutIn = (target: string[], folder: string) =>
  call<ShortcutCreated>("create_shortcut_in", { target, folder });
export const copyItems = (targets: string[][], cut: boolean) =>
  targets.every(inSystem) && byRoot(targets).length <= 1 ? call<void>("copy_downloads", { root: targets[0]?.[0], targets: targets.map((t) => t.slice(1)), cut }) : call<void>("copy_items", { targets, cut });
export const createFileShortcutIn = (target: string[], folder: string) =>
  call<ShortcutCreated>("create_file_shortcut_in", { target, folder });
export const setReviewAutomation = (relParts: string[], automation: keyof Picks, on: boolean) => call<Mutation>("set_review_automation", { relParts, automation, on });
export const planAutomations = (folder: string[], only: string[], picks: Picks) => call<PlannedFile[]>("plan_automations", { folder, only, ...picks });
export const openDictionaryWindow = (folder: string[]) => call<void>("open_dictionary_window", { folder });
export const openPropertiesWindow = (relParts: string[], icon: string) => call<void>("open_properties_window", { relParts, icon });
export const itemProperties = (relParts: string[]) => call<ItemFacts>("item_properties", { relParts });
// Null when the window closed before the walk ended; partial totals arrive as events meanwhile.
export const itemTotals = (relParts: string[]) => call<ItemTotals | null>("item_totals", { relParts });
// Null when the item cannot hold support folders.
export const supportNodes = (relParts: string[]) => call<SupportRow[] | null>("support_nodes", { relParts });
export const addSupportNode = (relParts: string[], kind: string) => call<Mutation>("add_support_node", { relParts, kind });
export const removeSupportNode = (relParts: string[], name: string) => call<Mutation>("remove_support_node", { relParts, name });
export const readDictionary = (folder: string[]) => call<string>("read_dictionary", { folder });
export const checkDictionary = (text: string) => call<DictionaryIssue[]>("check_dictionary", { text });
export const saveDictionary = (folder: string[], text: string) => call<DictionaryIssue[]>("save_dictionary", { folder, text });
export const buildDictionaryAi = (folder: string[], text: string) => call<string>("build_dictionary_ai", { folder, text });
export const applyAutomations = (folder: string[], rows: PlannedFile[]) => call<ApplyOutcome>("apply_automations", { folder, rows });
export const renameFile = (relParts: string[], newName: string) => call<Mutation>("rename_file", { relParts, newName });
export const deleteFile = (relParts: string[]) => call<Mutation>("delete_file", { relParts });
export const inspectDelete =(relParts: string[]) => call<DeletePreview>("inspect_delete", { relParts });
export const deleteNode = (relParts: string[], expectGhost: boolean, confirmNonEmpty: boolean) =>
  call<Mutation>("delete_node", { relParts, expectGhost, confirmNonEmpty });
export const answerUiRequest = (id: number, value: unknown, error: string | null) => call<void>("answer_ui_request", { id, value, error });
export const searchNodes = (query: string) => call<string[][]>("search_nodes", { query });
export const listFavorites = () => call<string[][]>("list_favorites");
export const systemFolders = () => call<string[]>("system_folders");
export const listReview = () => call<string[][]>("list_review");
export const listWorknodes = () => call<string[][]>("list_worknodes");
export const canUndoProjectSort = () => call<boolean>("can_undo_project_sort");
export const undoProjectSort = () => call<Mutation>("undo_project_sort");
export const setBoard = (cards: BoardCard[]) => call<Mutation>("set_board", { cards });
export const searchDisk = (query: string) => call<string[][]>("search_disk", { query });
export interface FileHit {
  folder: string[];
  name: string;
}
export const searchFiles = (query: string) => call<FileHit[]>("search_files", { query });
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
export const checkUpdate = (manual: boolean, beta: boolean) => call<UpdateInfo | null>("check_update", { manual, beta });
export const startUpdate = (beta: boolean) => call<void>("start_update", { beta });
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
  needsConfirmation: boolean;
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

export const aiClients = () => call<AiClient[]>("ai_clients");
export const aiConnect = (id: AiClient["id"]) => call<AiClient>("ai_connect", { id });
export const aiDisconnect = (id: AiClient["id"]) => call<AiClient>("ai_disconnect", { id });
export const aiRestartClient = (id: AiClient["id"]) => call<AiClient>("ai_restart_client", { id });
export const aiConfigSnippet = () => call<string>("ai_config_snippet");

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
    case "newerDatabase":
      return `${e.path} was written by a newer Folder Manager. Update the app to open this Storage`;
  }
}

export const cancelTransfer = () => call<void>("cancel_transfer");
export const pauseTransfer = (paused: boolean) => call<void>("pause_transfer", { paused });
export const beginDelete = (targets: string[][]) => call<void>("begin_delete", { targets });
export const endDelete = () => call<void>("end_delete");
export const pdfPage = (relParts: string[], page: number) => call<{ url: string; count: number } | null>("pdf_page", { relParts, page });
export const undoDelete = () => call<{ relParts: string[]; left: number } | null>("undo_delete");
