// Small dropdown menu that opens under a button and closes on pick, Escape or an outside click.
import { el } from "./dom";

export interface MenuChoice {
  label: string;
  // Present for radio-style menus, where exactly one choice is checked.
  checked?: boolean;
  run(): void;
}

export function attachMenu(anchor: HTMLElement, label: string, choices: () => MenuChoice[]): void {
  let closeMenu: (() => void) | null = null;

  function open(): void {
    const items = choices().map((c) => {
      const radio = c.checked !== undefined;
      const b = el("button", { type: "button", role: radio ? "menuitemradio" : "menuitem", tabindex: "-1", class: "ctx-item" }, c.label);
      if (radio) b.setAttribute("aria-checked", String(c.checked));
      b.addEventListener("click", () => {
        close();
        c.run();
      });
      return b;
    });
    const menu = el("div", { class: "ctx-menu", role: "menu", "aria-label": label }, el("div", { class: "ctx-group", role: "group" }, ...items));
    const outside = (ev: Event) => {
      if (!menu.contains(ev.target as Node) && !anchor.contains(ev.target as Node)) close();
    };
    function close(): void {
      menu.remove();
      document.removeEventListener("pointerdown", outside, true);
      anchor.setAttribute("aria-expanded", "false");
      closeMenu = null;
      anchor.focus();
    }
    const focusAt = (to: number) => items[(to + items.length) % items.length]?.focus();
    menu.addEventListener("keydown", (ev) => {
      const i = items.indexOf(document.activeElement as HTMLButtonElement);
      if (ev.key === "ArrowDown") focusAt(i + 1);
      else if (ev.key === "ArrowUp") focusAt(i - 1);
      else if (ev.key === "Escape") close();
      else return;
      ev.preventDefault();
    });
    const r = anchor.getBoundingClientRect();
    menu.style.left = `${r.left}px`;
    menu.style.top = `${r.bottom + 4}px`;
    document.body.append(menu);
    document.addEventListener("pointerdown", outside, true);
    anchor.setAttribute("aria-expanded", "true");
    closeMenu = close;
    (items.find((b) => b.getAttribute("aria-checked") === "true") ?? items[0])?.focus();
  }

  anchor.setAttribute("aria-haspopup", "menu");
  anchor.setAttribute("aria-expanded", "false");
  anchor.addEventListener("click", () => (closeMenu ? closeMenu() : open()));
}
