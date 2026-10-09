export const MIN_CENTER_WIDTH = 480;

export interface PanelBounds {
  min: number;
  max: number;
}

export function clampPanelWidth(
  requested: number,
  bounds: PanelBounds,
  viewport: number,
  otherPanels: number,
  minCenter: number = MIN_CENTER_WIDTH,
): number {
  const room = viewport - otherPanels - minCenter;
  const upper = Math.max(bounds.min, Math.min(bounds.max, room));
  return Math.round(Math.min(upper, Math.max(bounds.min, requested)));
}

export const PANELS_CHANGED = "rift:panels-changed";

export const announcePanels = (source: string) =>
  window.dispatchEvent(new CustomEvent<string>(PANELS_CHANGED, { detail: source }));
