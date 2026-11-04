// Small dropdown menu that opens under a button and closes on pick, Escape or an outside click.
import { el } from "../../dom";
import { focusInitial, menuItem, openPopup, placeUnder, popupTrigger, rove, separator } from "../../molecules/menu";

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
  let closeMenu: ((restore?: boolean) => void) | null = null;
  const menuId = `anchored-menu-${++menuCount}`;
  const close = (restore = true) => closeMenu?.(restore);

  function open(): void {
    const list = choices();
    const items = list.map((c) =>
      menuItem(c.label, c.checked !== undefined ? "menuitemradio" : "menuitem", true, () => {
        close();
        c.run();
      }, "ctx-item", c.checked),
    );
    const menu = el("div", { id: menuId, class: "ctx-menu", role: "menu", "aria-label": label }, el("div", { class: "ctx-group", role: "group" }, ...items.flatMap((b, i) => (list[i]?.divider ? [separator(), b] : [b]))));
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
    closeMenu = openPopup(anchor, menu, () => placeUnder(menu, r), () => (closeMenu = null));
    focusInitial(items);
  }

  anchor.setAttribute("aria-haspopup", "menu");
  anchor.setAttribute("aria-expanded", "false");
  anchor.setAttribute("aria-controls", menuId);
  popupTrigger(anchor, ["ArrowDown"], () => closeMenu, open);
}
