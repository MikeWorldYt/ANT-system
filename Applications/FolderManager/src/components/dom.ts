// Tiny element factory so modules build DOM without innerHTML, plus the attribute habits every control shares.
export function el<K extends keyof HTMLElementTagNameMap>(
  tag: K,
  attrs: Record<string, string> = {},
  ...kids: (Node | string)[]
): HTMLElementTagNameMap[K] {
  const node = document.createElement(tag);
  for (const [k, v] of Object.entries(attrs)) node.setAttribute(k, v);
  node.append(...kids);
  return node;
}

// aria-disabled keeps a control focusable, so its handlers check this instead of the disabled property.
export const isDisabled = (node: Element): boolean => node.getAttribute("aria-disabled") === "true";

export const setDisabled = (node: Element, disabled: boolean): void => node.setAttribute("aria-disabled", String(disabled));

// Controls inside a row keep their clicks, so the row neither selects, opens nor starts a drag under them.
export function keepClicks(node: Element): void {
  for (const type of ["click", "dblclick", "pointerdown"]) node.addEventListener(type, (ev) => ev.stopPropagation());
}
