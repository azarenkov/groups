export type EnterAction = "send" | "newline" | "native" | "none";

export interface EnterKey {
  key: string;
  shiftKey: boolean;
  ctrlKey: boolean;
  metaKey: boolean;
  altKey: boolean;
  isComposing: boolean;
}

export function enterAction(e: EnterKey): EnterAction {
  if (e.key !== "Enter" || e.isComposing) return "none";
  if (e.ctrlKey || e.metaKey) return "newline";
  if (e.shiftKey) return "native";
  if (e.altKey) return "none";
  return "send";
}

export function insertNewline(
  value: string,
  start: number,
  end: number,
): { value: string; caret: number } {
  return { value: `${value.slice(0, start)}\n${value.slice(end)}`, caret: start + 1 };
}
