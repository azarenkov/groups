type Revisions = typeof import("../mediaRevision");

const freshModule = (): Revisions => {
  let loaded: Revisions | null = null;
  jest.isolateModules(() => {
    loaded = require("../mediaRevision");
  });
  return loaded as unknown as Revisions;
};

beforeEach(() => sessionStorage.clear());

it("numbers this tab's changes in order even when the clock stands still or goes back", () => {
  const { nextMediaRevision } = freshModule();
  const first = nextMediaRevision(1_000);
  const second = nextMediaRevision(1_000);
  const third = nextMediaRevision(500);
  expect(second.seq).toBeGreaterThan(first.seq);
  expect(third.seq).toBeGreaterThan(second.seq);
  expect(new Set([first.source, second.source, third.source]).size).toBe(1);
});

it("keeps counting up after the tab reloads", () => {
  const before = freshModule().nextMediaRevision(5_000);
  const after = freshModule().nextMediaRevision(1_000);
  expect(after.source).toBe(before.source);
  expect(after.seq).toBeGreaterThan(before.seq);
});
