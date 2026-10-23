// Small dropdown menu that opens under a button and closes on pick, Escape or an outside click.
import { el } from "./dom";
import { EDGE, autoDismiss, focusInitial, menuItem, place, rove, separator } from "./menu";

export interface MenuChoice {
  label: string;
  // Present for radio-style menus, where exactly one choice is checked.
  checked?: boolean;
  // Draws a separator above this choice.
  divider?: boolean;
  run(): void;
}

let menuCount = 0;

export function attachMenu(anchor: HTMLElement, label: string, choices: () => MenuChoice[]): void {
  let closeMenu: (() => void) | null = null;
  const menuId = `anchored-menu-${++menuCount}`;

  function open(): void {
    const list = choices();
    const items = list.map((c) =>
      menuItem(c.label, c.checked !== undefined ? "menuitemradio" : "menuitem", true, () => {
        close();
        c.run();
      }, "ctx-item", c.checked),
    );
    const menu = el("div", { id: menuId, class: "ctx-menu", role: "menu", "aria-label": label }, el("div", { class: "ctx-group", role: "group" }, ...items.flatMap((b, i) => (list[i]?.divider ? [separator(), b] : [b]))));
    function close(restore = true): void {
      if (closeMenu === null) return;
      closeMenu = null;
      menu.remove();
      stopWatching();
      anchor.setAttribute("aria-expanded", "false");
      if (restore) anchor.focus();
    }
    menu.addEventListener("keydown", (ev) => {
      if (ev.key === "Tab") {
        close();
        return;
      }
      if (ev.key === "Escape") {
        close();
        ev.preventDefault();
      } else rove(items, ev);
    });
    menu.addEventListener("focusout", (ev) => {
      if (!menu.contains(ev.relatedTarget as Node | null)) close(false);
    });
    // Keeps a press on the menu padding from blurring the focused item, which would close the menu.
    menu.addEventListener("mousedown", (ev) => ev.preventDefault());
    const r = anchor.getBoundingClientRect();
    document.body.append(menu);
    const { height } = menu.getBoundingClientRect();
    const below = r.bottom + EDGE;
    place(menu, r.left, below + height <= window.innerHeight || r.top - EDGE - height < EDGE ? below : r.top - EDGE - height);
    const stopWatching = autoDismiss((t) => menu.contains(t) || anchor.contains(t), close);
    anchor.setAttribute("aria-expanded", "true");
    closeMenu = close;
    focusInitial(items);
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
