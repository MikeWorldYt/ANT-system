// In-memory Storage for the tutorial: built from the bundled template and served in place of the backend, so the tour never touches disk.
import template from "./template.md?raw";
import type { Mutation, NodeActions, TreeNode, TreeSnapshot, WkbStatus } from "../ipc/types";

const ROOT = "Tutorial Storage";

interface Draft {
  name: string;
  kids: Draft[];
  project: boolean;
  status?: WkbStatus | null;
  stamp?: number;
  file?: boolean;
  size?: number;
}

const DAY = 86_400_000;

// Stand-in for the native file icon: a page with the extension on a colour band, drawn as an SVG data URL.
const EXT_COLORS: Record<string, string> = { PDF: "#d9534f", PNG: "#5b8def", XLSX: "#2e9d5b", DOCX: "#3b6fd1" };
export function fileIcon(name: string): string {
  const ext = (name.split(".").pop() ?? "").toUpperCase().slice(0, 4);
  const color = EXT_COLORS[ext] ?? "#8a8a8a";
  const svg = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 32 32"><path d="M7 2h13l7 7v21H7z" fill="#e8e8e8"/><path d="M20 2v7h7" fill="#bdbdbd"/><rect x="4" y="17" width="22" height="9" rx="2" fill="${color}"/><text x="15" y="24" font-family="Segoe UI, sans-serif" font-size="7" font-weight="700" fill="#fff" text-anchor="middle">${ext}</text></svg>`;
  return `data:image/svg+xml;charset=utf-8,${encodeURIComponent(svg)}`;
}
const SAMPLE_PROJECTS = ["Brand-Refresh", "Client-Portal", "Annual-Report", "Mobile-App", "Office-Move", "Podcast-Season", "Price-List", "Training-Plan", "Newsletter", "Photo-Archive", "Budget-2027", "Website-Audit", "Product-Launch", "Supplier-Review", "Team-Retreat"];
const SAMPLE_STATUSES: (WkbStatus | null)[] = ["in-progress", "hold", "for-review", "done", null];

const kebab = (text: string): string => text.trim().replace(/\s+/g, "-");
const pad = (n: number, width: number): string => String(n).padStart(width, "0");
// The fixed part of a name: the number of a node, or the tracker and number of a worknode.
const prefixOf = (name: string): string => /^(?:PJ\.\d+_|\d+\.)/.exec(name)?.[0] ?? "";

// Same reading as CreateNewStorage: `##` categories, `-` subcategories, indented `-` topics and `PJ:` projects; support folders are left out.
function parse(text: string): Draft[] {
  const cats: Draft[] = [];
  let projects = 0;
  for (const line of text.split(/\r?\n/)) {
    const heading = /^## (.+)$/.exec(line);
    if (heading?.[1]) {
      cats.push({ name: `${(cats.length + 1) * 100}.${kebab(heading[1]).toUpperCase()}`, kids: [], project: false });
      continue;
    }
    const item = /^( *)- (.+)$/.exec(line);
    const cat = cats[cats.length - 1];
    if (!item?.[2] || !cat) continue;
    const title = item[2].trim();
    if (/^(RES|TPL)$/.test(title)) continue;
    const parent = item[1] ? cat.kids[cat.kids.length - 1] : cat;
    if (!parent) continue;
    const pj = /^PJ:\s*(.+)$/.exec(title);
    if (pj?.[1]) parent.kids.push({ name: `PJ.${pad(++projects, 3)}_${kebab(pj[1])}`, kids: [], project: true });
    else parent.kids.push({ name: `${pad(parent.kids.filter((k) => !k.project).length + 1, 2)}.${kebab(title)}`, kids: [], project: false });
  }
  return cats;
}

function actions(level: number): NodeActions {
  const editable = level > 1;
  return {
    rename: editable,
    changeType: editable,
    addNext: true,
    addChild: true,
    addFolder: false,
    moveUp: editable,
    moveDown: editable,
    delete: editable,
    copyPath: false,
    open: false,
    favorite: false,
    important: false,
    editDetails: false,
    editFlags: false,
    hideLink: false,
    hide: false,
  };
}

function node(draft: Draft, parent: string[]): TreeNode {
  const relParts = [...parent, draft.name];
  const level = relParts.length;
  const prefix = draft.file ? "" : prefixOf(draft.name);
  const children = draft.kids.map((k) => node(k, relParts));
  return {
    name: draft.name,
    relParts,
    depth: parent.length,
    ghost: false,
    inYaml: true,
    nodeType: draft.file ? null : draft.project ? { raw: "WKB", base: "WKB", level: null } : { raw: `NODE-L${level}`, base: "NODE", level },
    status: draft.status ?? null,
    statusRaw: draft.status ?? null,
    locked: false,
    reserved: false,
    linked: false,
    linkPath: null,
    flags: [],
    hasChildren: children.length > 0,
    isGit: false,
    fixedPrefix: prefix,
    editableName: draft.name.slice(prefix.length),
    favorite: false,
    important: false,
    hidden: false,
    concealed: false,
    file: draft.file === true,
    info: null,
    children,
    absPath: [ROOT, ...relParts].join("\\"),
    actions: actions(level),
    stamps: { modified: draft.stamp ?? null, created: draft.stamp ? draft.stamp - 30 * DAY : null, size: draft.size ?? null },
  };
}

export interface DemoStorage {
  handle(cmd: string, args: Record<string, unknown> | undefined): Promise<unknown>;
  // Fills a folder with made-up worknodes in mixed statuses and dates, so sorting has something to show.
  seedProjects(parent: string[]): void;
  addFile(folder: string[], name: string, size: number): void;
}

// A level-1 sibling would be a new category, which the tutorial does not offer, so new nodes go next to a deeper one.
export function createDemoStorage(): DemoStorage {
  const cats = parse(template);
  const find = (parts: string[]): { list: Draft[]; at: number } | null => {
    let list = cats;
    for (const [i, part] of parts.entries()) {
      const at = list.findIndex((d) => d.name === part);
      const found = list[at];
      if (!found) return null;
      if (i === parts.length - 1) return { list, at };
      list = found.kids;
    }
    return null;
  };
  const snapshot = (): TreeSnapshot => ({ root: ROOT, nodes: cats.map((c) => node(c, [])), yamlChanged: false, copies: [], warnings: [] });
  const mutation = (select: string[] | null): Mutation => ({ tree: snapshot(), levels: [], select, warnings: [] });
  const parts = (args: Record<string, unknown> | undefined): string[] => (Array.isArray(args?.relParts) ? (args.relParts as string[]) : []);

  // Like the backend, a new worknode takes the next project number among its siblings.
  const nextProject = (list: Draft[]): number => Math.max(0, ...list.map((d) => Number(/^PJ\.(\d+)_/.exec(d.name)?.[1] ?? 0))) + 1;

  function addNext(relParts: string[], kind: unknown): Mutation {
    const spot = find(relParts);
    if (!spot || relParts.length < 2) throw { kind: "internal", message: "The tutorial adds nodes inside a category." };
    const used = spot.list.filter((d) => !d.project).length;
    const draft: Draft =
      kind === "worknode"
        ? { name: `PJ.${pad(nextProject(spot.list), 3)}_New-Worknode`, kids: [], project: true }
        : { name: `${pad(used + 1, 2)}.New-Node`, kids: [], project: false };
    spot.list.splice(spot.at + 1, 0, draft);
    return mutation([...relParts.slice(0, -1), draft.name]);
  }

  // The selection moves to the sibling above (or below), so the level stays open instead of falling back to the parent.
  function remove(relParts: string[]): Mutation {
    const spot = find(relParts);
    if (!spot || relParts.length < 2) return mutation(relParts.slice(0, -1));
    spot.list.splice(spot.at, 1);
    const next = spot.list[spot.at - 1] ?? spot.list[spot.at];
    return mutation(next ? [...relParts.slice(0, -1), next.name] : relParts.slice(0, -1));
  }

  // The root is listed too: a relist of the visible levels asks for it with an empty path.
  // A rename keeps the id or tracker and cleans the new title to Train-Case, as the backend does.
  function rename(relParts: string[], title: unknown): Mutation {
    const spot = find(relParts);
    const draft = spot?.list[spot.at];
    const clean = typeof title === "string" ? title.trim().replace(/[^\p{L}\p{N}]+/gu, "-").replace(/^-|-$/g, "") : "";
    if (!draft || !clean || relParts.length < 2) throw { kind: "internal", message: "Pick a name with letters or numbers." };
    draft.name = `${prefixOf(draft.name)}${clean.charAt(0).toUpperCase()}${clean.slice(1)}`;
    return mutation([...relParts.slice(0, -1), draft.name]);
  }

  function setStatus(relParts: string[], status: unknown): Mutation {
    const spot = find(relParts);
    const draft = spot?.list[spot.at];
    if (draft) draft.status = (status as WkbStatus | null) ?? null;
    return mutation(relParts);
  }

  const listed = (relParts: string[]): TreeNode[] => {
    if (!relParts.length) return cats.map((c) => node(c, []));
    const spot = find(relParts);
    const draft = spot?.list[spot.at];
    return draft ? draft.kids.map((k) => node(k, relParts)) : [];
  };

  function seedProjects(parent: string[]): void {
    const spot = find(parent);
    const folder = spot?.list[spot.at];
    if (!folder || folder.kids.some((d) => d.name.endsWith(`_${SAMPLE_PROJECTS[0]}`))) return;
    // A fixed spread of days keeps every run alike while names, dates and statuses all sort differently.
    const now = Date.now();
    for (const [i, title] of SAMPLE_PROJECTS.entries()) {
      const number = nextProject(folder.kids);
      folder.kids.push({ name: `PJ.${pad(number, 3)}_${title}`, kids: [], project: true, status: SAMPLE_STATUSES[i % SAMPLE_STATUSES.length], stamp: now - ((i * 7) % 15) * 3 * DAY });
    }
  }

  function addFile(folder: string[], name: string, size: number): void {
    const spot = find(folder);
    const target = spot?.list[spot.at];
    if (target && !target.kids.some((d) => d.name === name)) target.kids.push({ name, kids: [], project: false, file: true, size, stamp: Date.now() });
  }

  return {
    seedProjects,
    addFile,
    async handle(cmd, args) {
      switch (cmd) {
        case "load_tree":
        case "refresh":
          return snapshot();
        case "list_children":
          return listed(parts(args));
        case "add_node":
          return addNext(parts(args), args?.kind);
        case "rename_node":
          return rename(parts(args), args?.newEditable);
        case "set_node_details":
          return setStatus(parts(args), args?.status);
        case "inspect_delete":
          return { ghost: false, files: 0, dirs: 0 };
        case "delete_node":
          return remove(parts(args));
        case "check_extra_reviews":
          return [];
        case "check_support_upgrade":
          return { needed: false, folders: [], root: false };
        case "run_intake":
          return { moved: [], loose: [], waiting: [], warnings: [] };
        case "check_root":
          return { invalid: [] };
        case "list_favorites":
        case "list_review":
        case "list_downloads":
        case "search_nodes":
          return [];
        case "file_image": {
          const name = parts(args).at(-1) ?? "";
          return name.includes(".") ? fileIcon(name) : null;
        }
        case "download_image":
          return null;
        default:
          throw { kind: "internal", message: "Not available in the tutorial." };
      }
    },
  };
}
