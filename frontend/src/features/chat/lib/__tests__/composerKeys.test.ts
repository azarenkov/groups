import { enterAction, insertNewline } from "../composerKeys";

const key = (over: Partial<Parameters<typeof enterAction>[0]> = {}) => ({
  key: "Enter",
  shiftKey: false,
  ctrlKey: false,
  metaKey: false,
  altKey: false,
  isComposing: false,
  ...over,
});

describe("enterAction", () => {
  it("sends on plain Enter", () => {
    expect(enterAction(key())).toBe("send");
  });

  it("lets the textarea break the line on Shift+Enter", () => {
    expect(enterAction(key({ shiftKey: true }))).toBe("native");
  });

  it("breaks the line on Ctrl+Enter and Cmd+Enter", () => {
    expect(enterAction(key({ ctrlKey: true }))).toBe("newline");
    expect(enterAction(key({ metaKey: true }))).toBe("newline");
  });

  it("does nothing while an input method is composing", () => {
    expect(enterAction(key({ isComposing: true }))).toBe("none");
  });

  it("ignores other keys", () => {
    expect(enterAction(key({ key: "a" }))).toBe("none");
  });
});

describe("insertNewline", () => {
  it("replaces the selection with a line break and moves the caret after it", () => {
    expect(insertNewline("hello world", 5, 6)).toEqual({ value: "hello\nworld", caret: 6 });
  });
});
