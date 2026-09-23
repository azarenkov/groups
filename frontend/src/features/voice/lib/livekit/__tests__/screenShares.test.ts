import { nextWatched } from "../screenShares";

const share = (identity: string, isLocal = false) => ({
  identity,
  name: identity,
  isLocal,
});

describe("nextWatched", () => {
  it("shows nothing until someone is chosen", () => {
    expect(nextWatched([share("ann"), share("bob")], null)).toBeNull();
  });

  it("keeps the chosen stream while it lasts", () => {
    expect(nextWatched([share("ann"), share("bob")], "ann")).toBe("ann");
  });

  it("moves to the next stream when the chosen one ends", () => {
    expect(nextWatched([share("bob")], "ann")).toBe("bob");
  });

  it("falls back to your own stream when no one else shares", () => {
    expect(nextWatched([share("me", true)], "ann")).toBe("me");
  });

  it("prefers someone else's stream over your own", () => {
    expect(nextWatched([share("me", true), share("bob")], "ann")).toBe("bob");
  });

  it("goes back to the grid when every stream ended", () => {
    expect(nextWatched([], "ann")).toBeNull();
  });
});
