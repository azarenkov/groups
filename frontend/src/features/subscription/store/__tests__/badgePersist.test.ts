import { useBadgeStore } from "../badgeStore";

jest.mock("../../api/entitlements", () => ({ entitlementsAPI: { badges: jest.fn(() => new Promise(() => undefined)) } }));

it("keeps known badges across a session reset so they show at once after a reload", () => {
  jest.useFakeTimers();
  localStorage.clear();
  useBadgeStore.getState().apply("u1", { tier: "pro", badgeStyle: "tint", memberNumber: null });
  jest.advanceTimersByTime(600);
  expect(JSON.parse(localStorage.getItem("rift.badges.v1") ?? "{}").u1.tier).toBe("pro");
  useBadgeStore.getState().clear();
  expect(useBadgeStore.getState().byUser.u1?.tier).toBe("pro");
  jest.useRealTimers();
});
