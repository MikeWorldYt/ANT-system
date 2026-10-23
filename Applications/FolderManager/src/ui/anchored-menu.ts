// Small dropdown menu that opens under a button and closes on pick, Escape or an outside click.
import { el } from "./dom";

export interface MenuChoice {
  label: string;
  // Present for radio-style menus, where exactly one choice is checked.
  checked?: boolean;
  // Draws a separator above this choice.
  divider?: boolean;
  run(): void;
}

let menuCount = 0;
const GAP = 4;

export function attachMenu(anchor: HTMLElement, label: string, choices: () => MenuChoice[]): void {
  let closeMenu: (() => void) | null = null;
  const menuId = `anchored-menu-${++menuCount}`;

  function open(): void {
    const list = choices();
    const items = list.map((c) => {
      const radio = c.checked !== undefined;
      const b = el("button", { type: "button", role: radio ? "menuitemradio" : "menuitem", tabindex: "-1", class: "ctx-item" }, c.label);
      if (radio) b.setAttribute("aria-checked", String(c.checked));
      b.addEventListener("click", () => {
        close();
        c.run();
      });
      return b;
    });
    const menu = el("div", { id: menuId, class: "ctx-menu", role: "menu", "aria-label": label }, el("div", { class: "ctx-group", role: "group" }, ...items.flatMap((b, i) => (list[i]?.divider ? [el("div", { class: "ctx-sep", role: "separator" }), b] : [b]))));
    const outside = (ev: Event) => {
      if (!menu.contains(ev.target as Node) && !anchor.contains(ev.target as Node)) close(false);
    };
    const onResize = () => close();
    function close(restore = true): void {
      if (closeMenu === null) return;
      closeMenu = null;
      menu.remove();
      document.removeEventListener("pointerdown", outside, true);
      window.removeEventListener("resize", onResize);
      anchor.setAttribute("aria-expanded", "false");
      if (restore) anchor.focus();
    }
    const focusAt = (to: number) => items[(to + items.length) % items.length]?.focus();
    menu.addEventListener("keydown", (ev) => {
      const i = items.indexOf(document.activeElement as HTMLButtonElement);
      if (ev.key === "ArrowDown") focusAt(i + 1);
      else if (ev.key === "ArrowUp") focusAt(i - 1);
      else if (ev.key === "Home") focusAt(0);
      else if (ev.key === "End") focusAt(items.length - 1);
      else if (ev.key === "Escape") close();
      else if (ev.key === "Tab") {
        close();
        return;
      } else return;
      ev.preventDefault();
    });
    menu.addEventListener("focusout", (ev) => {
      if (!menu.contains(ev.relatedTarget as Node | null)) close(false);
    });
    // Keeps a press on the menu padding from blurring the focused item, which would close the menu.
    menu.addEventListener("mousedown", (ev) => ev.preventDefault());
    const r = anchor.getBoundingClientRect();
    document.body.append(menu);
    const { width, height } = menu.getBoundingClientRect();
    const below = r.bottom + GAP;
    const top = below + height <= window.innerHeight || r.top - GAP - height < GAP ? Math.min(below, window.innerHeight - height - GAP) : r.top - GAP - height;
    menu.style.left = `${Math.max(GAP, Math.min(r.left, window.innerWidth - width - GAP))}px`;
    menu.style.top = `${Math.max(GAP, top)}px`;
    document.addEventListener("pointerdown", outside, true);
    window.addEventListener("resize", onResize);
    anchor.setAttribute("aria-expanded", "true");
    closeMenu = close;
    (items.find((b) => b.getAttribute("aria-checked") === "true") ?? items[0])?.focus();
  }

  anchor.setAttribute("aria-haspopup", "menu");
  anchor.setAttribute("aria-expanded", "false");
  anchor.setAttribute("aria-controls", menuId);
  anchor.addEventListener("click", () => (closeMenu ? closeMenu() : open()));
  anchor.addEventListener("keydown", (ev) => {
    if (ev.key !== "ArrowDown" || closeMenu) return;
    ev.preventDefault();
    open();
  });
}
