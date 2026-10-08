import React from "react";
import { useQuery } from "@tanstack/react-query";
import { createPortal } from "react-dom";
import { useTranslation } from "react-i18next";

import Icon from "../../../shared/ui/Icon";
import { ApiError } from "../../../shared/lib/apiError";
import { queryClient, queryKeys, retryUnless } from "../../../shared/query/queryClient";
import { useAuthStore } from "../../auth/store/authStore";
import { openReport } from "../../moderation/store/reportModalStore";
import { stickersAPI, type StickerPack } from "../api/stickers";
import { isInstalled, stickerSession, useStickerStore } from "../store/stickerStore";
import StickerImage from "./StickerImage";

const VIEWER_SIZE = 320;

interface ViewedSticker {
  id: string;
  animated?: boolean;
  packId: string;
  emoji: string;
  width: number;
  height: number;
}

interface StickerViewerProps {
  sticker: ViewedSticker;
  messageId?: string;
  onClose: () => void;
  onOpenPack: (packId: string) => void;
}

type PackState =
  | { kind: "loading" }
  | { kind: "ready"; pack: StickerPack }
  | { kind: "gone" }
  | { kind: "failed" };

const isGone = (err: unknown) =>
  err instanceof ApiError && (err.status === 404 || err.code === "STICKER_PACK_NOT_FOUND");

const StickerViewer: React.FC<StickerViewerProps> = ({ sticker, messageId, onClose, onOpenPack }) => {
  const { t } = useTranslation("stickers");
  const userId = useAuthStore((s) => s.user?.id ?? null);
  const myPacks = useStickerStore((s) => s.packs);
  const packQuery = useQuery(
    {
      queryKey: queryKeys.stickerPack(sticker.packId),
      queryFn: () => stickersAPI.pack(sticker.packId),
      retry: retryUnless(isGone),
    },
    queryClient,
  );
  const state: PackState =
    packQuery.isError && isGone(packQuery.error)
      ? { kind: "gone" }
      : packQuery.data
        ? { kind: "ready", pack: packQuery.data }
        : packQuery.isError
          ? { kind: "failed" }
          : { kind: "loading" };
  const [busy, setBusy] = React.useState(false);
  const [error, setError] = React.useState<string | null>(null);
  React.useEffect(() => {
    if (userId && useStickerStore.getState().userId !== userId) void useStickerStore.getState().load(userId);
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
        <StickerImage
          id={sticker.id}
          emoji={sticker.emoji}
          size={VIEWER_SIZE}
          width={sticker.width}
          height={sticker.height}
          animated={sticker.animated}
        />
        <div className="sticker-viewer-info">
          <span className="sticker-viewer-emoji" aria-hidden="true">
            {sticker.emoji}
          </span>
          <span className="sticker-viewer-pack">
            {state.kind === "loading"
              ? t("loading")
              : pack
                ? pack.name
                : state.kind === "gone"
                  ? t("packGone")
                  : t("loadFailed")}
          </span>
        </div>
        {state.kind === "failed" && (
          <div className="sticker-viewer-actions">
            <button type="button" className="s-btn secondary" onClick={() => void packQuery.refetch()}>
              {t("retry")}
            </button>
          </div>
        )}
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
                  openReport({ type: "stickerPack", id: pack.id, name: pack.name, messageId });
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
