import {
  appendedOwnMessage,
  IDLE_WHEEL_GESTURE,
  STICK_TO_BOTTOM_PX,
  WHEEL_GESTURE_GAP_MS,
  nextWheelGesture,
  shouldStickToBottom,
} from "../scrollRules";

describe("pulling with the wheel", () => {
  it("arms a gesture that starts at the bottom", () => {
    expect(nextWheelGesture(IDLE_WHEEL_GESTURE, 1000, true).armed).toBe(true);
  });

  it("ignores the momentum of a scroll that reached the bottom", () => {
    let gesture = nextWheelGesture(IDLE_WHEEL_GESTURE, 1000, false);
    gesture = nextWheelGesture(gesture, 1016, false);
    gesture = nextWheelGesture(gesture, 1032, true);
    gesture = nextWheelGesture(gesture, 1048, true);
    expect(gesture.armed).toBe(false);
  });

  it("arms again after a pause once the list rests at the bottom", () => {
    let gesture = nextWheelGesture(IDLE_WHEEL_GESTURE, 1000, false);
    gesture = nextWheelGesture(gesture, 1016, true);
    gesture = nextWheelGesture(gesture, 1016 + WHEEL_GESTURE_GAP_MS + 1, true);
    expect(gesture.armed).toBe(true);
  });

  it("disarms when the list leaves the bottom mid gesture", () => {
    let gesture = nextWheelGesture(IDLE_WHEEL_GESTURE, 1000, true);
    gesture = nextWheelGesture(gesture, 1016, false);
    gesture = nextWheelGesture(gesture, 1032, true);
    expect(gesture.armed).toBe(false);
  });
});

describe("sticking to the bottom", () => {
  it("follows new messages when the reader is at the bottom", () => {
    expect(shouldStickToBottom(0, false)).toBe(true);
    expect(shouldStickToBottom(STICK_TO_BOTTOM_PX, false)).toBe(true);
  });

  it("leaves the reader in the history alone", () => {
    expect(shouldStickToBottom(STICK_TO_BOTTOM_PX + 1, false)).toBe(false);
  });

  it("always shows the reader their own message", () => {
    expect(shouldStickToBottom(5000, true)).toBe(true);
  });
});

describe("appendedOwnMessage", () => {
  const m = (id: string, authorId: string) => ({ id, authorId });

  it("sticks to the bottom when you append a message", () => {
    expect(appendedOwnMessage("a", [m("a", "x"), m("b", "me")], "me")).toBe(true);
  });

  it("ignores a tail that is someone else's", () => {
    expect(appendedOwnMessage("a", [m("a", "me"), m("b", "x")], "me")).toBe(false);
  });

  it("ignores a deletion that leaves your older message at the tail", () => {
    expect(appendedOwnMessage("c", [m("a", "x"), m("b", "me")], "me")).toBe(false);
  });

  it("ignores rows prepended in front of an unchanged tail", () => {
    expect(appendedOwnMessage("b", [m("z", "x"), m("a", "x"), m("b", "me")], "me")).toBe(false);
  });

  it("treats the first load as appended", () => {
    expect(appendedOwnMessage(null, [m("a", "me")], "me")).toBe(true);
  });
});
