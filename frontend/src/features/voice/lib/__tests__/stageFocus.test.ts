import {
  AUTO_HOLD_MS,
  AUTO_MIN_GAP_MS,
  IDLE_AUTO_FOCUS,
  autoFocusWait,
  nextAutoFocus,
} from "../stageFocus";
import type { StageCandidate } from "../stageFocus";

const entry = (id: string, over: Partial<StageCandidate> = {}): StageCandidate => ({
  id,
  speaking: false,
  hasVideo: true,
  isSelf: false,
  ...over,
});

describe("nextAutoFocus", () => {
  it("starts on whoever speaks, or the first other person with video", () => {
    expect(nextAutoFocus(IDLE_AUTO_FOCUS, [entry("a"), entry("b", { speaking: true })], 0).current).toBe("b");
    expect(nextAutoFocus(IDLE_AUTO_FOCUS, [entry("me", { isSelf: true }), entry("a")], 0).current).toBe("a");
  });

  it("puts yourself on stage only when nobody else has video", () => {
    const solo = [entry("me", { isSelf: true, speaking: true }), entry("a", { hasVideo: false, speaking: true })];
    expect(nextAutoFocus(IDLE_AUTO_FOCUS, solo, 0).current).toBe("me");
  });

  it("switches to a new speaker only after they keep talking and not too often", () => {
    let state = nextAutoFocus(IDLE_AUTO_FOCUS, [entry("a", { speaking: true }), entry("b")], 0);
    const talkingB = [entry("a"), entry("b", { speaking: true })];
    state = nextAutoFocus(state, talkingB, 100);
    expect(state.current).toBe("a");
    state = nextAutoFocus(state, talkingB, 100 + AUTO_HOLD_MS);
    expect(state.current).toBe("a");
    state = nextAutoFocus(state, talkingB, AUTO_MIN_GAP_MS);
    expect(state.current).toBe("b");
  });

  it("ignores a short remark", () => {
    let state = nextAutoFocus(IDLE_AUTO_FOCUS, [entry("a", { speaking: true }), entry("b")], 0);
    state = nextAutoFocus(state, [entry("a"), entry("b", { speaking: true })], 2000);
    state = nextAutoFocus(state, [entry("a"), entry("b")], 2200);
    expect(state.candidate).toBeNull();
    state = nextAutoFocus(state, [entry("a"), entry("b")], 2000 + AUTO_HOLD_MS + 50);
    expect(state.current).toBe("a");
  });

  it("does not jump away while the current speaker is still talking", () => {
    let state = nextAutoFocus(IDLE_AUTO_FOCUS, [entry("a", { speaking: true }), entry("b")], 0);
    state = nextAutoFocus(state, [entry("a", { speaking: true }), entry("b", { speaking: true })], 5000);
    state = nextAutoFocus(state, [entry("a", { speaking: true }), entry("b", { speaking: true })], 9000);
    expect(state.current).toBe("a");
  });

  it("moves on right away when the person on stage turns their video off", () => {
    let state = nextAutoFocus(IDLE_AUTO_FOCUS, [entry("a"), entry("b")], 0);
    state = nextAutoFocus(state, [entry("a", { hasVideo: false }), entry("b")], 100);
    expect(state.current).toBe("b");
  });
});

describe("autoFocusWait", () => {
  it("tells how long to wait before a pending switch can happen", () => {
    let state = nextAutoFocus(IDLE_AUTO_FOCUS, [entry("a", { speaking: true }), entry("b")], 0);
    state = nextAutoFocus(state, [entry("a"), entry("b", { speaking: true })], 1000);
    expect(autoFocusWait(state, 1000)).toBe(Math.max(AUTO_HOLD_MS, AUTO_MIN_GAP_MS - 1000));
    expect(autoFocusWait(IDLE_AUTO_FOCUS, 0)).toBeNull();
  });
});
