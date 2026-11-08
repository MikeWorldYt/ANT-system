// Preview of a project sort: sure groups start ticked, doubtful ones wait for the user.
import { el } from "../../../dom";
import { cancelButton, dialogButton } from "../../../atoms/button";
import { dialogText, openDialog } from "../dialog-shell";
import { plural } from "../../../../services/plural";
import type { ProjectGroup, ProjectPlan } from "../../../../services/ipc/types";

function row(group: ProjectGroup): { element: HTMLElement; box: HTMLInputElement } {
  const box = el("input", { type: "checkbox", "aria-label": group.name });
  box.checked = group.tier === "sure";
  const owner = group.ownerConflict ? "several owners" : (group.owner ?? "no owner");
  const summary = `${group.tier === "sure" ? "" : "Check this: "}${plural(group.files.length, "file")}, ${owner}`;
  const files = el("ul", { class: "project-files" }, ...group.files.map((file) => el("li", {}, file)));
  const head = el("label", { class: "project-head" }, box, el("strong", {}, group.name), el("span", { class: "project-summary" }, summary));
  return { element: el("div", { class: "project-row" }, head, files), box };
}

// Resolves to the ticked projects; none when cancelled.
export async function projectsDialog(groups: ProjectGroup[]): Promise<ProjectPlan[]> {
  const rows = groups.map((group) => ({ group, ...row(group) }));
  const { closed } = openDialog({
    title: "Sort by projects",
    wide: true,
    body: [dialogText("Ticked projects get their own folder. Files without a project stay where they are."), el("div", { class: "project-list" }, ...rows.map((r) => r.element))],
    buttons: [cancelButton(), dialogButton("Apply", "ok", "primary")],
  });
  if ((await closed) !== "ok") return [];
  return rows.filter((r) => r.box.checked).map(({ group: { name, files, owner, created } }) => ({ name, files, owner, created }));
}
