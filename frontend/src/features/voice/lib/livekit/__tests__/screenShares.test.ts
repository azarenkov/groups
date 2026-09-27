import { SCREEN_HOLD_MS, followWatched, nextWatched } from "../screenShares";

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

describe("followWatched", () => {
  it("keeps waiting for a share that dropped out for a moment", () => {
    const gone = followWatched([], ["ann"], null, 1000);
    expect(gone.watched).toEqual([]);
    expect(gone.held).toEqual({ identity: "ann", until: 1000 + SCREEN_HOLD_MS, replacement: null });
    const back = followWatched([share("ann")], gone.watched, gone.held, 2000);
    expect(back).toEqual({ watched: ["ann"], held: null });
  });

  it("returns to the dropped share even after moving to another one", () => {
    const gone = followWatched([share("bob")], ["ann"], null, 1000);
    expect(gone.watched).toEqual(["bob"]);
    const back = followWatched([share("ann"), share("bob")], gone.watched, gone.held, 2000);
    expect(back.watched).toEqual(["ann"]);
  });

  it("lets go once the share stays away too long", () => {
    const gone = followWatched([], ["ann"], null, 1000);
    const late = followWatched([share("ann")], [], gone.held, 1000 + SCREEN_HOLD_MS + 1);
    expect(late).toEqual({ watched: [], held: null });
  });

  it("promotes a chosen share and keeps it when the dropped one returns", () => {
    const shares = [share("bob"), share("carol")];
    const gone = followWatched(shares, ["ann", "bob"], null, 1000);
    expect(gone.watched).toEqual(["bob"]);
    const back = followWatched([share("ann"), ...shares], gone.watched, gone.held, 2000);
    expect(back).toEqual({ watched: ["ann", "bob"], held: null });
  });

  it("drops a chosen extra that ended", () => {
    const next = followWatched([share("ann")], ["ann", "bob"], null, 1000);
    expect(next).toEqual({ watched: ["ann"], held: null });
  });
});
