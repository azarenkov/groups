import { installedPacks, searchMyStickers, useStickerStore } from "../stickerStore";
import { stickersAPI, type PackSticker, type StickerPack } from "../../api/stickers";

jest.mock("../../api/stickers", () => ({
  stickersAPI: {
    mine: jest.fn().mockResolvedValue([]),
    install: jest.fn().mockResolvedValue(undefined),
    uninstall: jest.fn().mockResolvedValue(undefined),
    pack: jest.fn(),
  },
}));

const sticker = (id: string, packId: string, emoji: string): PackSticker => ({
  id,
  packId,
  emoji,
  animated: false,
  width: 512,
  height: 512,
});

const pack = (id: string, stickers: PackSticker[]): StickerPack => ({
  id,
  ownerId: "owner",
  name: `Pack ${id}`,
  slug: `pack_${id}`,
  installCount: 0,
  stickerCount: stickers.length,
  stickers,
});

describe("stickerStore", () => {
  beforeEach(() => {
    localStorage.clear();
    useStickerStore.getState().clear();
  });

  it("keeps recent stickers unique, newest first and per user", async () => {
    await useStickerStore.getState().load("u1");
    const a = sticker("a", "p", "😺");
    const b = sticker("b", "p", "🐶");
    useStickerStore.getState().rememberUsed(a);
    useStickerStore.getState().rememberUsed(b);
    useStickerStore.getState().rememberUsed(a);
    expect(useStickerStore.getState().recent.map((s) => s.id)).toEqual(["a", "b"]);
    await useStickerStore.getState().load("u2");
    expect(useStickerStore.getState().recent).toEqual([]);
    await useStickerStore.getState().load("u1");
    expect(useStickerStore.getState().recent.map((s) => s.id)).toEqual(["a", "b"]);
  });

  it("adds, updates and removes packs and their recent stickers", () => {
    const first = pack("1", [sticker("a", "1", "😺")]);
    useStickerStore.getState().upsertPack(first);
    useStickerStore.getState().upsertPack({ ...first, name: "Renamed" });
    expect(useStickerStore.getState().packs.map((p) => p.name)).toEqual(["Renamed"]);
    useStickerStore.getState().rememberUsed(first.stickers[0]);
    useStickerStore.getState().dropPack("1");
    expect(useStickerStore.getState().packs).toEqual([]);
    expect(useStickerStore.getState().recent).toEqual([]);
  });
});

describe("stickerStore recovery", () => {
  beforeEach(() => {
    (stickersAPI.mine as jest.Mock).mockResolvedValue([]);
    localStorage.clear();
    useStickerStore.getState().clear();
  });

  it("does not let a refresh started before a removal bring the pack back", async () => {
    const mine = stickersAPI.mine as jest.Mock;
    const installed = pack("1", [sticker("a", "1", "😺")]);
    mine.mockResolvedValueOnce([installed]);
    await useStickerStore.getState().load("u1");
    let resolveStale: (packs: StickerPack[]) => void = () => undefined;
    mine.mockReturnValueOnce(new Promise<StickerPack[]>((resolve) => (resolveStale = resolve)));
    mine.mockResolvedValueOnce([]);
    const refreshing = useStickerStore.getState().refresh();
    useStickerStore.getState().dropPack("1");
    resolveStale([installed]);
    await refreshing;
    expect(useStickerStore.getState().packs).toEqual([]);
    expect(mine).toHaveBeenCalledTimes(3);
  });

  it("keeps an owned pack the server left uninstalled out of the picker until it is installed", async () => {
    const owned = { ...pack("1", [sticker("a", "1", "😺")]), installed: false };
    (stickersAPI.mine as jest.Mock).mockResolvedValueOnce([owned]);
    await useStickerStore.getState().load("u1");
    expect(installedPacks(useStickerStore.getState().packs)).toEqual([]);
    useStickerStore.getState().upsertPack({ ...owned, installed: undefined, name: "Renamed" });
    expect(installedPacks(useStickerStore.getState().packs)).toEqual([]);
    (stickersAPI.install as jest.Mock).mockResolvedValueOnce(undefined);
    await useStickerStore.getState().install(owned);
    expect(installedPacks(useStickerStore.getState().packs).map((p) => p.id)).toEqual(["1"]);
  });

  it("refreshes packs again only once they are stale", async () => {
    const mine = stickersAPI.mine as jest.Mock;
    const now = jest.spyOn(Date, "now").mockReturnValue(1_000_000);
    try {
      await useStickerStore.getState().load("u1");
      expect(mine).toHaveBeenCalledTimes(1);
      await useStickerStore.getState().refreshIfStale(60_000);
      expect(mine).toHaveBeenCalledTimes(1);
      now.mockReturnValue(1_060_000);
      mine.mockResolvedValueOnce([pack("1", [sticker("a", "1", "😺")])]);
      await useStickerStore.getState().refreshIfStale(60_000);
      expect(mine).toHaveBeenCalledTimes(2);
      expect(useStickerStore.getState().packs).toHaveLength(1);
    } finally {
      now.mockRestore();
    }
  });

  it("ignores an install that finishes after the account switched", async () => {
    await useStickerStore.getState().load("u1");
    let finishInstall: () => void = () => undefined;
    (stickersAPI.install as jest.Mock).mockReturnValueOnce(new Promise<void>((resolve) => (finishInstall = resolve)));
    const installing = useStickerStore.getState().install(pack("1", [sticker("a", "1", "😺")]));
    await useStickerStore.getState().load("u2");
    finishInstall();
    await installing;
    expect(useStickerStore.getState().packs).toEqual([]);
  });

  it("tries again after a failed load", async () => {
    const mine = stickersAPI.mine as jest.Mock;
    mine.mockRejectedValueOnce(new Error("offline"));
    await useStickerStore.getState().load("u1");
    expect(useStickerStore.getState()).toMatchObject({ loaded: false, failed: true });
    mine.mockResolvedValueOnce([pack("1", [sticker("a", "1", "😺")])]);
    await useStickerStore.getState().load("u1");
    expect(useStickerStore.getState()).toMatchObject({ loaded: true, failed: false });
    expect(useStickerStore.getState().packs).toHaveLength(1);
  });

  it("forgets a removed sticker everywhere", async () => {
    await useStickerStore.getState().load("u1");
    const a = sticker("a", "1", "😺");
    useStickerStore.getState().upsertPack(pack("1", [a, sticker("b", "1", "🐶")]));
    useStickerStore.getState().rememberUsed(a);
    useStickerStore.getState().forgetSticker("a");
    expect(useStickerStore.getState().recent).toEqual([]);
    expect(useStickerStore.getState().packs[0].stickers.map((s) => s.id)).toEqual(["b"]);
    expect(localStorage.getItem("rift.recentStickers.u1")).toBe("[]");
  });
});

describe("searchMyStickers", () => {
  it("finds stickers across packs by their emoji hint", () => {
    const packs = [pack("1", [sticker("a", "1", "😺 ❤️"), sticker("b", "1", "🐶")]), pack("2", [sticker("c", "2", "😺")])];
    expect(searchMyStickers(packs, "😺").map((s) => s.id)).toEqual(["a", "c"]);
    expect(searchMyStickers(packs, "  ")).toEqual([]);
  });
});
