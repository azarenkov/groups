import { extractUuid, randomUuid } from "../ids";

it("makes well-formed ids that differ each time", () => {
  const a = randomUuid();
  const b = randomUuid();
  expect(extractUuid(a)).toBe(a);
  expect(a).not.toBe(b);
});
