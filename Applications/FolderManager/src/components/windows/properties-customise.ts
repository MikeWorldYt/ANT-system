// The Customise tab: the support folders a node may hold (Add or Quit) and the defaults of its new worknodes, applied together from the window's Apply button.
import { el } from "../dom";
import { radio } from "../atoms/checkbox";
import { addSupportNode, describeError, removeSupportNode, supportNodes } from "../../services/ipc/api";
import { worknodeSection } from "./properties-worknode";
import type { OutcomeDefaults, SupportRow } from "../../services/ipc/types";

const LABELS: Record<string, string> = { REV: "Review", RES: "Resources", TPL: "Templates", SS: "Screenshots", REC: "Recordings" };

export interface Section {
  group: HTMLFieldSetElement;
  changed(): boolean;
  // The text of what went wrong, or null once every change is made.
  apply(): Promise<string | null>;
}

export interface Customise {
  panel: HTMLElement;
  changed(): boolean;
  apply(): Promise<string | null>;
}

function supportSection(relParts: string[], first: SupportRow[], onChange: () => void): Section {
  let rows = first;
  let want = rows.map((r) => r.exists);
  const body = el("tbody");
  const group = el("fieldset", { class: "prop-support" }, el("legend", { class: "prop-support-title" }, "Support Nodes"));
  const heads = el("tr", {}, el("th", { scope: "col" }, "Support node"), el("th", { scope: "col", class: "prop-sn-col" }, "Add"), el("th", { scope: "col", class: "prop-sn-col" }, "Quit"));
  const list = el("div", { class: "prop-sn-list" }, el("table", { class: "prop-sn" }, el("thead", {}, heads), body));
  group.append(list);

  const render = (): void => {
    body.replaceChildren(
      ...rows.map((row, i) => {
        const held = row.exists && !row.empty;
        const add = radio({ name: `support-${i}`, "aria-label": `Add ${row.name}` });
        const quit = radio({ name: `support-${i}`, "aria-label": `Quit ${row.name}` });
        add.checked = want[i] === true;
        quit.checked = want[i] === false;
        if (held) {
          quit.disabled = true;
          quit.title = "Not empty";
          quit.setAttribute("aria-label", `Quit ${row.name} (not empty)`);
        }
        add.addEventListener("change", () => {
          want[i] = true;
          onChange();
        });
        quit.addEventListener("change", () => {
          want[i] = false;
          onChange();
        });
        const name = el("th", { scope: "row", title: row.name }, el("span", { class: "prop-sn-kind" }, LABELS[row.kind] ?? row.kind));
        return el("tr", {}, name, el("td", { class: "prop-sn-col" }, add), el("td", { class: "prop-sn-col" }, quit));
      }),
    );
  };
  render();

  const changed = (): boolean => want.some((w, i) => w !== rows[i]?.exists);

  async function apply(): Promise<string | null> {
    group.disabled = true;
    let failure: string | null = null;
    try {
      // Adds go first so a failed removal never leaves a node without its folder.
      for (const [i, row] of rows.entries()) if (want[i] && !row.exists) await addSupportNode(relParts, row.kind);
      for (const [i, row] of rows.entries()) if (!want[i] && row.exists) await removeSupportNode(relParts, row.name);
    } catch (e) {
      failure = describeError(e);
    }
    try {
      const fresh = await supportNodes(relParts);
      if (fresh) {
        rows = fresh;
        want = rows.map((r) => r.exists);
        render();
      } else {
        failure ??= "This folder can no longer hold support nodes.";
      }
    } catch (e) {
      failure ??= `Could not read the support nodes: ${describeError(e)}`;
    }
    group.disabled = false;
    return failure;
  }

  return { group, changed, apply };
}

export function customise(relParts: string[], support: SupportRow[] | null, defaults: OutcomeDefaults | null, onChange: () => void, warn: (text: string) => void): Customise {
  const sections = [support && supportSection(relParts, support, onChange), defaults && worknodeSection(defaults, onChange, warn)].filter((s) => s !== null);
  const panel = el("div", { class: "prop-body" }, ...sections.map((s) => s.group));

  async function apply(): Promise<string | null> {
    const failures: string[] = [];
    for (const s of sections) {
      const failure = s.changed() ? await s.apply() : null;
      if (failure) failures.push(failure);
    }
    return failures.length > 0 ? failures.join(" ") : null;
  }

  return { panel, changed: () => sections.some((s) => s.changed()), apply };
}
