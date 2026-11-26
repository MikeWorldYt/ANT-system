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
  plain?: boolean;
  outcomes?: boolean;
  output?: string;
  support?: "REV" | "REC" | "RES" | "SS";
  data?: boolean;
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
const prefixOf = (name: string): string => /^(?:PJ\.\d+_|\d+\.|(?:REV|REC|RES|SS)\.)/.exec(name)?.[0] ?? "";

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

// Every project of a WORK, STUDY or HOBBIES area gets the same made-up skeleton of that trade; a path ending in "/" is a plain folder.
const SKELETONS: Record<string, string[]> = {
  Development: ["dist/", "dist/index.html", "dist/assets.js", "public/", "public/favicon.ico", "src/", "src/components/", "src/App.tsx", "src/main.tsx", "package.json", "README.md"],
  Design: ["exports/", "exports/logo.png", "exports/banner.png", "sources/", "sources/logo.ai", "sources/layout.fig", "fonts/", "brief.pdf"],
  "Video-edition": ["footage/", "footage/A001_C001.mp4", "footage/A001_C002.mp4", "audio/", "audio/music.wav", "graphics/", "graphics/title.psd", "project.prproj", "render.mp4"],
  Architecture: ["plans/", "plans/floor-plan.dwg", "plans/elevations.dwg", "renders/", "renders/exterior.jpg", "model.skp", "specs.pdf"],
  "Computer-Science": ["labs/", "labs/lab1.py", "labs/lab2.py", "notes/", "notes/week1.md", "assignments/", "assignments/hw1.pdf", "syllabus.pdf"],
  "Data-Analysis": ["data/", "data/raw.csv", "data/clean.csv", "notebooks/", "notebooks/analysis.ipynb", "charts/", "charts/trend.png", "report.pdf"],
  "Digital-Arts": ["sketches/", "sketches/draft-01.png", "references/", "references/palette.jpg", "final.psd", "export.png"],
  Business: ["research/", "research/market.pdf", "slides.pptx", "plan.docx", "budget.xlsx"],
  Medicine: ["papers/", "papers/study-2026.pdf", "images/", "images/x-ray.png", "patient-notes.docx", "dosage.xlsx"],
  Languages: ["vocabulary/", "vocabulary/unit1.xlsx", "audio/", "audio/lesson1.mp3", "exercises.docx", "notes.pdf"],
  Math: ["notes/", "notes/lecture1.pdf", "exercises/", "exercises/set1.pdf", "formulas.pdf", "homework.tex"],
  Music: ["stems/", "stems/drums.wav", "stems/bass.wav", "mix.flp", "master.mp3", "lyrics.docx"],
  Reading: ["ebooks/", "ebooks/novel.epub", "notes/", "notes/chapter-summary.docx", "reading-list.xlsx", "review.pdf"],
  Photography: ["raw/", "raw/DSC_0101.jpg", "raw/DSC_0102.jpg", "edits/", "edits/DSC_0101-edit.jpg", "preset.xmp", "contact-sheet.pdf"],
  Cooking: ["recipes/", "recipes/paella.pdf", "photos/", "photos/plated.jpg", "shopping-list.xlsx", "menu.docx"],
  Fitness: ["logs/", "logs/week1.xlsx", "photos/", "photos/progress.jpg", "plan.pdf", "meal-prep.docx"],
};
// The first game looks like a real fan's folder, so the tour has a full one to open.
const ENDFIELD = ["Mods/", "Mods/reshade-preset.ini", "Mods/ui-tweaks.zip", "Arts/", "Arts/endministrator-fanart.png", "Arts/talos-ii-wallpaper.jpg", "Arts/perlica-sketch.psd", "Builds/", "Builds/team-comps.xlsx", "Builds/gear-planner.xlsx", "Clips/", "Clips/boss-clear.mp4", "base-layout.png", "gacha-log.csv"];
// PERSONAL keeps documents, not projects, so its topics hold the files themselves.
const PERSONAL_FILES: Record<string, string[]> = {
  Identity: ["passport.pdf", "id-card.jpg"], Certificates: ["birth-certificate.pdf", "diploma.pdf"], Legal: ["rental-contract.pdf"], Insurance: ["home-policy.pdf", "car-policy.pdf"],
  Bank: ["statement-2026-10.pdf", "statement-2026-11.pdf"], Taxes: ["tax-return-2025.pdf"], Invoices: ["invoice-0012.pdf", "invoice-0013.pdf"], Budget: ["budget-2026.xlsx"],
  Records: ["blood-test.pdf", "check-up-2026.pdf"], Prescriptions: ["prescription.pdf"], Vaccines: ["vaccination-card.jpg"], Appointments: ["dentist.pdf"],
  Bills: ["electricity-2026-10.pdf", "water-2026-10.pdf"], Rent: ["receipt-2026-11.pdf"], Appliances: ["fridge-manual.pdf"], Vehicle: ["registration.pdf"],
  Photos: ["IMG_0001.jpg", "IMG_0002.jpg"], Videos: ["VID_0001.mp4"], Family: ["birthday.jpg"], Events: ["graduation.jpg"],
  Tickets: ["flight-MAD-NRT.pdf"], Bookings: ["hotel.pdf"], Itineraries: ["trip-plan.xlsx"], Visas: ["visa.pdf"],
};
// The tour shows loose files waiting in a review folder next to the projects.
const LOOSE = ["scan_0042.pdf", "IMG_20261103.png", "invoice-final-v2.pdf", "notes (1).docx", "budget_copy.xlsx"];

function skeleton(paths: string[]): Draft[] {
  const root: Draft[] = [];
  for (const [i, path] of paths.entries()) {
    const folder = path.endsWith("/");
    const parts = path.replace(/\/$/, "").split("/");
    const name = parts.pop() ?? "";
    const parent = parts.reduce((list, part) => list.find((d) => d.name === part)?.kids ?? list, root);
    parent.push(folder ? { name, kids: [], project: false, plain: true } : { name, kids: [], project: false, file: true, size: 20_000 * (i + 2), stamp: Date.now() - i * DAY });
  }
  return root;
}

function addSamples(cats: Draft[]): void {
  for (const area of cats.filter((c) => c.name === "200.WORK" || c.name === "300.STUDY" || c.name === "400.HOBBIES").flatMap((c) => c.kids)) {
    const paths = SKELETONS[area.name.slice(prefixOf(area.name).length)];
    if (paths) for (const pj of area.kids.filter((k) => k.project)) pj.kids.push(...skeleton(paths));
    // A list of projects delivers its finished work to the Outcomes folder beside them.
    if (area.kids.some((k) => k.project)) area.kids.unshift({ name: "00.Outcomes", kids: [], project: false, outcomes: true });
  }
  // Gaming holds one node per game, each with its mods and fan art.
  for (const game of cats.find((c) => c.name === "400.HOBBIES")?.kids.find((a) => a.name.endsWith(".Gaming"))?.kids.filter((k) => !k.support) ?? []) {
    game.kids.push(...skeleton(game.name.endsWith("Arknights-Endfield") ? ENDFIELD : ["Mods/", "Mods/texture-pack.zip", "Mods/config.ini", "Arts/", "Arts/wallpaper.png", "Arts/fan-art.jpg"]));
  }
  for (const topic of cats.find((c) => c.name === "100.PERSONAL")?.kids.flatMap((a) => a.kids) ?? []) {
    const names = PERSONAL_FILES[topic.name.slice(prefixOf(topic.name).length)];
    if (names) topic.kids.push(...skeleton(names));
  }
  const first = cats.find((c) => c.name === "200.WORK")?.kids[0];
  const file = (name: string, i: number): Draft => ({ name, kids: [], project: false, file: true, size: 40_000 * (i + 3), stamp: Date.now() - i * DAY });
  first?.kids.find((k) => k.support === "REV")?.kids.push(...LOOSE.map(file));
}

// As CreateNewStorage does: each category gets its Recordings, Resources, Review and Screenshots, and each subcategory and topic its Review.
function addSupports(list: Draft[], level: number): void {
  for (const d of list.filter((k) => !k.project && !k.file && !k.support && !k.plain)) {
    addSupports(d.kids, level + 1);
    const title = d.name.slice(prefixOf(d.name).length);
    const support = (kind: "REV" | "REC" | "RES" | "SS", name = title): Draft => ({ name: `${kind}.${name}`, kids: [], project: false, support: kind });
    if (level === 1) d.kids.push(support("REC"), support("SS"), support("RES", "Resources"), support("REV"));
    else if (level < 4) d.kids.push(support("REV"));
  }
}

// Only the user's own project is renamed, from its details, so an inline rename never catches the user off guard.
function actions(level: number, project: boolean): NodeActions {
  const editable = level > 1;
  return {
    editFlags: editable,
    rename: editable && project,
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
    hideLink: false,
    hide: false,
  };
}

function node(draft: Draft, parent: string[]): TreeNode {
  const relParts = [...parent, draft.name];
  const level = relParts.length;
  const prefix = draft.file || draft.plain ? "" : prefixOf(draft.name);
  const children = draft.kids.map((k) => node(k, relParts));
  return {
    name: draft.name,
    relParts,
    depth: parent.length,
    ghost: false,
    inYaml: true,
    nodeType: draft.file || draft.plain ? null : draft.outcomes ? { raw: "OUT", base: "OUT", level: null } : draft.project ? { raw: "WKB", base: "WKB", level: null } : draft.support ? { raw: draft.support, base: draft.support, level: null } : { raw: `NODE-L${level}`, base: "NODE", level },
    status: draft.status ?? null,
    statusRaw: draft.status ?? null,
    locked: false,
    reserved: false,
    linked: false,
    linkPath: null,
    // As the backend does, a node made on level 3, the edge of the default index, takes files.
    flags: draft.data || (level === 3 && !draft.project && !draft.support && !draft.file && !draft.plain) ? ["data"] : [],
    hasChildren: children.length > 0,
    isGit: false,
    fixedPrefix: prefix,
    editableName: draft.name.slice(prefix.length),
    favorite: false,
    important: false,
    hidden: false,
    concealed: false,
    file: draft.file === true,
    info: draft.output ? { fields: [{ key: "Output", value: draft.output }], error: null } : null,
    children,
    absPath: [ROOT, ...relParts].join("\\"),
    actions: actions(level, draft.project),
    stamps: { modified: draft.stamp ?? null, created: draft.stamp ? draft.stamp - 30 * DAY : null, size: draft.size ?? null },
  };
}

export interface DemoStorage {
  handle(cmd: string, args: Record<string, unknown> | undefined): Promise<unknown>;
  // Fills a folder with made-up worknodes in mixed statuses and dates, so sorting has something to show.
  seedProjects(parent: string[]): void;
  addFile(folder: string[], name: string, size: number): void;
  fill(folder: string[], paths: string[]): void;
}

// A level-1 sibling would be a new category, which the tutorial does not offer, so new nodes go next to a deeper one.
export function createDemoStorage(): DemoStorage {
  const cats = parse(template);
  addSupports(cats, 1);
  addSamples(cats);
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
    const used = spot.list.filter((d) => !d.project && !d.support && !d.file && !d.outcomes).length;
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

  function addChild(relParts: string[], kind: unknown): Mutation {
    const spot = find(relParts);
    const folder = spot?.list[spot.at];
    if (!folder || relParts.length < 2) throw { kind: "internal", message: "The tutorial adds nodes inside a category." };
    const used = folder.kids.filter((d) => !d.project && !d.support && !d.file && !d.outcomes).length;
    const draft: Draft = kind === "worknode" ? { name: `PJ.${pad(nextProject(folder.kids), 3)}_New-Worknode`, kids: [], project: true } : { name: `${pad(used + 1, 2)}.New-Node`, kids: [], project: false };
    const tail = folder.kids.findIndex((d) => d.support);
    folder.kids.splice(tail < 0 ? folder.kids.length : tail, 0, draft);
    return mutation([...relParts, draft.name]);
  }

  // A path inside a worknode, as the output and the picker give it.
  const inside = (root: Draft, path: string): { list: Draft[]; at: number } | null => {
    let list = root.kids;
    const parts = path.split("/");
    for (const [i, part] of parts.entries()) {
      const at = list.findIndex((d) => d.name === part);
      if (at < 0) return null;
      if (i === parts.length - 1) return { list, at };
      list = list[at]?.kids ?? [];
    }
    return null;
  };

  function setOutput(relParts: string[], output: unknown): Mutation {
    const spot = find(relParts);
    const draft = spot?.list[spot.at];
    if (draft) draft.output = typeof output === "string" ? output : undefined;
    return mutation(relParts);
  }

  // As the backend does: the output moves into `(date)_Title` inside the Outcomes folder beside the worknode, and a link stays behind.
  function deliver(relParts: string[], date: unknown, all: unknown): Mutation {
    const spot = find(relParts);
    const worknode = spot?.list[spot.at];
    const outcomes = spot?.list.find((d) => d.outcomes);
    if (!worknode || !outcomes) throw { kind: "internal", message: "No Outcomes folder beside this worknode." };
    const name = `(${String(date)})_${worknode.name.slice(prefixOf(worknode.name).length)}`;
    const out = worknode.output ? inside(worknode, worknode.output) : null;
    const picked = out?.list[out.at];
    let moved: Draft[];
    let linkHome = worknode.kids;
    if (all === true || !picked || !out) moved = worknode.kids.splice(0);
    else if (picked.plain) {
      moved = picked.kids.splice(0);
      linkHome = picked.kids;
    } else moved = out.list.splice(out.at, 1);
    outcomes.kids.push({ name, kids: moved, project: false, plain: true });
    linkHome.push({ name: `${name}.lnk`, kids: [], project: false, file: true, size: 1_024, stamp: Date.now() });
    return mutation(relParts);
  }

  // The tutorial hands the user's new project a believable set of files to deliver.
  function fill(relParts: string[], paths: string[]): void {
    const spot = find(relParts);
    const draft = spot?.list[spot.at];
    if (draft && draft.kids.length === 0) draft.kids.push(...skeleton(paths));
  }

  function setFlag(relParts: string[], on: unknown, flag: unknown): Mutation {
    const spot = find(relParts);
    const draft = spot?.list[spot.at];
    if (draft && flag === "data") draft.data = on === true;
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
    fill,
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
        case "add_child":
          return addChild(parts(args), args?.kind);
        case "set_node_output":
          return setOutput(parts(args), args?.output);
        case "deliver_output":
          return deliver(parts(args), args?.date, args?.all);
        case "restore_output":
          return mutation(parts(args));
        case "set_user_flag":
          return setFlag(parts(args), args?.on, args?.flag);
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
        case "folder_sizes":
          return null;
        default:
          throw { kind: "internal", message: "Not available in the tutorial." };
      }
    },
  };
}
