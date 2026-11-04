// Disclosure of a board card: where the workboard lives, who owns it, and a way to open its folder.
import { el } from "../ui/dom";
import { iconButton } from "../ui/icon-button";
import { ownerOf } from "../store/node-info";
import { openFolder } from "./context-menu";
import type { TreeNode } from "../ipc/types";

const NO_VALUE = "—";

export function cardDetails(n: TreeNode, name: string, id: string, open: boolean, onToggle: (open: boolean) => void): { toggle: HTMLButtonElement; panel: HTMLElement } {
  const facts: [string, string][] = [
    ["Path", n.relParts.slice(0, -1).join(" › ") || NO_VALUE],
    ["Owner", ownerOf(n) ?? NO_VALUE],
  ];
  const explorer = el("button", { type: "button", class: "btn board-explorer" }, "Open in Explorer");
  explorer.disabled = !n.actions.open;
  explorer.addEventListener("click", () => void openFolder(n));
  const panel = el("div", { id, class: "board-details" }, el("dl", { class: "board-facts" }, ...facts.flatMap(([term, value]) => [el("dt", {}, term), el("dd", {}, value)])), explorer);
  panel.hidden = !open;

  const toggle = iconButton(`Details of ${name}`, "chevronDown");
  toggle.classList.add("board-toggle");
  toggle.setAttribute("aria-expanded", String(open));
  toggle.setAttribute("aria-controls", id);
  toggle.addEventListener("click", () => {
    const next = toggle.getAttribute("aria-expanded") !== "true";
    panel.hidden = !next;
    toggle.setAttribute("aria-expanded", String(next));
    onToggle(next);
  });
  return { toggle, panel };
}
