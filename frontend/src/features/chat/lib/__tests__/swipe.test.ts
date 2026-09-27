import {
  IDLE_WHEEL_SWIPE,
  SWIPE_MAX_PX,
  SWIPE_TRIGGER_PX,
  WHEEL_SWIPE_GAP_MS,
  nextWheelSwipe,
  wheelSwipeOffset,
  touchSwipeOffset,
} from "../swipe";

describe("touch swipe", () => {
  it("follows a left swipe up to the limit", () => {
    expect(touchSwipeOffset(30, 5)).toBe(30);
    expect(touchSwipeOffset(200, 5)).toBe(SWIPE_MAX_PX);
  });

  it("ignores a right swipe", () => {
    expect(touchSwipeOffset(-40, 0)).toBe(0);
  });

  it("gives way to a vertical scroll", () => {
    expect(touchSwipeOffset(20, 40)).toBeNull();
  });
});

describe("trackpad swipe", () => {
  it("fires once when a horizontal gesture passes the threshold", () => {
    let state = IDLE_WHEEL_SWIPE;
    const fired: boolean[] = [];
    for (let i = 0; i < 10; i += 1) {
      const next = nextWheelSwipe(state, 15, 1, 1000 + i * 16);
      state = next.state;
      fired.push(next.fire);
    }
    expect(fired.filter(Boolean)).toHaveLength(1);
    expect(state.accum).toBeGreaterThanOrEqual(SWIPE_TRIGGER_PX);
  });

  it("does not react to vertical scrolling", () => {
    let state = IDLE_WHEEL_SWIPE;
    for (let i = 0; i < 20; i += 1) {
      const next = nextWheelSwipe(state, 3, 40, 1000 + i * 16);
      expect(next.fire).toBe(false);
      state = next.state;
    }
  });

  it("starts over after a pause", () => {
    let { state } = nextWheelSwipe(IDLE_WHEEL_SWIPE, 70, 0, 1000);
    expect(state.fired).toBe(true);
    const later = nextWheelSwipe(state, 70, 0, 1000 + WHEEL_SWIPE_GAP_MS + 1);
    expect(later.fire).toBe(true);
  });
});

describe("wheelSwipeOffset", () => {
  it("follows the fingers until the reply fires", () => {
    expect(wheelSwipeOffset({ accum: 30, lastAt: 0, fired: false })).toBe(30);
  });

  it("springs back as soon as the reply fires, even while momentum keeps scrolling", () => {
    const fired = nextWheelSwipe(IDLE_WHEEL_SWIPE, 70, 0, 1000).state;
    expect(wheelSwipeOffset(fired)).toBe(0);
    const momentum = nextWheelSwipe(fired, 20, 0, 1016).state;
    expect(wheelSwipeOffset(momentum)).toBe(0);
  });
});
