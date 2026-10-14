import React from "react";
import { createPortal } from "react-dom";

export const TOOLTIP_DELAY_MS = 300;
export const TOOLTIP_WARM_MS = 400;
const GAP = 8;
const EDGE = 8;
const SIDE_CONTAINERS = ".sb-rail, .sb-collapsed, .sb-mini-list";

type Placement = "top" | "bottom" | "left" | "right";

interface TipState {
  el: HTMLElement;
  text: string;
}

const TEMP_LABEL = "data-tip-label";

function adopt(el: HTMLElement): string | null {
  const title = el.getAttribute("title");
  if (title && title.trim()) {
    el.setAttribute("data-tip", title);
    el.removeAttribute("title");
    if (!el.hasAttribute("aria-label") && !el.hasAttribute("aria-labelledby") && !el.textContent?.trim()) {
      el.setAttribute("aria-label", title);
      el.setAttribute(TEMP_LABEL, "");
    }
    return title;
  }
  return el.getAttribute("data-tip");
}

function release(el: HTMLElement) {
  const text = el.getAttribute("data-tip");
  if (text === null) return;
  el.removeAttribute("data-tip");
  if (el.hasAttribute(TEMP_LABEL)) {
    el.removeAttribute(TEMP_LABEL);
    el.removeAttribute("aria-label");
  }
  if (!el.hasAttribute("title")) el.setAttribute("title", text);
}

function placementFor(el: HTMLElement): Placement {
  const explicit = el.closest<HTMLElement>("[data-tip-placement]")?.dataset.tipPlacement;
  if (explicit === "top" || explicit === "bottom" || explicit === "left" || explicit === "right") return explicit;
  return el.closest(SIDE_CONTAINERS) ? "right" : "bottom";
}

export function positionTip(
  anchor: DOMRect,
  tip: { width: number; height: number },
  preferred: Placement,
  viewport: { width: number; height: number },
): { top: number; left: number; placement: Placement } {
  const fits = (p: Placement) => {
    if (p === "bottom") return anchor.bottom + GAP + tip.height <= viewport.height - EDGE;
    if (p === "top") return anchor.top - GAP - tip.height >= EDGE;
    if (p === "right") return anchor.right + GAP + tip.width <= viewport.width - EDGE;
    return anchor.left - GAP - tip.width >= EDGE;
  };
  const opposite: Record<Placement, Placement> = { top: "bottom", bottom: "top", left: "right", right: "left" };
  const placement = fits(preferred) || !fits(opposite[preferred]) ? preferred : opposite[preferred];
  let top: number;
  let left: number;
  if (placement === "bottom" || placement === "top") {
    top = placement === "bottom" ? anchor.bottom + GAP : anchor.top - GAP - tip.height;
    left = anchor.left + anchor.width / 2 - tip.width / 2;
  } else {
    left = placement === "right" ? anchor.right + GAP : anchor.left - GAP - tip.width;
    top = anchor.top + anchor.height / 2 - tip.height / 2;
  }
  left = Math.min(Math.max(left, EDGE), Math.max(EDGE, viewport.width - EDGE - tip.width));
  top = Math.min(Math.max(top, EDGE), Math.max(EDGE, viewport.height - EDGE - tip.height));
  return { top, left, placement };
}

const TooltipHost: React.FC = () => {
  const [tip, setTip] = React.useState<TipState | null>(null);
  const [pos, setPos] = React.useState<{ top: number; left: number; placement: Placement } | null>(null);
  const bubbleRef = React.useRef<HTMLDivElement>(null);
  const tipRef = React.useRef<TipState | null>(null);
  tipRef.current = tip;

  React.useEffect(() => {
    let current: HTMLElement | null = null;
    let timer: number | null = null;
    let lastHiddenAt = 0;

    const clearTimer = () => {
      if (timer !== null) window.clearTimeout(timer);
      timer = null;
    };
    const hide = () => {
      clearTimer();
      if (current) release(current);
      current = null;
      if (tipRef.current) lastHiddenAt = Date.now();
      setTip(null);
      setPos(null);
    };
    const onOver = (e: PointerEvent) => {
      if (e.pointerType === "touch") return;
      const target = e.target instanceof Element ? e.target : null;
      const el = target?.closest<HTMLElement>("[title], [data-tip]") ?? null;
      if (el === current) return;
      const warm = !!tipRef.current || Date.now() - lastHiddenAt < TOOLTIP_WARM_MS;
      hide();
      if (!el) return;
      const text = adopt(el);
      if (!text) return;
      current = el;
      const show = () => {
        timer = null;
        if (current === el && el.isConnected) setTip({ el, text });
      };
      if (warm) show();
      else timer = window.setTimeout(show, TOOLTIP_DELAY_MS);
    };
    const onOut = (e: PointerEvent) => {
      if (!current) return;
      const next = e.relatedTarget instanceof Node ? e.relatedTarget : null;
      if (next && current.contains(next)) return;
      hide();
    };
    const onDismiss = () => {
      if (current) hide();
    };
    document.addEventListener("pointerover", onOver, true);
    document.addEventListener("pointerout", onOut, true);
    document.addEventListener("pointerdown", onDismiss, true);
    document.addEventListener("keydown", onDismiss, true);
    document.addEventListener("scroll", onDismiss, true);
    window.addEventListener("blur", onDismiss);
    const watch = window.setInterval(() => {
      if (current && !current.isConnected) hide();
    }, 250);
    return () => {
      hide();
      window.clearInterval(watch);
      document.removeEventListener("pointerover", onOver, true);
      document.removeEventListener("pointerout", onOut, true);
      document.removeEventListener("pointerdown", onDismiss, true);
      document.removeEventListener("keydown", onDismiss, true);
      document.removeEventListener("scroll", onDismiss, true);
      window.removeEventListener("blur", onDismiss);
    };
  }, []);

  React.useLayoutEffect(() => {
    const bubble = bubbleRef.current;
    if (!tip || !bubble) return;
    setPos(
      positionTip(
        tip.el.getBoundingClientRect(),
        { width: bubble.offsetWidth, height: bubble.offsetHeight },
        placementFor(tip.el),
        { width: window.innerWidth, height: window.innerHeight },
      ),
    );
  }, [tip]);

  if (!tip) return null;
  return createPortal(
    <div
      ref={bubbleRef}
      role="tooltip"
      className={"rift-tip" + (pos ? ` shown tip-${pos.placement}` : "")}
      style={pos ? { top: pos.top, left: pos.left } : { top: -9999, left: -9999 }}
    >
      {tip.text}
    </div>,
    document.body,
  );
};

export default TooltipHost;
