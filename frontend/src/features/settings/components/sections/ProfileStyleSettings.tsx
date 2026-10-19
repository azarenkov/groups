import React from "react";
import { useTranslation } from "react-i18next";

import { useImageCrop } from "../../../../shared/hooks/useImageCrop";
import { ApiError } from "../../../../shared/lib/apiError";
import { colorForId } from "../../../../shared/lib/avatar";
import { bustAvatar, useAvatarUrl } from "../../../../shared/lib/avatarCache";
import { useAuthStore } from "../../../auth/store/authStore";
import { userAPI } from "../../../social/api/user";
import { useActiveEmojiStatus } from "../../../social/lib/emojiStatus";
import { useEmojiStatusStore } from "../../../social/store/emojiStatusStore";
import StickerImage from "../../../stickers/components/StickerImage";
import { installedPacks, packCover, useStickerStore } from "../../../stickers/store/stickerStore";

const MAX_BANNER_BYTES = 8 * 1024 * 1024;
const BANNER_TYPES = ["image/png", "image/jpeg", "image/webp"];

const ProfileStyleSettings: React.FC = () => {
  const { t } = useTranslation("settings");
  const { t: tCommon } = useTranslation("common");
  const { requestCrop, cropModal } = useImageCrop();
  const userId = useAuthStore((s) => s.user?.id ?? null);
  const bannerUrl = useAvatarUrl("banner", userId);
  const fileRef = React.useRef<HTMLInputElement>(null);
  const [busy, setBusy] = React.useState(false);
  const [error, setError] = React.useState<string | null>(null);
  const emojiStatus = useActiveEmojiStatus(useEmojiStatusStore((s) => (userId ? s.byUser[userId] : null)));
  const stickerPacks = useStickerStore((s) => s.packs);
  const stickersLoaded = useStickerStore((s) => s.loaded);
  const stickersOwner = useStickerStore((s) => s.userId);
  const [statusBusy, setStatusBusy] = React.useState(false);
  const statusPacks = React.useMemo(
    () => stickerPacks.filter((pack) => pack.ownerId === userId || installedPacks([pack]).length > 0),
    [stickerPacks, userId],
  );
  const [chosenPackId, setChosenPackId] = React.useState<string | null>(null);
  const statusPack =
    statusPacks.find((pack) => pack.id === chosenPackId) ??
    statusPacks.find((pack) => pack.stickers.some((sticker) => sticker.id === emojiStatus?.stickerId)) ??
    statusPacks[0] ??
    null;

  React.useEffect(() => {
    if (!userId) return;
    if (!stickersLoaded || stickersOwner !== userId) void useStickerStore.getState().load(userId);
  }, [userId, stickersLoaded, stickersOwner]);

  const describe = (err: unknown) => {
    const apiErr = err instanceof ApiError ? err : null;
    switch (apiErr?.code) {
      case "BANNER_TOO_LARGE":
        return t("profileStyle.banner.tooLarge");
      case "BANNER_TOO_SMALL":
        return t("profileStyle.banner.tooSmall");
      case "BANNER_CROP_INVALID":
      case "BANNER_TYPE_NOT_ALLOWED":
      case "BANNER_INVALID_IMAGE":
        return t("profileStyle.banner.badType");
      case "IMAGE_CONTENT_REJECTED":
        return t("profileStyle.banner.rejected");
      case "STICKER_NOT_FOUND":
        return t("profileStyle.emojiStatus.gone");
      case "STICKERS_UNAVAILABLE":
        return t("profileStyle.emojiStatus.failed");
      default:
        return apiErr?.message || t("profileStyle.banner.failed");
    }
  };

  const runStatus = async (action: () => Promise<void>) => {
    if (statusBusy) return;
    setStatusBusy(true);
    setError(null);
    try {
      await action();
    } catch (err) {
      setError(describe(err));
    } finally {
      setStatusBusy(false);
    }
  };

  const upload = async (file: File | undefined) => {
    if (!file || !userId) return;
    setError(null);
    if (!BANNER_TYPES.includes(file.type)) {
      setError(t("profileStyle.banner.badType"));
      return;
    }
    if (file.size > MAX_BANNER_BYTES) {
      setError(t("profileStyle.banner.tooLarge"));
      return;
    }
    setBusy(true);
    try {
      await userAPI.uploadBanner(file);
      bustAvatar("banner", userId);
    } catch (err) {
      setError(describe(err));
    } finally {
      setBusy(false);
      if (fileRef.current) fileRef.current.value = "";
    }
  };

  const remove = async () => {
    if (!userId) return;
    setError(null);
    setBusy(true);
    try {
      await userAPI.deleteBanner();
      bustAvatar("banner", userId);
    } catch (err) {
      setError(describe(err));
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="s-section profile-style">
      <h2>{t("profileStyle.title")}</h2>

      <div className="profile-style-block">
        <div className="profile-style-label">{t("profileStyle.banner.title")}</div>
        <div
          className="profile-style-banner"
          style={{
            background: bannerUrl
              ? `center / cover no-repeat url(${bannerUrl})`
              : `color-mix(in srgb, ${colorForId(userId ?? "")} 28%, var(--surface-raised))`,
          }}
        />
        <div className="profile-style-actions">
          <input
            ref={fileRef}
            type="file"
            accept={BANNER_TYPES.join(",")}
            hidden
            disabled={busy}
            onChange={(e) => {
              const picked = e.target.files?.[0];
              e.target.value = "";
              if (!picked) return;
              void requestCrop(picked, {
                aspect: 3,
                shape: "rect",
                title: tCommon("cropBanner"),
              }).then((cropped) => {
                if (cropped) void upload(cropped);
              });
            }}
          />
          <button type="button" className="ssv-btn" disabled={busy} onClick={() => fileRef.current?.click()}>
            {busy ? t("profileStyle.banner.uploading") : t("profileStyle.banner.upload")}
          </button>
          <button type="button" className="ssv-btn" disabled={busy || !bannerUrl} onClick={() => void remove()}>
            {t("profileStyle.banner.remove")}
          </button>
        </div>
        <div className="profile-style-hint">{t("profileStyle.banner.hint")}</div>
      </div>

      <div className="profile-style-block">
        <div className="profile-style-label">{t("profileStyle.emojiStatus.title")}</div>
        <div className="emoji-status-current">
          {emojiStatus ? (
            <>
              <StickerImage id={emojiStatus.stickerId} emoji="" size={32} animated />
              <button
                type="button"
                className="s-btn secondary"
                disabled={statusBusy}
                onClick={() => void runStatus(() => useEmojiStatusStore.getState().clearMine())}
              >
                {t("profileStyle.emojiStatus.remove")}
              </button>
            </>
          ) : (
            <span className="profile-style-hint">{t("profileStyle.emojiStatus.none")}</span>
          )}
        </div>
        {statusPack ? (
          <>
            {statusPacks.length > 1 && (
              <div className="emoji-status-packs" role="tablist" aria-label={t("profileStyle.emojiStatus.packs")}>
                {statusPacks.map((pack) => {
                  const cover = packCover(pack);
                  return (
                    <button
                      key={pack.id}
                      type="button"
                      role="tab"
                      aria-selected={pack.id === statusPack.id}
                      title={pack.name}
                      aria-label={pack.name}
                      className={"emoji-status-pack" + (pack.id === statusPack.id ? " on" : "")}
                      onClick={() => setChosenPackId(pack.id)}
                    >
                      {cover ? (
                        <StickerImage id={cover.id} emoji={cover.emoji} size={22} />
                      ) : (
                        <span>{pack.name.slice(0, 1)}</span>
                      )}
                    </button>
                  );
                })}
              </div>
            )}
            <div className="emoji-status-grid">
              {statusPack.stickers.map((sticker) => (
                <button
                  key={sticker.id}
                  type="button"
                  className={"emoji-status-option" + (emojiStatus?.stickerId === sticker.id ? " on" : "")}
                  aria-label={sticker.emoji || t("profileStyle.emojiStatus.title")}
                  disabled={statusBusy}
                  onClick={() => void runStatus(() => useEmojiStatusStore.getState().setMine(sticker.id))}
                >
                  <StickerImage id={sticker.id} emoji={sticker.emoji} size={36} animated={sticker.animated} />
                </button>
              ))}
            </div>
          </>
        ) : (
          <div className="profile-style-hint">{t("profileStyle.emojiStatus.noStickers")}</div>
        )}
      </div>

      {error && <div className="s-error">{error}</div>}
      {cropModal}
    </div>
  );
};

export default ProfileStyleSettings;
