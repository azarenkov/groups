import React from "react";
import { useTranslation } from "react-i18next";
import Icon from "../../../../shared/ui/Icon";
import { serverAPI } from "../../api/server";
import { useServerStore } from "../../store/serverStore";
import { ApiError } from "../../../../shared/lib/apiError";
import { colorForId } from "../../../../shared/lib/avatar";
import { ServerLogo } from "../../../../shared/ui/Avatar";
import { bustAvatar, useAvatarUrl } from "../../../../shared/lib/avatarCache";
import type { Server } from "../../../../shared/types/server";


interface OverviewSectionProps {
  server: Server;
  canManageServer: boolean;
}

export const OverviewSection: React.FC<OverviewSectionProps> = ({
  server,
  canManageServer,
}) => {
  const { t } = useTranslation(["serverSettings", "common", "modals"]);
  const [name, setName] = React.useState(server.name);
  const [description, setDescription] = React.useState(server.description ?? "");
  const [saving, setSaving] = React.useState(false);
  const [saveError, setSaveError] = React.useState<string | null>(null);
  const [saved, setSaved] = React.useState(false);
  const patchServer = useServerStore((s) => s.patchServer);

  const logoInputRef = React.useRef<HTMLInputElement>(null);
  const [logoBusy, setLogoBusy] = React.useState(false);
  const logoUrl = useAvatarUrl("server", server.id);

  const handleLogoFile = async (file: File) => {
    setLogoBusy(true);
    setSaveError(null);
    try {
      await serverAPI.uploadLogo(server.id, file);
      bustAvatar("server", server.id);
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
      await serverAPI.deleteLogo(server.id);
      bustAvatar("server", server.id);
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
      await serverAPI.updateServer(server.id, {
        name: trimmed,
        description: description.trim() || undefined,
      });
      patchServer(server.id, {
        name: trimmed,
        description: description.trim() || null,
      });
      setSaved(true);
      setTimeout(() => setSaved(false), 1600);
    } catch (err) {
      const apiErr = err instanceof ApiError ? err : null;
      if (apiErr?.code === "SERVER_NAME_NOT_ALLOWED") {
        setSaveError(t("nameNotAllowed"));
      } else if (apiErr?.code === "SERVER_DESCRIPTION_NOT_ALLOWED") {
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

  const accent = server.accentColor ?? colorForId(server.id);

  return (
            <>
              <h1>{t("nav.overview")}</h1>
              <p className="lead">{t("overviewLead")}</p>
              <div className="ssv-hero">
                <input
                  ref={logoInputRef}
                  type="file"
                  accept="image/png,image/jpeg,image/webp"
                  hidden
                  onChange={(e) => {
                    const f = e.target.files?.[0];
                    if (f) void handleLogoFile(f);
                    e.target.value = "";
                  }}
                />
                <ServerLogo
                  serverId={server.id}
                  name={server.name}
                  className="ssv-hero-ava"
                  style={{
                    background: accent,
                    cursor: canManageServer ? "pointer" : undefined,
                  }}
                  title={canManageServer ? t("uploadLogo") : undefined}
                >
                  {canManageServer && (
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
                </ServerLogo>
                {canManageServer && (
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
              <label className="ssv-label">{t("serverName")}</label>
              <input
                className="ssv-input"
                value={name}
                maxLength={20}
                disabled={!canManageServer}
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
                disabled={!canManageServer}
                onChange={(e) => setDescription(e.target.value)}
              />
              <div className="ssv-hint">{t("descriptionHint")}</div>
              {saveError && <div className="ssv-error">{saveError}</div>}
              {canManageServer && (
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
                      setName(server.name);
                      setDescription(server.description ?? "");
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
