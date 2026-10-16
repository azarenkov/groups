import { nextMediaRevision } from "../mediaRevision";

it("numbers this tab's changes in order even when the clock stands still or goes back", () => {
  const first = nextMediaRevision(1_000);
  const second = nextMediaRevision(1_000);
  const third = nextMediaRevision(500);
  expect(second.seq).toBeGreaterThan(first.seq);
  expect(third.seq).toBeGreaterThan(second.seq);
  expect(new Set([first.source, second.source, third.source]).size).toBe(1);
});
