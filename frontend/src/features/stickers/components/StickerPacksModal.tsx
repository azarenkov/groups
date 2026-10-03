import React from "react";
import { createPortal } from "react-dom";
import { useTranslation } from "react-i18next";

import Icon from "../../../shared/ui/Icon";
import { ApiError } from "../../../shared/lib/apiError";
import { openReport } from "../../moderation/store/reportModalStore";
import { copyTextToClipboard } from "../../../shared/lib/clipboard";
import { publicAppOrigin } from "../../../shared/config/runtime";
import { useAuthStore } from "../../auth/store/authStore";
import { stickersAPI, type StickerPack } from "../api/stickers";
import { isInstalled, moveSticker, packCover, stickerSession, useStickerStore } from "../store/stickerStore";
import StickerImage from "./StickerImage";

type View = { kind: "search" } | { kind: "mine" } | { kind: "create" } | { kind: "pack"; id: string };

interface StickerPacksModalProps {
  onClose: () => void;
  packId?: string;
}

const SEARCH_DEBOUNCE_MS = 250;
const STICKER_ACCEPT = "image/png,image/jpeg,image/webp,image/gif";

const errorText = (err: unknown, fallback: string) =>
  err instanceof ApiError && err.message ? err.message : fallback;

const orderedPreview = (pack: StickerPack) => {
  const cover = packCover(pack);
  return cover ? [cover, ...pack.stickers.filter((s) => s.id !== cover.id)].slice(0, 5) : [];
};

export const packLink = (slug: string) => `${publicAppOrigin()}/stickers/${slug}`;

const PackRow: React.FC<{
  pack: StickerPack;
  installed: boolean;
  owned: boolean;
  onOpen: () => void;
  onToggle: () => void;
  busy: boolean;
}> = ({ pack, installed, owned, onOpen, onToggle, busy }) => {
  const { t } = useTranslation("stickers");
  return (
    <div className="sticker-pack-row">
      <button type="button" className="sticker-pack-row-main" onClick={onOpen}>
        <span className="sticker-pack-row-name">{pack.name}</span>
        <span className="sticker-pack-row-meta">{t("count", { count: pack.stickerCount })}</span>
        <span className="sticker-pack-row-preview">
          {orderedPreview(pack).map((s) => (
            <StickerImage key={s.id} id={s.id} emoji={s.emoji} size={40} width={s.width} height={s.height} animated={s.animated} />
          ))}
        </span>
      </button>
      {owned && installed ? (
        <button type="button" className="s-btn secondary" onClick={onOpen}>
          {t("manageOwn")}
        </button>
      ) : owned ? (
        <button type="button" className="s-btn primary" disabled={busy} onClick={onToggle}>
          {t("add")}
        </button>
      ) : (
        <button
          type="button"
          className={"s-btn " + (installed ? "secondary" : "primary")}
          disabled={busy}
          onClick={onToggle}
        >
          {installed ? t("remove") : t("add")}
        </button>
      )}
    </div>
  );
};

const StickerPacksModal: React.FC<StickerPacksModalProps> = ({ onClose, packId }) => {
  const { t } = useTranslation("stickers");
  const userId = useAuthStore((s) => s.user?.id ?? null);
  const myPacks = useStickerStore((s) => s.packs);
  const [view, setView] = React.useState<View>(packId ? { kind: "pack", id: packId } : { kind: "search" });
  const [query, setQuery] = React.useState("");
  const [results, setResults] = React.useState<StickerPack[]>([]);
  const [searching, setSearching] = React.useState(false);
  const [busy, setBusy] = React.useState(false);
  const [error, setError] = React.useState<string | null>(null);
  const [pack, setPack] = React.useState<StickerPack | null>(null);
  const [name, setName] = React.useState("");
  const [selectedSticker, setSelectedSticker] = React.useState<string | null>(null);
  const [renaming, setRenaming] = React.useState(false);
  const [draftName, setDraftName] = React.useState("");
  const [dragging, setDragging] = React.useState<string | null>(null);
  const [linkCopied, setLinkCopied] = React.useState(false);

  React.useEffect(() => {
    setSelectedSticker(null);
    setRenaming(false);
    setLinkCopied(false);
  }, [view]);
  const [emoji, setEmoji] = React.useState("🙂");
  const fileRef = React.useRef<HTMLInputElement>(null);

  const installedIds = React.useMemo(() => new Set(myPacks.filter(isInstalled).map((p) => p.id)), [myPacks]);

  React.useEffect(() => {
    if (!userId) return;
    if (useStickerStore.getState().userId !== userId) void useStickerStore.getState().load(userId);
    else void useStickerStore.getState().refresh();
  }, [userId]);

  React.useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key !== "Escape") return;
      e.stopPropagation();
      onClose();
    };
    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
  }, [onClose]);

  React.useEffect(() => {
    if (view.kind !== "search") return;
    if (!query.trim()) {
      setResults([]);
      setSearching(false);
      return;
    }
    let cancelled = false;
    setSearching(true);
    const timer = window.setTimeout(() => {
      stickersAPI
        .search(query.trim())
        .then((packs) => {
          if (!cancelled) setResults(packs);
        })
        .catch((err) => {
          if (!cancelled) setError(errorText(err, t("loadFailed")));
        })
        .finally(() => {
          if (!cancelled) setSearching(false);
        });
    }, SEARCH_DEBOUNCE_MS);
    return () => {
      cancelled = true;
      window.clearTimeout(timer);
    };
  }, [query, view.kind, t]);

  React.useEffect(() => {
    if (view.kind !== "pack") return;
    let cancelled = false;
    setPack(null);
    stickersAPI
      .pack(view.id)
      .then((loaded) => {
        if (!cancelled) setPack(loaded);
      })
      .catch((err) => {
        if (!cancelled) setError(errorText(err, t("loadFailed")));
      });
    return () => {
      cancelled = true;
    };
  }, [view, t]);

  const run = async (action: (sameSession: () => boolean) => Promise<void>) => {
    setBusy(true);
    setError(null);
    try {
      await action(stickerSession());
    } catch (err) {
      setError(errorText(err, t("actionFailed")));
    } finally {
      setBusy(false);
    }
  };

  const toggleInstall = (target: StickerPack) =>
    run(async () => {
      if (installedIds.has(target.id)) await useStickerStore.getState().uninstall(target.id);
      else await useStickerStore.getState().install(target);
    });

  const createPack = () =>
    run(async (sameSession) => {
      const created = await stickersAPI.create(name.trim());
      if (!sameSession()) return;
      useStickerStore.getState().upsertPack(created);
      setName("");
      setView({ kind: "pack", id: created.id });
    });

  const addSticker = (file: File) =>
    run(async (sameSession) => {
      if (!pack) return;
      const updated = await stickersAPI.addSticker(pack.id, file, emoji.trim());
      if (!sameSession()) return;
      setPack(updated);
      useStickerStore.getState().upsertPack(updated);
    });

  const removeSticker = (stickerId: string) =>
    run(async (sameSession) => {
      if (!pack) return;
      await stickersAPI.removeSticker(pack.id, stickerId);
      if (!sameSession()) return;
      setPack({
        ...pack,
        stickers: pack.stickers.filter((s) => s.id !== stickerId),
        stickerCount: Math.max(0, pack.stickerCount - 1),
      });
      useStickerStore.getState().forgetSticker(stickerId);
    });

  const deletePack = () =>
    run(async (sameSession) => {
      if (!pack || !window.confirm(t("deleteConfirm", { name: pack.name }))) return;
      await stickersAPI.remove(pack.id);
      if (!sameSession()) return;
      useStickerStore.getState().dropPack(pack.id);
      setView({ kind: "mine" });
    });

  const updatePack = (next: StickerPack) => {
    setPack(next);
    if (useStickerStore.getState().packs.some((p) => p.id === next.id)) useStickerStore.getState().upsertPack(next);
  };

  const renamePack = () =>
    run(async (sameSession) => {
      if (!pack) return;
      const name = draftName.trim();
      await stickersAPI.rename(pack.id, name);
      if (!sameSession()) return;
      updatePack({ ...pack, name });
      setRenaming(false);
    });

  const reorder = (stickerId: string, to: number) => {
    if (!pack) return;
    const ids = moveSticker(
      pack.stickers.map((s) => s.id),
      stickerId,
      to,
    );
    if (ids.every((id, i) => id === pack.stickers[i]?.id)) return;
    const previous = pack;
    const byId = new Map(pack.stickers.map((s) => [s.id, s]));
    updatePack({ ...pack, stickers: ids.map((id) => byId.get(id)!) });
    void run(async (sameSession) => {
      try {
        await stickersAPI.reorder(previous.id, ids);
      } catch (err) {
        if (sameSession()) updatePack(previous);
        throw err;
      }
    });
  };

  const makeCover = (stickerId: string) =>
    run(async (sameSession) => {
      if (!pack) return;
      await stickersAPI.setCover(pack.id, stickerId);
      if (!sameSession()) return;
      updatePack({ ...pack, coverStickerId: stickerId });
    });

  const copyLink = async () => {
    if (!pack) return;
    try {
      await copyTextToClipboard(packLink(pack.slug));
      setLinkCopied(true);
    } catch {
      setError(t("actionFailed"));
    }
  };

  const own = !!pack && pack.ownerId === userId;
  const coverId = pack ? packCover(pack)?.id : undefined;
  const selectedIndex = pack && selectedSticker ? pack.stickers.findIndex((s) => s.id === selectedSticker) : -1;

  return createPortal(
    <div className="modal-overlay sticker-modal-overlay" onMouseDown={onClose} onSubmit={(e) => e.stopPropagation()}>
      <div className="modal sticker-modal" role="dialog" aria-modal="true" onMouseDown={(e) => e.stopPropagation()}>
        <div className="m-head">
          <h3>{t("title")}</h3>
          <button type="button" className="close" onClick={onClose} title={t("close")}>
            <Icon name="x" size={14} />
          </button>
        </div>
        <div className="sticker-modal-tabs" role="tablist">
          {(["search", "mine", "create"] as const).map((kind) => (
            <button
              key={kind}
              type="button"
              role="tab"
              aria-selected={view.kind === kind}
              className={"sticker-modal-tab" + (view.kind === kind ? " on" : "")}
              onClick={() => {
                setError(null);
                setView({ kind });
              }}
            >
              {t(`tabs.${kind}`)}
            </button>
          ))}
        </div>
        <div className="m-body sticker-modal-body">
          {view.kind === "search" && (
            <>
              <input
                className="s-input sticker-modal-search"
                autoFocus
                value={query}
                placeholder={t("searchByName")}
                aria-label={t("searchByName")}
                onChange={(e) => setQuery(e.target.value)}
              />
              {results.length === 0 ? (
                <div className="sticker-modal-empty">
                  {!query.trim() ? t("searchHint") : searching ? t("loading") : t("nothingFound")}
                </div>
              ) : (
                results.map((p) => (
                  <PackRow
                    key={p.id}
                    pack={p}
                    installed={installedIds.has(p.id)}
                    owned={p.ownerId === userId}
                    busy={busy}
                    onOpen={() => setView({ kind: "pack", id: p.id })}
                    onToggle={() => void toggleInstall(p)}
                  />
                ))
              )}
            </>
          )}
          {view.kind === "mine" &&
            (myPacks.length === 0 ? (
              <div className="sticker-modal-empty">{t("noPacks")}</div>
            ) : (
              myPacks.map((p) => (
                <PackRow
                  key={p.id}
                  pack={p}
                  installed={isInstalled(p)}
                  owned={p.ownerId === userId}
                  busy={busy}
                  onOpen={() => setView({ kind: "pack", id: p.id })}
                  onToggle={() => void toggleInstall(p)}
                />
              ))
            ))}
          {view.kind === "create" && (
            <form
              className="sticker-create"
              onSubmit={(e) => {
                e.preventDefault();
                void createPack();
              }}
            >
              <label className="sticker-field">
                <span>{t("name")}</span>
                <input
                  className="s-input"
                  value={name}
                  maxLength={64}
                  onChange={(e) => setName(e.target.value)}
                />
              </label>
              <button type="submit" className="s-btn primary" disabled={busy || !name.trim()}>
                {t("create")}
              </button>
            </form>
          )}
          {view.kind === "pack" &&
            (!pack ? (
              !error && <div className="sticker-modal-empty">{t("loading")}</div>
            ) : (
              <div className="sticker-pack-view">
                <div className="sticker-pack-head">
                  <div>
                    {renaming ? (
                      <form
                        className="sticker-pack-rename"
                        onSubmit={(e) => {
                          e.preventDefault();
                          void renamePack();
                        }}
                      >
                        <input
                          className="s-input"
                          autoFocus
                          value={draftName}
                          maxLength={64}
                          aria-label={t("name")}
                          onChange={(e) => setDraftName(e.target.value)}
                        />
                        <button type="submit" className="s-btn primary" disabled={busy || draftName.trim().length < 3}>
                          {t("save")}
                        </button>
                        <button type="button" className="s-btn secondary" onClick={() => setRenaming(false)}>
                          {t("cancel")}
                        </button>
                      </form>
                    ) : (
                      <div className="sticker-pack-title">
                        <span className="sticker-pack-name">{pack.name}</span>
                        {own && (
                          <button
                            type="button"
                            className="sticker-icon-btn"
                            title={t("rename")}
                            aria-label={t("rename")}
                            onClick={() => {
                              setDraftName(pack.name);
                              setRenaming(true);
                            }}
                          >
                            <Icon name="edit" size={13} />
                          </button>
                        )}
                      </div>
                    )}
                    <div className="sticker-pack-meta">
                      {t("count", { count: pack.stickerCount })} · /{pack.slug} ·{" "}
                      <button type="button" className="sticker-link-btn" onClick={() => void copyLink()}>
                        {linkCopied ? t("linkCopied") : t("copyLink")}
                      </button>
                    </div>
                  </div>
                  {own ? (
                    <>
                      {!installedIds.has(pack.id) && (
                        <button type="button" className="s-btn primary" disabled={busy} onClick={() => void toggleInstall(pack)}>
                          {t("add")}
                        </button>
                      )}
                      <button type="button" className="s-btn danger" disabled={busy} onClick={() => void deletePack()}>
                        {t("deletePack")}
                      </button>
                    </>
                  ) : (
                    <>
                      <button
                        type="button"
                        className="s-btn secondary"
                        onClick={() => {
                          onClose();
                          openReport({ type: "stickerPack", id: pack.id, name: pack.name });
                        }}
                      >
                        {t("report")}
                      </button>
                      <button
                        type="button"
                        className={"s-btn " + (installedIds.has(pack.id) ? "secondary" : "primary")}
                        disabled={busy}
                        onClick={() => void toggleInstall(pack)}
                      >
                        {installedIds.has(pack.id) ? t("remove") : t("add")}
                      </button>
                    </>
                  )}
                </div>
                <div className="sticker-pack-grid">
                  {pack.stickers.map((s, index) => (
                    <div
                      key={s.id}
                      className={
                        "sticker-pack-cell" +
                        (selectedSticker === s.id ? " selected" : "") +
                        (dragging === s.id ? " dragging" : "")
                      }
                      draggable={own && !busy}
                      onDragStart={(e) => {
                        e.dataTransfer.effectAllowed = "move";
                        setDragging(s.id);
                      }}
                      onDragEnd={() => setDragging(null)}
                      onDragOver={(e) => {
                        if (dragging) e.preventDefault();
                      }}
                      onDrop={(e) => {
                        e.preventDefault();
                        if (dragging) reorder(dragging, index);
                        setDragging(null);
                      }}
                    >
                      <button
                        type="button"
                        className="sticker-pack-select"
                        aria-pressed={selectedSticker === s.id}
                        aria-label={s.emoji}
                        onClick={() => setSelectedSticker((current) => (current === s.id ? null : s.id))}
                      >
                        <StickerImage id={s.id} emoji={s.emoji} size={88} width={s.width} height={s.height} animated={s.animated} />
                        <span className="sticker-pack-emoji" aria-hidden="true">
                          {s.emoji}
                        </span>
                        {s.id === coverId && <span className="sticker-pack-cover-badge">{t("cover")}</span>}
                      </button>
                      {own && (
                        <button
                          type="button"
                          className="sticker-pack-remove"
                          title={t("removeSticker")}
                          aria-label={t("removeSticker")}
                          disabled={busy}
                          onClick={() => void removeSticker(s.id)}
                        >
                          <Icon name="x" size={11} />
                        </button>
                      )}
                    </div>
                  ))}
                </div>
                {own && selectedSticker && selectedIndex >= 0 && (
                  <div className="sticker-pack-tools">
                    <button
                      type="button"
                      className="s-btn secondary"
                      title={t("moveLeft")}
                      aria-label={t("moveLeft")}
                      disabled={busy || selectedIndex === 0}
                      onClick={() => reorder(selectedSticker, selectedIndex - 1)}
                    >
                      <Icon name="chev-left" size={14} />
                    </button>
                    <button
                      type="button"
                      className="s-btn secondary"
                      title={t("moveRight")}
                      aria-label={t("moveRight")}
                      disabled={busy || selectedIndex === pack.stickers.length - 1}
                      onClick={() => reorder(selectedSticker, selectedIndex + 1)}
                    >
                      <Icon name="chev" size={14} />
                    </button>
                    <button
                      type="button"
                      className="s-btn secondary"
                      disabled={busy || selectedSticker === coverId}
                      onClick={() => void makeCover(selectedSticker)}
                    >
                      {t("makeCover")}
                    </button>
                  </div>
                )}
                {own && pack.stickers.length > 1 && <small className="sticker-pack-hint">{t("reorderHint")}</small>}
                {own && (
                  <div className="sticker-add">
                    <input
                      className="s-input sticker-add-emoji"
                      value={emoji}
                      maxLength={16}
                      aria-label={t("emoji")}
                      title={t("emoji")}
                      onChange={(e) => setEmoji(e.target.value)}
                    />
                    <input
                      ref={fileRef}
                      type="file"
                      accept={STICKER_ACCEPT}
                      hidden
                      onChange={(e) => {
                        const file = e.target.files?.[0];
                        e.target.value = "";
                        if (file) void addSticker(file);
                      }}
                    />
                    <button
                      type="button"
                      className="s-btn primary"
                      disabled={busy || !emoji.trim()}
                      onClick={() => fileRef.current?.click()}
                    >
                      {t("addSticker")}
                    </button>
                    <small>{t("addHint")}</small>
                  </div>
                )}
              </div>
            ))}
          {error && (
            <div className="sticker-modal-error" role="alert">
              {error}
            </div>
          )}
        </div>
      </div>
    </div>,
    document.body,
  );
};

export default StickerPacksModal;
