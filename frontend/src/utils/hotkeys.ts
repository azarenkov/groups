export type HotkeyGroup = "navigation" | "messaging" | "voice";
export type HotkeyScope = "global" | "composer";
export type HotkeyActionId =
  | "quickSwitcher"
  | "prevChannel"
  | "nextChannel"
  | "historyBack"
  | "historyForward"
  | "openSettings"
  | "editLastMessage"
  | "toggleMute"
  | "leaveVoice";

export interface HotkeyAction {
  id: HotkeyActionId;
  group: HotkeyGroup;
  scope: HotkeyScope;
  defaultBinding: string;
}

export interface ParsedBinding {
  mod: boolean;
  ctrl: boolean;
  alt: boolean;
  shift: boolean;
  key: string;
}

export const HOTKEY_ACTIONS: HotkeyAction[] = [
  { id: "quickSwitcher", group: "navigation", scope: "global", defaultBinding: "Mod+K" },
  { id: "prevChannel", group: "navigation", scope: "global", defaultBinding: "Alt+ArrowUp" },
  { id: "nextChannel", group: "navigation", scope: "global", defaultBinding: "Alt+ArrowDown" },
  { id: "historyBack", group: "navigation", scope: "global", defaultBinding: "Mod+BracketLeft" },
  { id: "historyForward", group: "navigation", scope: "global", defaultBinding: "Mod+BracketRight" },
  { id: "openSettings", group: "navigation", scope: "global", defaultBinding: "Mod+Comma" },
  { id: "editLastMessage", group: "messaging", scope: "composer", defaultBinding: "ArrowUp" },
  { id: "toggleMute", group: "voice", scope: "global", defaultBinding: "Mod+Shift+M" },
  { id: "leaveVoice", group: "voice", scope: "global", defaultBinding: "Mod+Shift+L" },
];

export const HOTKEY_ACTION_IDS: HotkeyActionId[] = HOTKEY_ACTIONS.map((a) => a.id);

export const DESKTOP_SHORTCUTS: { id: string; binding: string; key: string }[] = [
  { id: "zoomIn", binding: "Mod+Equal", key: "Equal" },
  { id: "zoomOut", binding: "Mod+Minus", key: "Minus" },
  { id: "zoomReset", binding: "Mod+0", key: "0" },
];

export function desktopReservedId(binding: string): string | null {
  const parsed = parseBinding(binding);
  if (!parsed || !(parsed.mod || parsed.ctrl)) return null;
  return DESKTOP_SHORTCUTS.find((d) => d.key === parsed.key)?.id ?? null;
}

export const isHotkeyActionId = (value: unknown): value is HotkeyActionId =>
  typeof value === "string" && (HOTKEY_ACTION_IDS as string[]).includes(value);

export const hotkeyAction = (id: HotkeyActionId): HotkeyAction =>
  HOTKEY_ACTIONS.find((a) => a.id === id) as HotkeyAction;

export const IS_MAC =
  typeof navigator !== "undefined" &&
  (/Mac|iPhone|iPad/.test(navigator.platform) || /Macintosh/.test(navigator.userAgent));

const MODIFIER_ORDER = ["Mod", "Ctrl", "Alt", "Shift"] as const;

const NAMED_KEYS = new Set([
  "ArrowUp",
  "ArrowDown",
  "ArrowLeft",
  "ArrowRight",
  "Escape",
  "Space",
  "Enter",
  "Tab",
  "Backspace",
  "Delete",
  "Insert",
  "Home",
  "End",
  "PageUp",
  "PageDown",
  "Comma",
  "Period",
  "Slash",
  "Backslash",
  "BracketLeft",
  "BracketRight",
  "Semicolon",
  "Quote",
  "Backquote",
  "Minus",
  "Equal",
  "IntlBackslash",
]);

const CODE_TOKENS: Record<string, string> = {
  NumpadEnter: "Enter",
};

const FUNCTION_KEY = /^F([1-9]|1[0-2])$/;
const NAV_KEY = /^(Arrow(Up|Down|Left|Right)|Home|End|PageUp|PageDown|Escape|F([1-9]|1[0-2]))$/;

const isKeyToken = (token: string): boolean =>
  /^[A-Z0-9]$/.test(token) || FUNCTION_KEY.test(token) || NAMED_KEYS.has(token);

export function parseBinding(binding: string): ParsedBinding | null {
  if (!binding || binding.length > 40) return null;
  const parts = binding.split("+");
  const key = parts[parts.length - 1];
  if (!isKeyToken(key)) return null;
  const parsed: ParsedBinding = { mod: false, ctrl: false, alt: false, shift: false, key };
  let last = -1;
  for (const modifier of parts.slice(0, -1)) {
    const index = (MODIFIER_ORDER as readonly string[]).indexOf(modifier);
    if (index < 0 || index <= last) return null;
    last = index;
    if (modifier === "Mod") parsed.mod = true;
    if (modifier === "Ctrl") parsed.ctrl = true;
    if (modifier === "Alt") parsed.alt = true;
    if (modifier === "Shift") parsed.shift = true;
  }
  return parsed;
}

export const bindingAvailable = (binding: string): boolean => {
  const parsed = parseBinding(binding);
  return !!parsed && (IS_MAC || !parsed.ctrl);
};

const platformModifiers = (parsed: ParsedBinding): ParsedBinding | null =>
  IS_MAC || !parsed.ctrl ? parsed : null;

export const isValidBinding = (binding: unknown): binding is string =>
  typeof binding === "string" && parseBinding(binding) !== null;

export function keyTokenFromCode(code: string): string | null {
  if (/^Key[A-Z]$/.test(code)) return code.slice(3);
  if (/^Digit[0-9]$/.test(code)) return code.slice(5);
  if (FUNCTION_KEY.test(code)) return code;
  if (code in CODE_TOKENS) return CODE_TOKENS[code];
  if (NAMED_KEYS.has(code)) return code;
  return null;
}

export const keyTokenFromEvent = (e: KeyboardEvent): string | null =>
  keyTokenFromCode(e.code);

export function bindingIsBareKey(binding: string, token: string | null): boolean {
  const parsed = parseBinding(binding);
  return (
    !!parsed &&
    token !== null &&
    !parsed.mod &&
    !parsed.ctrl &&
    !parsed.alt &&
    !parsed.shift &&
    parsed.key === token
  );
}

const modifiersFromEvent = (e: KeyboardEvent) =>
  IS_MAC
    ? { mod: e.metaKey, ctrl: e.ctrlKey, alt: e.altKey, shift: e.shiftKey }
    : { mod: e.ctrlKey, ctrl: false, alt: e.altKey, shift: e.shiftKey };

const isAltGraph = (e: KeyboardEvent): boolean =>
  typeof e.getModifierState === "function" && e.getModifierState("AltGraph");

export function bindingFromEvent(e: KeyboardEvent): string | null {
  if (isAltGraph(e)) return null;
  const key = keyTokenFromEvent(e);
  if (!key) return null;
  const mods = modifiersFromEvent(e);
  const parts: string[] = [];
  if (mods.mod) parts.push("Mod");
  if (mods.ctrl) parts.push("Ctrl");
  if (mods.alt) parts.push("Alt");
  if (mods.shift) parts.push("Shift");
  parts.push(key);
  return parts.join("+");
}

export function eventMatches(e: KeyboardEvent, binding: string): boolean {
  if (isAltGraph(e)) return false;
  const raw = parseBinding(binding);
  const parsed = raw ? platformModifiers(raw) : null;
  if (!parsed) return false;
  if (keyTokenFromEvent(e) !== parsed.key) return false;
  const mods = modifiersFromEvent(e);
  return (
    mods.mod === parsed.mod &&
    mods.ctrl === parsed.ctrl &&
    mods.alt === parsed.alt &&
    mods.shift === parsed.shift
  );
}

export const hasStrongModifier = (parsed: ParsedBinding): boolean =>
  parsed.mod || parsed.ctrl || parsed.alt;

export type BindingVerdict = "ok" | "needModifier";

export function bindingVerdict(action: HotkeyAction, binding: string): BindingVerdict {
  const parsed = parseBinding(binding);
  if (!parsed) return "needModifier";
  if (hasStrongModifier(parsed)) return "ok";
  if (action.scope === "composer") return NAV_KEY.test(parsed.key) ? "ok" : "needModifier";
  return FUNCTION_KEY.test(parsed.key) || parsed.key === "Escape" ? "ok" : "needModifier";
}

const KEY_GLYPHS: Record<string, string> = {
  ArrowUp: "↑",
  ArrowDown: "↓",
  ArrowLeft: "←",
  ArrowRight: "→",
  Escape: "Esc",
  Enter: "↵",
  Tab: "⇥",
  Backspace: "⌫",
  Delete: "⌦",
  Insert: "Ins",
  Space: "Space",
  Home: "Home",
  End: "End",
  PageUp: "PgUp",
  PageDown: "PgDn",
  Comma: ",",
  Period: ".",
  Slash: "/",
  Backslash: "\\",
  BracketLeft: "[",
  BracketRight: "]",
  Semicolon: ";",
  Quote: "'",
  Backquote: "`",
  Minus: "-",
  Equal: "=",
  IntlBackslash: "\\",
};

export function bindingKeycaps(binding: string): string[] {
  const parsed = parseBinding(binding);
  if (!parsed) return [binding];
  const caps: string[] = [];
  if (IS_MAC) {
    if (parsed.ctrl) caps.push("⌃");
    if (parsed.alt) caps.push("⌥");
    if (parsed.shift) caps.push("⇧");
    if (parsed.mod) caps.push("⌘");
  } else {
    if (parsed.ctrl) caps.push("⌃");
    if (parsed.mod) caps.push("Ctrl");
    if (parsed.alt) caps.push("Alt");
    if (parsed.shift) caps.push("Shift");
  }
  caps.push(KEY_GLYPHS[parsed.key] ?? parsed.key);
  return caps;
}

export const modifierGlyphs = (): string => (IS_MAC ? "⌘ / ⌥ / ⌃" : "Ctrl / Alt");

export const codeLabel = (code: string): string => {
  if (code.startsWith("Key")) return code.slice(3);
  if (code.startsWith("Digit")) return code.slice(5);
  return KEY_GLYPHS[code] ?? code;
};

export const isTextEntryTarget = (target: EventTarget | null): boolean => {
  const el = target as HTMLElement | null;
  if (!el || typeof el.closest !== "function") return false;
  return (
    el.closest('input, textarea, select, [contenteditable="true"]') !== null ||
    el.isContentEditable
  );
};
