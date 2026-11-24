// The row that heads ANT.TEMPLATES in every view and sort: it creates a template instead of standing for a folder.
import { el } from "../dom";
import { guiIcon } from "../atoms/gui-icons";
import { newTemplate } from "../../hooks/templates";
import type { TreeNode } from "../../services/ipc/types";

export const isTemplatesHome = (n: TreeNode | null | undefined): n is TreeNode => n?.relParts.length === 1 && n.name.toUpperCase() === "ANT.TEMPLATES";

export function newTemplateItem(home: TreeNode, cls: string, role: string): HTMLElement {
  const item = el("div", { class: `${cls} new-template`, role, tabindex: "-1", title: "Create a worknode template" }, el("span", { class: "virtual-icon", "aria-hidden": "true" }, guiIcon("add")), el("span", { class: "name" }, el("span", { class: "name-text" }, "New template")));
  const create = (ev: Event) => {
    ev.stopPropagation();
    void newTemplate(home.relParts);
  };
  item.addEventListener("click", create);
  item.addEventListener("keydown", (ev) => {
    if (ev.key === "Enter" || ev.key === " ") create(ev);
  });
  return item;
}
