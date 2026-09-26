import React from "react";
import { createPortal } from "react-dom";

import Icon from "../Icon";
import { useContextMenuStore } from "./contextMenuStore";
import { placeMenu } from "./placeMenu";

const ContextMenuHost: React.FC = () => {
  const menu = useContextMenuStore((s) => s.menu);
  const close = useContextMenuStore((s) => s.close);
  const ref = React.useRef<HTMLDivElement>(null);
  const [pos, setPos] = React.useState<{ left: number; top: number; maxHeight: number } | null>(null);
  const returnFocusRef = React.useRef<HTMLElement | null>(null);

  React.useEffect(() => () => useContextMenuStore.getState().close(), []);

  React.useEffect(() => {
    if (menu) {
      returnFocusRef.current = menu.returnFocus;
      return;
    }
    const target = returnFocusRef.current;
    returnFocusRef.current = null;
    if (!target || !target.isConnected) return;
    const active = document.activeElement;
    if (active && active !== document.body) return;
    target.focus({ preventScroll: true });
  }, [menu]);

  React.useLayoutEffect(() => {
    if (!menu) {
      setPos(null);
      return;
    }
    const el = ref.current;
    if (!el) return;
    const rect = el.getBoundingClientRect();
    setPos(placeMenu(menu.x, menu.y, rect.width, rect.height, window.innerWidth, window.innerHeight));
    const first = el.querySelector<HTMLElement>('[role="menuitem"]:not([aria-disabled="true"])');
    first?.focus({ preventScroll: true });
  }, [menu]);

  React.useEffect(() => {
    if (!menu) return;
    const onPointer = (e: PointerEvent) => {
      if (ref.current && ref.current.contains(e.target as Node)) return;
      close();
    };
    const onScroll = (e: Event) => {
      if (ref.current && ref.current.contains(e.target as Node)) return;
      close();
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.preventDefault();
        close();
      }
    };
    document.addEventListener("pointerdown", onPointer, true);
    document.addEventListener("scroll", onScroll, true);
    document.addEventListener("keydown", onKey, true);
    window.addEventListener("resize", close);
    window.addEventListener("blur", close);
    return () => {
      document.removeEventListener("pointerdown", onPointer, true);
      document.removeEventListener("scroll", onScroll, true);
      document.removeEventListener("keydown", onKey, true);
      window.removeEventListener("resize", close);
      window.removeEventListener("blur", close);
    };
  }, [menu, close]);

  if (!menu) return null;

  const moveFocus = (step: number) => {
    const el = ref.current;
    if (!el) return;
    const items = Array.from(
      el.querySelectorAll<HTMLElement>(
        '[role="menuitem"]:not([aria-disabled="true"]), .ctx-custom input:not([disabled]), .ctx-custom button:not([disabled])',
      ),
    );
    if (items.length === 0) return;
    const index = items.indexOf(document.activeElement as HTMLElement);
    const next = index < 0 ? 0 : (index + step + items.length) % items.length;
    items[next].focus();
  };

  return createPortal(
    <div
      ref={ref}
      key={menu.seq}
      className="ctx-menu"
      role="menu"
      style={pos ? { left: pos.left, top: pos.top, maxHeight: pos.maxHeight, overflowY: "auto" } : { left: menu.x, top: menu.y, visibility: "hidden" }}
      onContextMenu={(e) => e.preventDefault()}
      onKeyDown={(e) => {
        const inControl = (e.target as HTMLElement).closest(".ctx-custom") !== null;
        if (e.key === "ArrowDown" && !inControl) {
          e.preventDefault();
          moveFocus(1);
        } else if (e.key === "ArrowUp" && !inControl) {
          e.preventDefault();
          moveFocus(-1);
        } else if (e.key === "Tab") {
          e.preventDefault();
          moveFocus(e.shiftKey ? -1 : 1);
        }
      }}
    >
      {menu.entries.map((entry) => {
        if (entry.kind === "separator") return <div key={entry.id} className="ctx-sep" role="separator" />;
        if (entry.kind === "custom") return <div key={entry.id} className="ctx-custom">{entry.render()}</div>;
        return (
          <button
            key={entry.id}
            type="button"
            role="menuitem"
            className={"ctx-item" + (entry.danger ? " danger" : "")}
            aria-disabled={entry.disabled || undefined}
            disabled={entry.disabled}
            onClick={() => {
              close();
              entry.onSelect();
            }}
          >
            <span className="ctx-ico">{entry.icon && <Icon name={entry.icon} size={15} />}</span>
            <span className="ctx-label">{entry.label}</span>
          </button>
        );
      })}
    </div>,
    document.body,
  );
};

export default ContextMenuHost;
