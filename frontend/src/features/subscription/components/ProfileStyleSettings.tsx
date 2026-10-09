import React from "react";
import { useImageCrop } from "../../../shared/hooks/useImageCrop";
import { useTranslation } from "react-i18next";

import { useAuthStore } from "../../auth/store/authStore";
import { userAPI } from "../../social/api/user";
import { ApiError } from "../../../shared/lib/apiError";
import { bustAvatar, useAvatarUrl } from "../../../shared/lib/avatarCache";
import { colorForId } from "../../../shared/lib/avatar";
import Icon from "../../../shared/ui/Icon";
import { BADGE_STYLES, badgeStyleTier, hasTier } from "../lib/plans";
import type { BadgeStyle } from "../lib/plans";
import { useEntitlementsStore } from "../store/entitlementsStore";
import ProBadge from "./ProBadge";
import TrialOffer from "./TrialOffer";
import { whilePlanSyncs } from "../lib/planSync";
import { useActiveEmojiStatus } from "../lib/emojiStatus";
import StickerImage from "../../stickers/components/StickerImage";
import { installedPacks, useStickerStore } from "../../stickers/store/stickerStore";

const MAX_BANNER_BYTES = 8 * 1024 * 1024;
const BANNER_TYPES = ["image/png", "image/jpeg", "image/webp"];

const ProfileStyleSettings: React.FC = () => {
  const { t } = useTranslation("subscription");
  const { t: tCommon } = useTranslation("common");
  const { requestCrop, cropModal } = useImageCrop();
  const userId = useAuthStore((s) => s.user?.id ?? null);
  const tier = useEntitlementsStore((s) => s.tier);
  const badgeStyle = useEntitlementsStore((s) => s.badgeStyle);
  const memberNumber = useEntitlementsStore((s) => s.memberNumber);
  const setBadgeStyle = useEntitlementsStore((s) => s.setBadgeStyle);
  const bannerUrl = useAvatarUrl("banner", tier === "free" ? null : userId);
  const fileRef = React.useRef<HTMLInputElement>(null);
  const [busy, setBusy] = React.useState(false);
  const [error, setError] = React.useState<string | null>(null);
  const locked = tier === "free";
  const shownTier = locked ? "pro" : tier;
  const emojiStatus = useActiveEmojiStatus(useEntitlementsStore((s) => s.emojiStatus));
  const stickerPacks = useStickerStore((s) => s.packs);
  const stickersLoaded = useStickerStore((s) => s.loaded);
  const stickersOwner = useStickerStore((s) => s.userId);
  const [statusBusy, setStatusBusy] = React.useState(false);
  const statusStickers = React.useMemo(
    () => installedPacks(stickerPacks).flatMap((pack) => pack.stickers),
    [stickerPacks],
  );

  React.useEffect(() => {
    if (locked || !userId) return;
    if (!stickersLoaded || stickersOwner !== userId) void useStickerStore.getState().load(userId);
  }, [locked, userId, stickersLoaded, stickersOwner]);

  const describe = (err: unknown) => {
    const apiErr = err instanceof ApiError ? err : null;
    switch (apiErr?.code) {
      case "PRO_REQUIRED":
        return t("needs.pro");
      case "PREMIUM_REQUIRED":
        return t("needs.premium");
      case "BANNER_TOO_LARGE":
        return t("banner.tooLarge");
      case "BANNER_TOO_SMALL":
        return t("banner.tooSmall");
      case "BANNER_CROP_INVALID":
      case "BANNER_TYPE_NOT_ALLOWED":
      case "BANNER_INVALID_IMAGE":
        return t("banner.badType");
      case "IMAGE_CONTENT_REJECTED":
        return t("banner.rejected");
      case "STICKER_NOT_FOUND":
        return t("emojiStatus.gone");
      case "STICKERS_UNAVAILABLE":
        return t("emojiStatus.failed");
      default:
        return apiErr?.message || t("banner.failed");
    }
  };

  const runStatus = async (action: () => Promise<void>) => {
    if (locked || statusBusy) return;
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

  const pickStyle = (style: BadgeStyle) => {
    if (locked || (style === badgeStyle && !emojiStatus)) return;
    setError(null);
    if (style !== badgeStyle) setBadgeStyle(style).catch((err) => setError(describe(err)));
    if (emojiStatus) void runStatus(() => useEntitlementsStore.getState().clearEmojiStatus());
  };

  const upload = async (file: File | undefined) => {
    if (!file || locked || !userId) return;
    setError(null);
    if (!BANNER_TYPES.includes(file.type)) {
      setError(t("banner.badType"));
      return;
    }
    if (file.size > MAX_BANNER_BYTES) {
      setError(t("banner.tooLarge"));
      return;
    }
    setBusy(true);
    try {
      await whilePlanSyncs(() => userAPI.uploadBanner(file));
      bustAvatar("banner", userId);
    } catch (err) {
      setError(describe(err));
    } finally {
      setBusy(false);
      if (fileRef.current) fileRef.current.value = "";
    }
  };

  const remove = async () => {
    if (locked || !userId) return;
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
    <div className={"s-section profile-style" + (locked ? " locked" : "")}>
      <h2>
        {t("profileStyle.title")}
        <ProBadge tier="pro" badgeStyle="tint" size="sm" className="profile-style-tag" />
      </h2>
      {locked && (
        <p className="profile-style-note">
          <Icon name="lock" size={12} />
          {t("profileStyle.locked")}
        </p>
      )}
      {locked && <TrialOffer compact />}

      <div className="profile-style-block">
        <div className="profile-style-label">{t("banner.title")}</div>
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
            disabled={locked || busy}
            onChange={(e) => {
              const picked = e.target.files?.[0];
              e.target.value = "";
              if (!picked || locked) return;
              void requestCrop(picked, {
                aspect: 3,
                shape: "rect",
                title: tCommon("cropBanner"),
              }).then((cropped) => {
                if (cropped) void upload(cropped);
              });
            }}
          />
          <button
            type="button"
            className="ssv-btn"
            disabled={locked || busy}
            onClick={() => fileRef.current?.click()}
          >
            {busy ? t("banner.uploading") : t("banner.upload")}
          </button>
          <button
            type="button"
            className="ssv-btn"
            disabled={locked || busy || !bannerUrl}
            onClick={() => void remove()}
          >
            {t("banner.remove")}
          </button>
        </div>
        <div className="profile-style-hint">{t("banner.hint")}</div>
      </div>

      <div className="profile-style-block">
        <div className="profile-style-label">{t("badge.title")}</div>
        <div className="badge-style-grid" role="radiogroup" aria-label={t("badge.title")}>
          {BADGE_STYLES.map((style) => {
            const required = badgeStyleTier(style);
            const allowed = hasTier(tier, required);
            const previewTier = hasTier(shownTier, required) ? shownTier : required;
            return (
              <button
                key={style}
                type="button"
                role="radio"
                aria-checked={allowed && badgeStyle === style && !emojiStatus}
                className={
                  "badge-style-option" +
                  (allowed && badgeStyle === style && !emojiStatus ? " on" : "") +
                  (allowed ? "" : " locked")
                }
                disabled={!allowed}
                title={allowed ? undefined : t(`needs.${required}`)}
                onClick={() => pickStyle(style)}
              >
                <ProBadge tier={previewTier} badgeStyle={style} size="md" memberNumber={locked ? 1 : memberNumber} />
                <span className="badge-style-name">{t(`badge.styles.${style}`)}</span>
              </button>
            );
          })}
        </div>
        <div className="profile-style-sublabel">{t("emojiStatus.title")}</div>
        <div className="emoji-status-current">
          {emojiStatus ? (
            <>
              <StickerImage id={emojiStatus.stickerId} emoji="" size={32} animated />
              <button
                type="button"
                className="s-btn secondary"
                disabled={locked || statusBusy}
                onClick={() => void runStatus(() => useEntitlementsStore.getState().clearEmojiStatus())}
              >
                {t("emojiStatus.remove")}
              </button>
            </>
          ) : (
            <span className="profile-style-hint">{t("emojiStatus.none")}</span>
          )}
        </div>
        {statusStickers.length > 0 ? (
          <div className="emoji-status-grid">
            {statusStickers.map((sticker) => (
              <button
                key={sticker.id}
                type="button"
                className={"emoji-status-option" + (emojiStatus?.stickerId === sticker.id ? " on" : "")}
                aria-label={sticker.emoji || t("emojiStatus.title")}
                disabled={locked || statusBusy}
                onClick={() =>
                  void runStatus(() => useEntitlementsStore.getState().setEmojiStatus(sticker.id))
                }
              >
                <StickerImage id={sticker.id} emoji={sticker.emoji} size={36} animated={sticker.animated} />
              </button>
            ))}
          </div>
        ) : (
          <div className="profile-style-hint">{t("emojiStatus.noStickers")}</div>
        )}
      </div>

      {error && <div className="profile-style-error">{error}</div>}
      {cropModal}
    </div>
  );
};

export default ProfileStyleSettings;
