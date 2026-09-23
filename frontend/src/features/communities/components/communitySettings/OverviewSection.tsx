import React from "react";
import i18n from "../../../../i18n";
import { useImageCrop } from "../../../../shared/hooks/useImageCrop";
import { useTranslation } from "react-i18next";
import Icon from "../../../../shared/ui/Icon";
import { communityAPI } from "../../api/community";
import { useCommunityStore } from "../../store/communityStore";
import { ApiError } from "../../../../shared/lib/apiError";
import { colorForId } from "../../../../shared/lib/avatar";
import { CommunityLogo } from "../../../../shared/ui/Avatar";
import { bustAvatar, useAvatarUrl } from "../../../../shared/lib/avatarCache";
import type { Community } from "../../../../shared/types/community";


interface OverviewSectionProps {
  community: Community;
  canManageCommunity: boolean;
}

export const OverviewSection: React.FC<OverviewSectionProps> = ({
  community,
  canManageCommunity,
}) => {
  const { t } = useTranslation(["communitySettings", "common", "modals"]);
  const [name, setName] = React.useState(community.name);
  const [description, setDescription] = React.useState(community.description ?? "");
  const [saving, setSaving] = React.useState(false);
  const [saveError, setSaveError] = React.useState<string | null>(null);
  const [saved, setSaved] = React.useState(false);
  const patchCommunity = useCommunityStore((s) => s.patchCommunity);

  const logoInputRef = React.useRef<HTMLInputElement>(null);
  const { requestCrop, cropModal } = useImageCrop();
  const [logoBusy, setLogoBusy] = React.useState(false);
  const logoUrl = useAvatarUrl("community", community.id);

  const handleLogoFile = async (file: File) => {
    setLogoBusy(true);
    setSaveError(null);
    try {
      await communityAPI.uploadLogo(community.id, file);
      bustAvatar("community", community.id);
    } catch (err) {
      const apiErr = err instanceof ApiError ? err : null;
      if (apiErr?.code === "IMAGE_CONTENT_REJECTED") {
        setSaveError(t("logoRejected"));
      } else {
        setSaveError(apiErr?.message || t("logoError"));
      }
    } finally {
      setLogoBusy(false);
    }
  };

  const handleLogoRemove = async () => {
    setLogoBusy(true);
    setSaveError(null);
    try {
      await communityAPI.deleteLogo(community.id);
      bustAvatar("community", community.id);
    } catch (err) {
      const apiErr = err instanceof ApiError ? err : null;
      setSaveError(apiErr?.message || t("logoError"));
    } finally {
      setLogoBusy(false);
    }
  };

  const handleSave = async () => {
    const trimmed = name.trim();
    if (trimmed.length < 3) {
      setSaveError(t("nameTooShort"));
      return;
    }
    setSaving(true);
    setSaveError(null);
    try {
      await communityAPI.updateCommunity(community.id, {
        name: trimmed,
        description: description.trim() || undefined,
      });
      patchCommunity(community.id, {
        name: trimmed,
        description: description.trim() || null,
      });
      setSaved(true);
      setTimeout(() => setSaved(false), 1600);
    } catch (err) {
      const apiErr = err instanceof ApiError ? err : null;
      if (apiErr?.code === "COMMUNITY_NAME_NOT_ALLOWED") {
        setSaveError(t("nameNotAllowed"));
      } else if (apiErr?.code === "COMMUNITY_DESCRIPTION_NOT_ALLOWED") {
        setSaveError(t("descriptionNotAllowed"));
      } else if (apiErr?.code === "MODERATION_UNAVAILABLE") {
        setSaveError(t("moderationUnavailable"));
      } else {
        setSaveError(apiErr?.message || t("common:error"));
      }
    } finally {
      setSaving(false);
    }
  };

  const accent = community.accentColor ?? colorForId(community.id);

  return (
            <>
              {cropModal}
              <h1>{t("nav.overview")}</h1>
              <p className="lead">{t("overviewLead")}</p>
              <div className="ssv-hero">
                <input
                  ref={logoInputRef}
                  type="file"
                  accept="image/png,image/jpeg,image/webp"
                  hidden
                  onChange={(e) => {
                    const picked = e.target.files?.[0];
                    e.target.value = "";
                    if (!picked) return;
                    void requestCrop(picked, {
                      aspect: 1,
                      shape: "circle",
                      title: i18n.t("cropLogo", { ns: "common" }),
                    }).then((cropped) => {
                      if (cropped) void handleLogoFile(cropped);
                    });
                  }}
                />
                <CommunityLogo
                  communityId={community.id}
                  name={community.name}
                  className="ssv-hero-ava"
                  style={{
                    background: accent,
                    cursor: canManageCommunity ? "pointer" : undefined,
                  }}
                  title={canManageCommunity ? t("uploadLogo") : undefined}
                >
                  {canManageCommunity && (
                    <span
                      className="edit-badge ssv-logo-badge"
                      role="button"
                      tabIndex={0}
                      onClick={() => logoInputRef.current?.click()}
                      onKeyDown={(e) => {
                        if (e.key === "Enter" || e.key === " ") {
                          logoInputRef.current?.click();
                        }
                      }}
                    >
                      <Icon name="camera" size={12} />
                    </span>
                  )}
                </CommunityLogo>
                {canManageCommunity && (
                  <div className="ssv-logo-actions">
                    <button
                      type="button"
                      className="ssv-btn"
                      disabled={logoBusy}
                      onClick={() => logoInputRef.current?.click()}
                    >
                      {t("uploadLogo")}
                    </button>
                    {logoUrl && (
                      <button
                        type="button"
                        className="ssv-btn danger"
                        disabled={logoBusy}
                        onClick={() => void handleLogoRemove()}
                      >
                        {t("removeLogo")}
                      </button>
                    )}
                  </div>
                )}
              </div>
              <label className="ssv-label">{t("communityName")}</label>
              <input
                className="ssv-input"
                value={name}
                maxLength={20}
                disabled={!canManageCommunity}
                onChange={(e) => {
                  setName(e.target.value);
                  setSaveError(null);
                }}
              />
              <label className="ssv-label">{t("description")}</label>
              <textarea
                className="ssv-input ssv-textarea"
                value={description}
                maxLength={300}
                disabled={!canManageCommunity}
                onChange={(e) => setDescription(e.target.value)}
              />
              <div className="ssv-hint">{t("descriptionHint")}</div>
              {saveError && <div className="ssv-error">{saveError}</div>}
              {canManageCommunity && (
                <div className="ssv-actions">
                  <button
                    type="button"
                    className={"ssv-btn primary" + (saved ? " saved" : "")}
                    disabled={saving}
                    onClick={handleSave}
                  >
                    {saved ? t("common:saved") : t("common:save")}
                  </button>
                  <button
                    type="button"
                    className="ssv-btn"
                    onClick={() => {
                      setName(community.name);
                      setDescription(community.description ?? "");
                      setSaveError(null);
                    }}
                  >
                    {t("common:cancel")}
                  </button>
                </div>
              )}
            </>
  );
};

export default OverviewSection;
