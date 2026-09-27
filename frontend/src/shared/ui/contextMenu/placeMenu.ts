export const MENU_MARGIN = 8;

export function placeMenu(
  x: number,
  y: number,
  width: number,
  height: number,
  viewportWidth: number,
  viewportHeight: number,
  margin: number = MENU_MARGIN,
): { left: number; top: number; maxHeight: number; maxWidth: number } {
  const left =
    x + width + margin <= viewportWidth ? x : Math.max(margin, x - width);
  const top =
    y + height + margin <= viewportHeight
      ? y
      : Math.max(margin, viewportHeight - height - margin);
  return {
    left,
    top,
    maxHeight: Math.max(0, viewportHeight - top - margin),
    maxWidth: Math.max(0, viewportWidth - left - margin),
  };
}
