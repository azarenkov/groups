import { visibleEntries } from "../contextMenuStore";

const item = (id: string) => ({ kind: "item" as const, id, label: id, onSelect: () => {} });
const sep = (id: string) => ({ kind: "separator" as const, id });

describe("visibleEntries", () => {
  it("drops separators at the edges and doubled ones", () => {
    const out = visibleEntries([sep("s0"), item("a"), sep("s1"), sep("s2"), item("b"), sep("s3")]);
    expect(out.map((e) => e.id)).toEqual(["a", "s1", "b"]);
  });

  it("returns nothing when only separators are left", () => {
    expect(visibleEntries([sep("s0"), sep("s1")])).toEqual([]);
  });
});
