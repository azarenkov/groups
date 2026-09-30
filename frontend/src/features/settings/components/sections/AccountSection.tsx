import React from "react";
import i18n from "../../../../i18n";
import { useEntitlementsStore } from "../../../subscription/store/entitlementsStore";
import UserBadge from "../../../subscription/components/UserBadge";
import { useImageCrop } from "../../../../shared/hooks/useImageCrop";
import { useTranslation } from "react-i18next";
import Icon from "../../../../shared/ui/Icon";
import VideoAvatarModal from "../../../../shared/ui/VideoAvatarModal";
import { whilePlanSyncs } from "../../../subscription/lib/planSync";
import { useAuthStore } from "../../../auth/store/authStore";
import { userAPI } from "../../../social/api/user";
import { ApiError } from "../../../../shared/lib/apiError";
import { UserAvatar } from "../../../../shared/ui/Avatar";
import { bustAvatar, useAvatarUrl } from "../../../../shared/lib/avatarCache";
import { validateUsername, type UsernameIssue } from "../../../../shared/lib/username";
import { ChangeEmailDialog, ChangePasswordDialog, DangerPasswordDialog } from "../AccountModals";
import { profileVersion, useUserProfileStore } from "../../../social/store/userProfileStore";

import { SettingsRow, Switch } from "../controls";

export function AccountSection({ onAccountGone }: { onAccountGone: () => void }) {
  const { t } = useTranslation("settings");
  const { t: tAuth } = useTranslation("auth");
  const user = useAuthStore((s) => s.user);
  const [dialog, setDialog] = React.useState<null | "email" | "password" | "disable" | "delete">(null);
  const [bio, setBio] = React.useState(user?.bio ?? "");
  const [bioBusy, setBioBusy] = React.useState(false);
  const [bioError, setBioError] = React.useState<string | null>(null);
  const [bioEditing, setBioEditing] = React.useState(false);
  const [activityBusy, setActivityBusy] = React.useState(false);
  const [gameActivityBusy, setGameActivityBusy] = React.useState(false);
  const bioBaseline = React.useRef(user?.bio ?? "");
  React.useEffect(() => {
    const remote = user?.bio ?? "";
    setBio((draft) => {
      const clean = draft.trim() === bioBaseline.current.trim();
      bioBaseline.current = remote;
      return clean ? remote : draft;
    });
  }, [user?.bio]);
  const bioDirty = bio.trim() !== (user?.bio ?? "");
  const profileQueue = React.useRef<Promise<void>>(Promise.resolve());
  const runProfileUpdate = (patch: { bio?: string; showActivity?: boolean; showGameActivity?: boolean }) => {
    const initiator = useAuthStore.getState().user?.id;
    const next = profileQueue.current.then(async () => {
      if (!initiator || useAuthStore.getState().user?.id !== initiator) return;
      const saved = await userAPI.updateProfile(patch);
      const current = useAuthStore.getState().user;
      if (!current || current.id !== initiator) return;
      const version = profileVersion(saved.profileUpdatedAt);
      const cached = useUserProfileStore.getState().bios[current.id];
      if (cached && cached.at > version) return;
      useUserProfileStore.getState().apply(current.id, saved.bio, version);
      useAuthStore.getState().setUser({ ...current, bio: saved.bio, showActivity: saved.showActivity, showGameActivity: saved.showGameActivity });
    });
    profileQueue.current = next.catch(() => {});
    return next;
  };
  const saveBio = async (): Promise<boolean> => {
    if (!user || bioBusy || !bioDirty) return false;
    setBioBusy(true);
    setBioError(null);
    try {
      await runProfileUpdate({ bio: bio.trim() });
      return true;
    } catch (err) {
      const apiErr = err instanceof ApiError ? err : null;
      if (apiErr?.code === "BIO_NOT_ALLOWED") {
        setBioError(t("account.bioNotAllowed"));
      } else if (apiErr?.code === "MODERATION_UNAVAILABLE") {
        setBioError(t("account.moderationUnavailable"));
      } else {
        setBioError(apiErr?.message || t("account.bioError"));
      }
      return false;
    } finally {
      setBioBusy(false);
    }
  };
  const toggleActivity = async () => {
    if (!user || activityBusy) return;
    const next = !(user.showActivity ?? true);
    setActivityBusy(true);
    try {
      await runProfileUpdate({ showActivity: next });
    } catch (err) {
      console.warn("[account] activity toggle failed:", err);
    } finally {
      setActivityBusy(false);
    }
  };
  const toggleGameActivity = async () => {
    if (!user || gameActivityBusy) return;
    const next = !(user.showGameActivity ?? true);
    setGameActivityBusy(true);
    try {
      await runProfileUpdate({ showGameActivity: next });
    } catch (err) {
      console.warn("[account] game activity toggle failed:", err);
    } finally {
      setGameActivityBusy(false);
    }
  };
  const passwordHint = user?.passwordChangedAt
    ? t("account.passwordChangedOn", { date: new Date(user.passwordChangedAt).toLocaleDateString() })
    : t("account.passwordNeverChanged");
  const emailHint = user?.email
    ? user.emailVerified
      ? t("account.emailVerified", { email: user.email })
      : t("account.emailUnverified", { email: user.email })
    : "";
  const avatarUrl = useAvatarUrl("user", user?.id);
  const fileRef = React.useRef<HTMLInputElement>(null);
  const [busy, setBusy] = React.useState(false);
  const [error, setError] = React.useState<string | null>(null);
  const [nameEditing, setNameEditing] = React.useState(false);
  const [nameValue, setNameValue] = React.useState("");
  const [nameBusy, setNameBusy] = React.useState(false);
  const [nameError, setNameError] = React.useState<string | null>(null);

  const usernameIssueMessage = (issue: UsernameIssue): string => {
    switch (issue) {
      case "required":
        return tAuth("displayNameRequired");
      case "tooShort":
        return tAuth("displayNameMin");
      case "tooLong":
        return tAuth("displayNameMax");
      case "charset":
        return tAuth("displayNameInvalid");
      case "underscoreEdge":
      case "doubleUnderscore":
        return tAuth("displayNameUnderscore");
      case "digitsOnly":
        return tAuth("displayNameDigits");
      case "reserved":
        return tAuth("displayNameReserved");
    }
  };

  const startNameEdit = () => {
    setNameValue(user?.name ?? "");
    setNameError(null);
    setNameEditing(true);
  };

  const cancelNameEdit = () => {
    setNameEditing(false);
    setNameError(null);
  };

  const saveName = async () => {
    if (!user || nameBusy) return;
    const next = nameValue.trim();
    if (next === user.name) {
      cancelNameEdit();
      return;
    }
    const issue = validateUsername(next);
    if (issue !== null) {
      setNameError(usernameIssueMessage(issue));
      return;
    }
    setNameBusy(true);
    setNameError(null);
    try {
      const saved = await userAPI.rename(next);
      const latest = useAuthStore.getState().user;
      if (latest && latest.id === user.id) useAuthStore.getState().setUser({ ...latest, name: saved });
      setNameEditing(false);
    } catch (err) {
      const apiErr = err instanceof ApiError ? err : null;
      if (apiErr?.code === "NAME_TAKEN") {
        setNameError(tAuth("usernameTaken"));
      } else if (apiErr?.code === "NAME_NOT_ALLOWED") {
        setNameError(tAuth("usernameNotAllowed"));
      } else if (apiErr?.code === "NAME_INVALID_FORMAT") {
        setNameError(tAuth("usernameInvalid"));
      } else if (apiErr?.code === "MODERATION_UNAVAILABLE") {
        setNameError(tAuth("usernameModerationUnavailable"));
      } else {
        setNameError(apiErr?.message || t("account.usernameError"));
      }
    } finally {
      setNameBusy(false);
    }
  };

  const pickFile = () => fileRef.current?.click();
  const { requestCrop, cropModal } = useImageCrop();
  const paid = useEntitlementsStore((state) => state.tier !== "free");
  const [videoFile, setVideoFile] = React.useState<File | null>(null);

  const handleFile = async (file: File) => {
    if (!user) return;
    setBusy(true);
    setError(null);
    try {
      await whilePlanSyncs(() => userAPI.uploadAvatar(file));
      bustAvatar("user", user.id);
    } catch (err) {
      const apiErr = err instanceof ApiError ? err : null;
      if (apiErr?.code === "IMAGE_CONTENT_REJECTED") {
        setError(t("account.avatarRejected"));
      } else if (apiErr?.code === "PRO_REQUIRED") {
        setError(i18n.t("needs.pro", { ns: "subscription" }));
      } else {
        setError(apiErr?.message || t("account.avatarError"));
      }
    } finally {
      setBusy(false);
    }
  };

  const handleRemove = async () => {
    if (!user) return;
    setBusy(true);
    setError(null);
    try {
      await userAPI.deleteAvatar();
      bustAvatar("user", user.id);
    } catch (err) {
      const apiErr = err instanceof ApiError ? err : null;
      setError(apiErr?.message || t("account.avatarError"));
    } finally {
      setBusy(false);
    }
  };

  return (
    <>
      <h1>{t("account.title")}</h1>
      <p className="lead">{t("account.lead")}</p>

      <div className="profile-card">
        <input
          ref={fileRef}
          type="file"
          accept={
            paid
              ? "image/png,image/jpeg,image/webp,image/gif,video/mp4,video/webm,video/quicktime"
              : "image/png,image/jpeg,image/webp"
          }
          hidden
          onChange={(e) => {
            const f = e.target.files?.[0];
            e.target.value = "";
            if (!f) return;
            if (paid && (f.type.startsWith("video/") || /\.(mp4|m4v|mov|webm)$/i.test(f.name))) {
              setVideoFile(f);
              return;
            }
            void requestCrop(f, {
              aspect: 1,
              shape: "circle",
              title: i18n.t("cropAvatar", { ns: "common" }),
            }).then((cropped) => {
              if (cropped) void handleFile(cropped);
            });
          }}
        />
        {cropModal}
        {videoFile && (
          <VideoAvatarModal
            file={videoFile}
            onCancel={() => setVideoFile(null)}
            onConfirm={(gif) => {
              setVideoFile(null);
              void handleFile(gif);
            }}
          />
        )}
        <UserAvatar
          userId={user?.id}
          name={user?.name ?? "?"}
          className="ava-lg"
          title={t("account.changeAvatar")}
          style={{ cursor: "pointer" }}
        >
          <span
            className="edit-badge"
            role="button"
            tabIndex={0}
            onClick={pickFile}
            onKeyDown={(e) => {
              if (e.key === "Enter" || e.key === " ") pickFile();
            }}
          >
            <Icon name="camera" size={12} />
          </span>
        </UserAvatar>
        <div className="pc-info">
          <div className="pc-name">
            <span className="pc-name-text">{user?.name ?? t("account.username")}</span>
            <UserBadge userId={user?.id} size="sm" />
          </div>
          <div className="pc-handle">{user?.email ?? ""}</div>
        </div>
        <button
          className="s-btn secondary"
          disabled={busy}
          onClick={pickFile}
        >
          {t("account.uploadAvatar")}
        </button>
        {avatarUrl && (
          <button
            className="s-btn danger"
            disabled={busy}
            onClick={() => void handleRemove()}
          >
            {t("account.removeAvatar")}
          </button>
        )}
      </div>
      {error && <div className="s-error">{error}</div>}

      <div className="s-section">
        <h2>{t("account.accountInformation")}</h2>
        <SettingsRow
          label={t("account.username")}
          hint={user?.name ? `@${user.name}` : t("account.usernameHint")}
          hintClassName={user?.name ? "s-data" : undefined}
        >
          {nameEditing ? (
            <>
              <input
                className="s-input"
                style={{ minWidth: 160 }}
                autoFocus
                value={nameValue}
                disabled={nameBusy}
                onChange={(e) => {
                  setNameValue(e.target.value);
                  setNameError(null);
                }}
                onKeyDown={(e) => {
                  if (e.key === "Enter") void saveName();
                  if (e.key === "Escape") cancelNameEdit();
                }}
              />
              <button
                className="s-btn primary"
                disabled={nameBusy}
                onClick={() => void saveName()}
              >
                {t("actions.save")}
              </button>
              <button
                className="s-btn secondary"
                disabled={nameBusy}
                onClick={cancelNameEdit}
              >
                {t("actions.cancel")}
              </button>
            </>
          ) : (
            <button className="s-btn secondary" onClick={startNameEdit}>
              {t("actions.edit")}
            </button>
          )}
        </SettingsRow>
        {nameError && <div className="s-error">{nameError}</div>}
        <SettingsRow label={t("account.email")} hint={emailHint} hintClassName={user?.email ? "s-data" : undefined}>
          <button className="s-btn secondary" onClick={() => setDialog("email")}>{t("actions.edit")}</button>
        </SettingsRow>
      </div>

      <div className="s-section">
        <h2>{t("account.aboutSection")}</h2>
        {bioEditing ? (
          <div className="s-field-row">
            <div className="field-lbl">{t("account.about")}</div>
            <textarea
              className="s-input"
              autoFocus
              placeholder={t("account.aboutPlaceholder")}
              value={bio}
              disabled={bioBusy}
              onChange={(e) => {
                const next = Array.from(e.target.value).slice(0, 190).join("");
                setBio(next);
                setBioError(null);
              }}
              onKeyDown={(e) => {
                if (e.key === "Escape") {
                  setBio(user?.bio ?? "");
                  setBioEditing(false);
                }
              }}
            />
            <div className="s-field-foot">
              <span className="s-field-count">{Array.from(bio).length}/190</span>
              <span className="s-field-actions">
                <button
                  className="s-btn secondary"
                  disabled={bioBusy}
                  onClick={() => {
                    setBio(user?.bio ?? "");
                    setBioError(null);
                    setBioEditing(false);
                  }}
                >
                  {t("actions.cancel")}
                </button>
                <button
                  className="s-btn primary"
                  disabled={bioBusy || !bioDirty}
                  onClick={() => void saveBio().then((ok) => ok && setBioEditing(false))}
                >
                  {t("actions.save")}
                </button>
              </span>
            </div>
            {bioError && <div className="s-error">{bioError}</div>}
          </div>
        ) : (
          <SettingsRow
            label={t("account.about")}
            hint={user?.bio ? user.bio : t("account.aboutEmpty")}
            hintClassName={user?.bio ? "s-data" : undefined}
          >
            <button className="s-btn secondary" onClick={() => setBioEditing(true)}>
              {user?.bio ? t("actions.edit") : t("actions.add")}
            </button>
          </SettingsRow>
        )}
        <SettingsRow label={t("account.showActivity")} hint={t("account.showActivityHint")}>
          <Switch on={user?.showActivity ?? true} onClick={() => void toggleActivity()} />
        </SettingsRow>
        <SettingsRow label={t("account.showGameActivity")} hint={t("account.showGameActivityHint")}>
          <Switch on={user?.showGameActivity ?? true} onClick={() => void toggleGameActivity()} />
        </SettingsRow>
      </div>

      <div className="s-section">
        <h2>{t("account.passwordAuth")}</h2>
        <SettingsRow label={t("account.password")} hint={passwordHint}>
          <button className="s-btn secondary" onClick={() => setDialog("password")}>{t("actions.change")}</button>
        </SettingsRow>
      </div>

      <div className="s-section">
        <h2>{t("account.accountRemoval")}</h2>
        <SettingsRow label={t("account.disableAccount")} hint={t("account.disableAccountHint")}>
          <button className="s-btn danger" onClick={() => setDialog("disable")}>{t("actions.disable")}</button>
        </SettingsRow>
        <SettingsRow label={t("account.deleteAccount")} hint={t("account.deleteAccountHint")}>
          <button className="s-btn danger" onClick={() => setDialog("delete")}>{t("actions.delete")}</button>
        </SettingsRow>
      </div>

      {dialog === "email" && <ChangeEmailDialog onClose={() => setDialog(null)} />}
      {dialog === "password" && <ChangePasswordDialog onClose={() => setDialog(null)} />}
      {(dialog === "disable" || dialog === "delete") && (
        <DangerPasswordDialog mode={dialog} onClose={() => setDialog(null)} onDone={onAccountGone} />
      )}
    </>
  );
}
