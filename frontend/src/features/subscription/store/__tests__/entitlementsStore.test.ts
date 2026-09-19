import { entitlementsAPI } from "../../api/entitlements";
import { useEntitlementsStore } from "../entitlementsStore";

jest.mock("../../api/entitlements", () => ({
  entitlementsAPI: {
    getMine: jest.fn(),
    setBadgeStyle: jest.fn(),
    lookupBadges: jest.fn(),
  },
}));

const mockedApi = entitlementsAPI as jest.Mocked<typeof entitlementsAPI>;

const deferred = <T,>() => {
  let resolve: (value: T) => void = () => {};
  let reject: (reason?: unknown) => void = () => {};
  const promise = new Promise<T>((res, rej) => {
    resolve = res;
    reject = rej;
  });
  return { promise, resolve, reject };
};

describe("entitlementsStore badge style", () => {
  beforeEach(() => {
    useEntitlementsStore.getState().clear();
    useEntitlementsStore.getState().applyBadgeStyle("tint");
    useEntitlementsStore.getState().applyTier("pro");
  });

  it("sends one request at a time and ends on the last click", async () => {
    const first = deferred<"ink">();
    const second = deferred<"solid">();
    mockedApi.setBadgeStyle
      .mockImplementationOnce(() => first.promise)
      .mockImplementationOnce(() => second.promise);

    const done = useEntitlementsStore.getState().setBadgeStyle("ink");
    void useEntitlementsStore.getState().setBadgeStyle("solid");
    expect(mockedApi.setBadgeStyle).toHaveBeenCalledTimes(1);
    expect(useEntitlementsStore.getState().badgeStyle).toBe("solid");

    first.resolve("ink");
    await Promise.resolve();
    await Promise.resolve();
    expect(mockedApi.setBadgeStyle).toHaveBeenCalledTimes(2);
    expect(mockedApi.setBadgeStyle).toHaveBeenLastCalledWith("solid");
    expect(useEntitlementsStore.getState().badgeStyle).toBe("solid");

    second.resolve("solid");
    await done;
    expect(useEntitlementsStore.getState().badgeStyle).toBe("solid");
  });

  it("puts the confirmed style back when the last click fails", async () => {
    mockedApi.setBadgeStyle
      .mockResolvedValueOnce("ink")
      .mockRejectedValueOnce(new Error("nope"));

    const done = useEntitlementsStore.getState().setBadgeStyle("ink");
    void useEntitlementsStore.getState().setBadgeStyle("solid");

    await expect(done).rejects.toThrow("nope");
    expect(useEntitlementsStore.getState().badgeStyle).toBe("ink");
  });

  it("keeps the newer click when an older request fails", async () => {
    mockedApi.setBadgeStyle
      .mockRejectedValueOnce(new Error("first failed"))
      .mockResolvedValueOnce("solid");

    const done = useEntitlementsStore.getState().setBadgeStyle("ink");
    void useEntitlementsStore.getState().setBadgeStyle("solid");

    await done;
    expect(mockedApi.setBadgeStyle).toHaveBeenCalledTimes(2);
    expect(useEntitlementsStore.getState().badgeStyle).toBe("solid");
  });
});

describe("entitlementsStore startup fetch", () => {
  beforeEach(() => {
    useEntitlementsStore.getState().clear();
  });

  it("does not let a slow fetch undo a realtime tier change", async () => {
    const response = deferred<{ tier: "free"; badgeStyle: "tint"; memberNumber: null }>();
    mockedApi.getMine.mockImplementationOnce(() => response.promise);
    const fetching = useEntitlementsStore.getState().fetch();
    useEntitlementsStore.getState().applyTier("pro");
    response.resolve({ tier: "free", badgeStyle: "tint", memberNumber: null });
    await fetching;
    expect(useEntitlementsStore.getState().tier).toBe("pro");
    expect(useEntitlementsStore.getState().loaded).toBe(true);
  });

  it("applies the fetch when nothing changed meanwhile", async () => {
    mockedApi.getMine.mockResolvedValueOnce({ tier: "premium", badgeStyle: "ink", memberNumber: 12 });
    await useEntitlementsStore.getState().fetch();
    expect(useEntitlementsStore.getState().tier).toBe("premium");
    expect(useEntitlementsStore.getState().badgeStyle).toBe("ink");
    expect(useEntitlementsStore.getState().memberNumber).toBe(12);
  });
});

describe("entitlementsStore session reset", () => {
  beforeEach(() => {
    useEntitlementsStore.getState().clear();
  });

  it("starts a fresh request for the next session instead of reusing the old one", async () => {
    const stale = deferred<"ink">();
    mockedApi.setBadgeStyle
      .mockImplementationOnce(() => stale.promise)
      .mockResolvedValueOnce("ink");
    useEntitlementsStore.getState().applyTier("pro");
    const old = useEntitlementsStore.getState().setBadgeStyle("ink");

    useEntitlementsStore.getState().clear();
    useEntitlementsStore.getState().applyBadgeStyle("tint");
    const fresh = useEntitlementsStore.getState().setBadgeStyle("ink");
    expect(fresh).not.toBe(old);

    stale.resolve("ink");
    await old;
    await fresh;
    expect(mockedApi.setBadgeStyle).toHaveBeenCalledTimes(2);
    expect(useEntitlementsStore.getState().badgeStyle).toBe("ink");
  });
});

describe("entitlementsStore remote badge events", () => {
  beforeEach(() => {
    useEntitlementsStore.getState().clear();
    useEntitlementsStore.getState().applyTier("pro");
    useEntitlementsStore.getState().applyBadgeStyle("tint");
  });

  it("keeps a newer event from another session over an older response", async () => {
    const response = deferred<"ink">();
    mockedApi.setBadgeStyle.mockImplementationOnce(() => response.promise);
    const done = useEntitlementsStore.getState().setBadgeStyle("ink");
    useEntitlementsStore.getState().applyBadgeStyle("solid");
    response.resolve("ink");
    await done;
    expect(useEntitlementsStore.getState().badgeStyle).toBe("solid");
    expect(mockedApi.setBadgeStyle).toHaveBeenCalledTimes(1);
  });
});
