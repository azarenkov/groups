import React from "react";
import { createPortal } from "react-dom";
import { useTranslation } from "react-i18next";

import Icon from "../../../shared/ui/Icon";
import { useAuthStore } from "../../auth/store/authStore";
import { openReport } from "../../moderation/store/reportModalStore";
import { stickersAPI, type StickerPack } from "../api/stickers";
import { isInstalled, stickerSession, useStickerStore } from "../store/stickerStore";
import StickerImage from "./StickerImage";

const VIEWER_SIZE = 320;

interface ViewedSticker {
  id: string;
  packId: string;
  emoji: string;
  width: number;
  height: number;
}

interface StickerViewerProps {
  sticker: ViewedSticker;
  onClose: () => void;
  onOpenPack: (packId: string) => void;
}

type PackState = { kind: "loading" } | { kind: "ready"; pack: StickerPack } | { kind: "gone" };

const StickerViewer: React.FC<StickerViewerProps> = ({ sticker, onClose, onOpenPack }) => {
  const { t } = useTranslation("stickers");
  const userId = useAuthStore((s) => s.user?.id ?? null);
  const myPacks = useStickerStore((s) => s.packs);
  const [state, setState] = React.useState<PackState>({ kind: "loading" });
  const [busy, setBusy] = React.useState(false);
  const [error, setError] = React.useState<string | null>(null);

  React.useEffect(() => {
    if (userId && useStickerStore.getState().userId !== userId) void useStickerStore.getState().load(userId);
  }, [userId]);

  React.useEffect(() => {
    let cancelled = false;
    setState({ kind: "loading" });
    stickersAPI
      .pack(sticker.packId)
      .then((pack) => {
        if (!cancelled) setState({ kind: "ready", pack });
      })
      .catch(() => {
        if (!cancelled) setState({ kind: "gone" });
      });
    return () => {
      cancelled = true;
    };
  }, [sticker.packId]);

  React.useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key !== "Escape") return;
      e.stopPropagation();
      onClose();
    };
    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
  }, [onClose]);

  const installed = myPacks.some((p) => p.id === sticker.packId && isInstalled(p));
  const pack = state.kind === "ready" ? state.pack : null;
  const own = !!pack && pack.ownerId === userId;

  const install = async () => {
    if (!pack) return;
    const sameSession = stickerSession();
    setBusy(true);
    setError(null);
    try {
      await useStickerStore.getState().install(pack);
    } catch {
      if (sameSession()) setError(t("actionFailed"));
    } finally {
      setBusy(false);
    }
  };

  return createPortal(
    <div className="sticker-viewer-overlay" onMouseDown={onClose}>
      <div
        className="sticker-viewer"
        role="dialog"
        aria-modal="true"
        aria-label={t("sticker")}
        onMouseDown={(e) => e.stopPropagation()}
      >
        <button type="button" className="sticker-viewer-close" onClick={onClose} title={t("close")} aria-label={t("close")}>
          <Icon name="x" size={14} />
        </button>
        <StickerImage id={sticker.id} emoji={sticker.emoji} size={VIEWER_SIZE} width={sticker.width} height={sticker.height} />
        <div className="sticker-viewer-info">
          <span className="sticker-viewer-emoji" aria-hidden="true">
            {sticker.emoji}
          </span>
          <span className="sticker-viewer-pack">
            {state.kind === "loading" ? t("loading") : pack ? pack.name : t("packGone")}
          </span>
        </div>
        {pack && (
          <div className="sticker-viewer-actions">
            {!installed && (
              <button type="button" className="s-btn primary" disabled={busy} onClick={() => void install()}>
                {t("addPack")}
              </button>
            )}
            <button type="button" className="s-btn secondary" onClick={() => onOpenPack(pack.id)}>
              {own ? t("manageOwn") : t("openPack")}
            </button>
            {!own && (
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
            )}
          </div>
        )}
        {error && (
          <div className="sticker-modal-error" role="alert">
            {error}
          </div>
        )}
      </div>
    </div>,
    document.body,
  );
};

export default StickerViewer;
