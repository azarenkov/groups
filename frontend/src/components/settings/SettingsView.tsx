import React from "react";
import { useTranslation } from "react-i18next";
import Icon, { type IconName } from "../rift/Icon";
import { currentLocale, setLocale } from "../../i18n";
import { useAuthStore } from "../../store/authStore";
import { userAPI, type Connection } from "../../api/user";
import { ApiError } from "../../utils/apiError";
import { UserAvatar } from "../rift/Avatar";
import { bustAvatar, useAvatarUrl } from "../../utils/avatarCache";
import { validateUsername, type UsernameIssue } from "../../utils/username";
import { homeAPI, type PrivacySettings } from "../../api/home";
import { ChangeEmailDialog, ChangePasswordDialog, DangerPasswordDialog } from "./AccountModals";
import { profileVersion, useUserProfileStore } from "../../store/userProfileStore";
import { useAppearanceStore } from "../../store/appearanceStore";
import { useVoiceSettingsStore } from "../../store/voiceSettingsStore";
import { getLiveKitService } from "../../services/livekit";

type Theme = "light" | "dark";

interface SettingsViewProps {
  open: boolean;
  onClose: () => void;
  onSignOut: () => void;
}

interface TweaksPanelProps {
  theme: Theme;
  onTheme: (t: Theme) => void;
}

type SectionId =
  | "account"
  | "privacy"
  | "connections"
  | "subscription"
  | "appearance"
  | "voice"
  | "notifications"
  | "shortcuts"
  | "language";

const SETTINGS_GROUPS: { groupKey: "userSettings" | "appSettings"; items: { id: SectionId; icon: IconName }[] }[] = [
  {
    groupKey: "userSettings",
    items: [
      { id: "account", icon: "user" },
      { id: "privacy", icon: "shield" },
      { id: "connections", icon: "link" },
      { id: "subscription", icon: "credit-card" },
    ],
  },
  {
    groupKey: "appSettings",
    items: [
      { id: "appearance", icon: "palette" },
      { id: "voice", icon: "mic" },
      { id: "notifications", icon: "bell" },
      { id: "shortcuts", icon: "keyboard" },
      { id: "language", icon: "globe" },
    ],
  },
];

const ACCENT_OPTIONS = [
  { name: 'Blue',     value: '#0071e3' },
  { name: 'Sage',     value: '#5b7e5e' },
  { name: 'Indigo',   value: '#5b6cd9' },
  { name: 'Coral',    value: '#d97557' },
  { name: 'Amber',    value: '#c89a3d' },
  { name: 'Plum',     value: '#8a5b9e' },
  { name: 'Slate',    value: '#5a6470' },
];

interface SwitchProps {
  disabled?: boolean; on: boolean; onClick: () => void; }

function Switch({ on, onClick, disabled }: SwitchProps) {
  return <button className={'s-switch' + (on ? ' on' : '')} onClick={onClick} disabled={disabled} aria-pressed={on}/>;
}

interface SettingsRowProps {
  label: string;
  hint?: string;
  children: React.ReactNode;
}

function SettingsRow({ label, hint, children }: SettingsRowProps) {
  return (
    <div className="s-row">
      <div className="s-row-text">
        <div className="lbl">{label}</div>
        {hint && <div className="hint">{hint}</div>}
      </div>
      <div className="s-row-control">{children}</div>
    </div>
  );
}


function AccountSection({ onAccountGone }: { onAccountGone: () => void }) {
  const { t } = useTranslation("settings");
  const { t: tAuth } = useTranslation("auth");
  const user = useAuthStore((s) => s.user);
  const [dialog, setDialog] = React.useState<null | "email" | "password" | "disable" | "delete">(null);
  const [bio, setBio] = React.useState(user?.bio ?? "");
  const [bioBusy, setBioBusy] = React.useState(false);
  const [bioError, setBioError] = React.useState<string | null>(null);
  const [bioEditing, setBioEditing] = React.useState(false);
  const [activityBusy, setActivityBusy] = React.useState(false);
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
  const runProfileUpdate = (patch: { bio?: string; showActivity?: boolean }) => {
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
      useAuthStore.getState().setUser({ ...current, bio: saved.bio, showActivity: saved.showActivity });
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
      setBioError(apiErr?.message || t("account.bioError"));
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

  const handleFile = async (file: File) => {
    if (!user) return;
    setBusy(true);
    setError(null);
    try {
      await userAPI.uploadAvatar(file);
      bustAvatar("user", user.id);
    } catch (err) {
      const apiErr = err instanceof ApiError ? err : null;
      if (apiErr?.code === "IMAGE_CONTENT_REJECTED") {
        setError(t("account.avatarRejected"));
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
          accept="image/png,image/jpeg,image/webp"
          hidden
          onChange={(e) => {
            const f = e.target.files?.[0];
            if (f) void handleFile(f);
            e.target.value = "";
          }}
        />
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
          <div className="pc-name">{user?.name ?? t("account.username")}</div>
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
        <SettingsRow label={t("account.email")} hint={emailHint}>
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
          <SettingsRow label={t("account.about")} hint={user?.bio ? user.bio : t("account.aboutEmpty")}>
            <button className="s-btn secondary" onClick={() => setBioEditing(true)}>
              {user?.bio ? t("actions.edit") : t("actions.add")}
            </button>
          </SettingsRow>
        )}
        <SettingsRow label={t("account.showActivity")} hint={t("account.showActivityHint")}>
          <Switch on={user?.showActivity ?? true} onClick={() => void toggleActivity()} />
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

function PrivacySection() {
  const { t } = useTranslation("settings");
  const [settings, setSettings] = React.useState<PrivacySettings | null>(null);
  const [failed, setFailed] = React.useState(false);
  const [busy, setBusy] = React.useState(false);
  React.useEffect(() => {
    let cancelled = false;
    homeAPI
      .privacy()
      .then((s) => {
        if (!cancelled) setSettings(s);
      })
      .catch(() => {
        if (!cancelled) setFailed(true);
      });
    return () => {
      cancelled = true;
    };
  }, []);
  const apply = (patch: Partial<PrivacySettings>) => {
    if (busy) return;
    setBusy(true);
    homeAPI
      .updatePrivacy(patch)
      .then(setSettings)
      .catch((err) => console.warn("[privacy] update failed:", err))
      .finally(() => setBusy(false));
  };
  return (
    <>
      <h1>{t("privacy.title")}</h1>
      <p className="lead">{t("privacy.lead")}</p>

      <div className="s-section">
        <h2>{t("privacy.directMessages")}</h2>
        {failed ? (
          <div className="s-error">{t("privacy.loadFailed")}</div>
        ) : !settings ? (
          <div className="s-row"><span className="loading-spinner" /></div>
        ) : (
          <>
            <SettingsRow label={t("privacy.whoCanDm")} hint={t("privacy.whoCanDmHint")}>
              <div className="s-seg">
                <button disabled={busy} className={settings.dmPolicy === "all" ? "on" : ""} onClick={() => apply({ dmPolicy: "all" })}>{t("privacy.everyone")}</button>
                <button disabled={busy} className={settings.dmPolicy === "friends" ? "on" : ""} onClick={() => apply({ dmPolicy: "friends" })}>{t("privacy.friends")}</button>
                <button disabled={busy} className={settings.dmPolicy === "no_one" ? "on" : ""} onClick={() => apply({ dmPolicy: "no_one" })}>{t("privacy.noOne")}</button>
              </div>
            </SettingsRow>
            <SettingsRow label={t("privacy.allowRequests")} hint={t("privacy.allowRequestsHint")}>
              <Switch on={settings.acceptFriendRequests} disabled={busy} onClick={() => apply({ acceptFriendRequests: !settings.acceptFriendRequests })} />
            </SettingsRow>
          </>
        )}
      </div>
      {!failed && settings && (
        <div className="s-section">
          <h2>{t("privacy.calls")}</h2>
          <SettingsRow label={t("privacy.whoCanCall")} hint={t("privacy.whoCanCallHint")}>
            <div className="s-seg">
              <button disabled={busy} className={settings.callPolicy === "all" ? "on" : ""} onClick={() => apply({ callPolicy: "all" })}>{t("privacy.everyone")}</button>
              <button disabled={busy} className={settings.callPolicy === "friends" ? "on" : ""} onClick={() => apply({ callPolicy: "friends" })}>{t("privacy.friends")}</button>
              <button disabled={busy} className={settings.callPolicy === "no_one" ? "on" : ""} onClick={() => apply({ callPolicy: "no_one" })}>{t("privacy.noOne")}</button>
            </div>
          </SettingsRow>
        </div>
      )}
    </>
  );
}

function ConnectionsSection() {
  const { t } = useTranslation("settings");
  const [connections, setConnections] = React.useState<Connection[] | null>(null);
  const [error, setError] = React.useState<string | null>(null);
  const [busy, setBusy] = React.useState(false);
  const load = React.useCallback(() => {
    userAPI
      .listConnections()
      .then((items) => {
        setConnections(items);
        setError(null);
      })
      .catch(() => setError(t("connections.loadFailed")));
  }, [t]);
  React.useEffect(() => {
    load();
  }, [load]);
  React.useEffect(() => {
    const onReturn = () => {
      setBusy(false);
      load();
    };
    window.addEventListener("focus", onReturn);
    window.addEventListener("pageshow", onReturn);
    return () => {
      window.removeEventListener("focus", onReturn);
      window.removeEventListener("pageshow", onReturn);
    };
  }, [load]);
  const pollRef = React.useRef<number | null>(null);
  const stopPolling = React.useCallback(() => {
    if (pollRef.current !== null) {
      window.clearInterval(pollRef.current);
      pollRef.current = null;
    }
  }, []);
  const startPolling = React.useCallback(
    (provider: string) => {
      stopPolling();
      let ticks = 0;
      pollRef.current = window.setInterval(() => {
        ticks += 1;
        if (ticks > 48) {
          stopPolling();
          return;
        }
        userAPI
          .listConnections()
          .then((items) => {
            setConnections(items);
            setError(null);
            if (items.some((c) => c.provider === provider)) stopPolling();
          })
          .catch(() => {});
      }, 2500);
    },
    [stopPolling],
  );
  React.useEffect(() => stopPolling, [stopPolling]);
  const providers: { id: string; name: string; icon: IconName; desc: string }[] = [
    { id: "github", name: "GitHub", icon: "github", desc: t("connections.githubDesc") },
    { id: "steam", name: "Steam", icon: "steam", desc: t("connections.steamDesc") },
  ];
  const connect = (provider: string) => {
    if (busy) return;
    setBusy(true);
    setError(null);
    const popup = window.open("", "_blank");
    userAPI
      .startConnection(provider)
      .then((url) => {
        if (popup) {
          popup.location.href = url;
          startPolling(provider);
        } else {
          window.location.href = url;
        }
      })
      .catch((e) => {
        popup?.close();
        const apiErr = e instanceof ApiError ? e : null;
        setError(
          apiErr?.code === "PROVIDER_NOT_CONFIGURED"
            ? t("connections.notConfigured")
            : t("connections.failed"),
        );
      })
      .finally(() => setBusy(false));
  };
  const disconnect = (provider: string) => {
    if (busy) return;
    setBusy(true);
    setError(null);
    userAPI
      .removeConnection(provider)
      .then(() => load())
      .catch(() => setError(t("connections.failed")))
      .finally(() => setBusy(false));
  };
  return (
    <>
      <h1>{t("connections.title")}</h1>
      <p className="lead">{t("connections.lead")}</p>
      <div className="s-section">
        <h2>{t("connections.available")}</h2>
        {error && <div className="s-error">{error}</div>}
        {!connections && !error ? (
          <div className="s-row"><span className="loading-spinner" /></div>
        ) : (
          providers.map((p) => {
            const linked = connections?.find((c) => c.provider === p.id);
            return (
              <div className="conn-row" key={p.id}>
                <div className="conn-ico"><Icon name={p.icon} size={18} /></div>
                <div className="conn-info">
                  <div className="conn-name">{p.name}</div>
                  <div className="conn-meta">
                    {linked
                      ? t("connections.connectedAs", { name: linked.displayName || p.name })
                      : p.desc}
                  </div>
                </div>
                <button
                  className={"conn-action" + (linked ? "" : " connect")}
                  disabled={busy}
                  onClick={() => (linked ? disconnect(p.id) : connect(p.id))}
                >
                  {linked ? t("connections.disconnect") : t("connections.connect")}
                </button>
              </div>
            );
          })
        )}
      </div>
    </>
  );
}

function SubscriptionSection() {
  const { t } = useTranslation("settings");
  return (
    <>
      <h1>{t("subscription.title")}</h1>
      <p className="lead">{t("subscription.lead")}</p>

      <div className="s-section">
        <h2>{t("subscription.currentPlan")}</h2>
        <div className="plan-card current">
          <div className="plan-info">
            <div className="plan-name">{t("subscription.free")}</div>
            <div className="plan-price">{t("subscription.noPaymentMethod")}</div>
          </div>
          <span className="plan-badge">{t("subscription.current")}</span>
        </div>
        <div className="plan-card">
          <div className="plan-info">
            <div className="plan-name">{t("subscription.premium")}</div>
            <div className="plan-price">$8/month</div>
          </div>
          <button className="s-btn primary">{t("subscription.upgrade")}</button>
        </div>
        <div className="plan-card">
          <div className="plan-info">
            <div className="plan-name">Rift Studio</div>
            <div className="plan-price">$16/month · Everything in Pro plus screen-share recording, AI summaries</div>
          </div>
          <button className="s-btn secondary">See features</button>
        </div>
      </div>

      <div className="s-section">
        <h2>Billing</h2>
        <SettingsRow label="Payment method" hint="Add a card to upgrade.">
          <button className="s-btn secondary">Add card</button>
        </SettingsRow>
        <SettingsRow label="Billing history" hint="Download past invoices.">
          <button className="s-btn secondary">View</button>
        </SettingsRow>
      </div>
    </>
  );
}

function AppearanceSection() {
  const { t } = useTranslation("settings");
  const mode = useAppearanceStore((s) => s.mode);
  const accent = useAppearanceStore((s) => s.accent);
  const chatFontSize = useAppearanceStore((s) => s.chatFontSize);
  const reduceMotion = useAppearanceStore((s) => s.reduceMotion);
  const update = useAppearanceStore((s) => s.update);
  return (
    <>
      <h1>{t("appearance.title")}</h1>
      <p className="lead">{t("appearance.lead")}</p>

      <div className="s-section">
        <h2>{t("appearance.theme")}</h2>
        <div className="theme-grid">
          <button className={'theme-card' + (mode === 'light' ? ' on' : '')} onClick={() => update({ mode: 'light' })}>
            <div className="preview"><div className="pp-side"/><div className="pp-main"/></div>
            {t("appearance.light")}
          </button>
          <button className={'theme-card dark-pv' + (mode === 'dark' ? ' on' : '')} onClick={() => update({ mode: 'dark' })}>
            <div className="preview"><div className="pp-side"/><div className="pp-main"/></div>
            {t("appearance.dark")}
          </button>
          <button className={'theme-card system-pv' + (mode === 'system' ? ' on' : '')} onClick={() => update({ mode: 'system' })}>
            <div className="preview"><div className="pp-side"/><div className="pp-main"/></div>
            {t("appearance.system")}
          </button>
        </div>
      </div>

      <div className="s-section">
        <h2>{t("appearance.accent")}</h2>
        <SettingsRow label={t("appearance.accent")} hint="">
          <div className="accent-row">
            {ACCENT_OPTIONS.map(a => (
              <button key={a.value}
                className={'accent-swatch' + (accent === a.value ? ' on' : '')}
                style={{ background: a.value }}
                onClick={() => update({ accent: a.value })}
                title={a.name}/>
            ))}
          </div>
        </SettingsRow>
      </div>

      <div className="s-section">
        <h2>{t("appearance.interface")}</h2>
        <SettingsRow label={t("appearance.fontSize")} hint={`${chatFontSize}px`}>
          <input
            type="range"
            className="s-range"
            min={12}
            max={18}
            value={chatFontSize}
            style={{ "--pct": `${((chatFontSize - 12) / 6) * 100}%` } as React.CSSProperties}
            onChange={(e) => update({ chatFontSize: Number(e.target.value) })}
          />
        </SettingsRow>
        <SettingsRow label={t("appearance.reduceMotion")} hint={t("appearance.reduceMotionHint")}>
          <Switch on={reduceMotion} onClick={() => update({ reduceMotion: !reduceMotion })}/>
        </SettingsRow>
      </div>
    </>
  );
}
function MicTestMeter() {
  const micDeviceId = useVoiceSettingsStore((s) => s.micDeviceId);
  const noiseMode = useVoiceSettingsStore((s) => s.noiseMode);
  const autoSensitivity = useVoiceSettingsStore((s) => s.autoSensitivity);
  const sensitivity = useVoiceSettingsStore((s) => s.sensitivity);
  const [level, setLevel] = React.useState(-120);
  React.useEffect(() => {
    let cancelled = false;
    let stream: MediaStream | null = null;
    let ctx: AudioContext | null = null;
    let pipeline: {
      dispose: () => Promise<void>;
      outputTrack: MediaStreamTrack;
      hasDenoiser: boolean;
      setStrength: (value: number) => void;
    } | null = null;
    let timer = 0;
    const start = async () => {
      try {
        stream = await navigator.mediaDevices.getUserMedia({
          audio: {
            ...(micDeviceId ? { deviceId: { exact: micDeviceId } } : {}),
            echoCancellation: true,
            noiseSuppression: noiseMode !== "off",
            autoGainControl: true,
          },
        });
      } catch {
        return;
      }
      if (cancelled) {
        stream.getTracks().forEach((tr) => tr.stop());
        return;
      }
      let analyser: AnalyserNode;
      let data: Uint8Array;
      try {
        const AudioCtx =
          window.AudioContext ||
          (window as unknown as { webkitAudioContext: typeof AudioContext })
            .webkitAudioContext;
        let tapSource: MediaStreamAudioSourceNode;
        if (noiseMode === "high") {
          const { buildDfnPipeline, DENOISER_SAMPLE_RATE } = await import(
            "../../services/dfn"
          );
          if (cancelled) return;
          ctx = new AudioCtx({ sampleRate: DENOISER_SAMPLE_RATE });
          const source = ctx.createMediaStreamSource(stream);
          pipeline = await buildDfnPipeline({ source, context: ctx });
          if (cancelled) return;
          if (pipeline.hasDenoiser) {
            pipeline.setStrength(getLiveKitService().getNoiseStrength());
            tapSource = ctx.createMediaStreamSource(
              new MediaStream([pipeline.outputTrack]),
            );
          } else {
            try {
              await pipeline.dispose();
            } catch {}
            try {
              pipeline.outputTrack.stop();
            } catch {}
            pipeline = null;
            tapSource = ctx.createMediaStreamSource(stream);
          }
        } else {
          ctx = new AudioCtx();
          tapSource = ctx.createMediaStreamSource(stream);
        }
        analyser = ctx.createAnalyser();
        analyser.fftSize = 1024;
        tapSource.connect(analyser);
        data = new Uint8Array(analyser.fftSize);
      } catch {
        return;
      }
      void ctx?.resume().catch(() => {});
      const tick = () => {
        if (cancelled) return;
        analyser.getByteTimeDomainData(data);
        let sum = 0;
        for (let i = 0; i < data.length; i++) {
          const d = (data[i] - 128) / 128;
          sum += d * d;
        }
        const db =
          20 * Math.log10(Math.max(Math.sqrt(sum / data.length), 1e-6));
        setLevel((prev) => Math.max(db, prev - 4));
        timer = window.setTimeout(tick, 50);
      };
      tick();
    };
    void start();
    return () => {
      cancelled = true;
      window.clearTimeout(timer);
      const p = pipeline;
      if (p) {
        void p.dispose().catch(() => {});
        try {
          p.outputTrack.stop();
        } catch {}
      }
      stream?.getTracks().forEach((tr) => tr.stop());
      ctx?.close().catch(() => {});
    };
  }, [micDeviceId, noiseMode]);
  const threshold = -72 + 0.72 * (autoSensitivity ? 50 : sensitivity);
  const levelPct = Math.min(100, Math.max(0, ((level + 72) / 72) * 100));
  const thresholdPct = Math.min(100, Math.max(0, ((threshold + 72) / 72) * 100));
  return (
    <div className="mic-test">
      <div
        className={"mic-test-fill" + (level > threshold ? " live" : "")}
        style={{ width: `${levelPct}%` }}
      />
      <div className="mic-test-thr" style={{ left: `${thresholdPct}%` }} />
    </div>
  );
}

const pttKeyLabel = (code: string): string => {
  if (code.startsWith("Key")) return code.slice(3);
  if (code.startsWith("Digit")) return code.slice(5);
  return code;
};

function VoiceSection() {
  const { t } = useTranslation("settings");
  const micVolume = useVoiceSettingsStore((s) => s.micVolume);
  const outputVolume = useVoiceSettingsStore((s) => s.outputVolume);
  const noiseMode = useVoiceSettingsStore((s) => s.noiseMode);
  const pttKey = useVoiceSettingsStore((s) => s.pttKey);
  const autoSensitivity = useVoiceSettingsStore((s) => s.autoSensitivity);
  const sensitivity = useVoiceSettingsStore((s) => s.sensitivity);
  const pushToTalk = useVoiceSettingsStore((s) => s.pushToTalk);
  const micDeviceId = useVoiceSettingsStore((s) => s.micDeviceId);
  const speakerDeviceId = useVoiceSettingsStore((s) => s.speakerDeviceId);
  const update = useVoiceSettingsStore((s) => s.update);
  const updateDevices = useVoiceSettingsStore((s) => s.updateDevices);
  const [devices, setDevices] = React.useState<MediaDeviceInfo[]>([]);
  const [capturingKey, setCapturingKey] = React.useState(false);
  React.useEffect(() => {
    if (!capturingKey) return;
    const onKey = (e: KeyboardEvent) => {
      e.preventDefault();
      e.stopPropagation();
      if (e.key !== "Escape" && /^[A-Za-z0-9]+$/.test(e.code)) {
        update({ pttKey: e.code });
      }
      setCapturingKey(false);
    };
    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
  }, [capturingKey, update]);
  React.useEffect(() => {
    let cancelled = false;
    const media = navigator.mediaDevices;
    if (!media?.enumerateDevices) return;
    const refresh = () => {
      media
        .enumerateDevices()
        .then((list) => {
          if (!cancelled) setDevices(list);
        })
        .catch(() => {});
    };
    refresh();
    media.addEventListener?.("devicechange", refresh);
    return () => {
      cancelled = true;
      media.removeEventListener?.("devicechange", refresh);
    };
  }, []);
  const inputs = devices.filter((d) => d.kind === "audioinput");
  const outputs = devices.filter((d) => d.kind === "audiooutput");
  const pct = (v: number) => ({ "--pct": `${v}%` } as React.CSSProperties);
  return (
    <>
      <h1>{t("voice.title")}</h1>
      <p className="lead">{t("voice.lead")}</p>

      <div className="s-section">
        <h2>{t("voice.inputDevice")}</h2>
        <div className="s-field-row">
          <div className="field-lbl">{t("voice.microphone")}</div>
          <select
            className="s-input"
            value={micDeviceId}
            onChange={(e) => updateDevices({ micDeviceId: e.target.value })}
          >
            <option value="">{t("voice.defaultDevice")}</option>
            {inputs.map((d, i) => (
              <option key={d.deviceId} value={d.deviceId}>
                {d.label || `${t("voice.microphone")} ${i + 1}`}
              </option>
            ))}
          </select>
        </div>
        <SettingsRow label={t("voice.inputVolume")} hint={`${micVolume}%`}>
          <input
            type="range"
            className="s-range"
            min={0}
            max={100}
            value={micVolume}
            style={pct(micVolume)}
            onChange={(e) => update({ micVolume: Number(e.target.value) })}
          />
        </SettingsRow>
      </div>

      <div className="s-section">
        <h2>{t("voice.outputDevice")}</h2>
        <div className="s-field-row">
          <div className="field-lbl">{t("voice.speakers")}</div>
          <select
            className="s-input"
            value={speakerDeviceId}
            onChange={(e) => updateDevices({ speakerDeviceId: e.target.value })}
          >
            <option value="">{t("voice.defaultDevice")}</option>
            {outputs.map((d, i) => (
              <option key={d.deviceId} value={d.deviceId}>
                {d.label || `${t("voice.speakers")} ${i + 1}`}
              </option>
            ))}
          </select>
        </div>
        <SettingsRow label={t("voice.outputVolume")} hint={`${outputVolume}%`}>
          <input
            type="range"
            className="s-range"
            min={0}
            max={100}
            value={outputVolume}
            style={pct(outputVolume)}
            onChange={(e) => update({ outputVolume: Number(e.target.value) })}
          />
        </SettingsRow>
      </div>

      <div className="s-section">
        <h2>{t("voice.behavior")}</h2>
        <SettingsRow label={t("voice.pushToTalk")} hint={t("voice.pushToTalkHint", { key: pttKeyLabel(pttKey) })}>
          <button
            type="button"
            className={"kbd s-kbd-btn" + (capturingKey ? " capturing" : "")}
            onClick={() => setCapturingKey(true)}
            title={t("voice.pttBindHint")}
          >
            {capturingKey ? t("voice.pttPressKey") : pttKeyLabel(pttKey)}
          </button>
          <Switch on={pushToTalk} onClick={() => update({ pushToTalk: !pushToTalk })}/>
        </SettingsRow>
        <SettingsRow label={t("voice.noiseDefault")} hint={t("voice.noiseDefaultHint")}>
          <div className="s-seg">
            <button className={noiseMode === "off" ? "on" : ""} onClick={() => update({ noiseMode: "off" })}>{t("voice.noiseOff")}</button>
            <button className={noiseMode === "standard" ? "on" : ""} onClick={() => update({ noiseMode: "standard" })}>{t("voice.noiseStandard")}</button>
            <button className={noiseMode === "high" ? "on" : ""} onClick={() => update({ noiseMode: "high" })}>{t("voice.noiseHigh")}</button>
          </div>
        </SettingsRow>
        <SettingsRow label={t("voice.autoSensitivity")} hint={t("voice.autoSensitivityHint")}>
          <Switch on={autoSensitivity} onClick={() => update({ autoSensitivity: !autoSensitivity })}/>
        </SettingsRow>
        {!autoSensitivity && (
          <SettingsRow label={t("voice.sensitivity")} hint={`${sensitivity}%`}>
            <input
              type="range"
              className="s-range"
              min={0}
              max={100}
              value={sensitivity}
              style={pct(sensitivity)}
              onChange={(e) => update({ sensitivity: Number(e.target.value) })}
            />
          </SettingsRow>
        )}
        {!autoSensitivity && (
          <div className="s-mictest">
            <div className="s-mictest-head">
              <span>{t("voice.micTest")}</span>
              <span className="s-mictest-hint">{t("voice.micTestHint")}</span>
            </div>
            <MicTestMeter />
          </div>
        )}
      </div>
    </>
  );
}
function NotificationsSection() {
  const { t } = useTranslation("settings");
  const [desk, setDesk] = React.useState(true);
  const [sound, setSound] = React.useState(true);
  const [mentions, setMentions] = React.useState('all');
  return (
    <>
      <h1>{t("notifications.title")}</h1>
      <p className="lead">{t("notifications.lead")}</p>

      <div className="s-section">
        <h2>{t("notifications.desktop")}</h2>
        <SettingsRow label={t("notifications.enableDesktop")} hint={t("notifications.enableDesktopHint")}>
          <Switch on={desk} onClick={() => setDesk(v => !v)}/>
        </SettingsRow>
        <SettingsRow label={t("notifications.enableSounds")} hint={t("notifications.enableSoundsHint")}>
          <Switch on={sound} onClick={() => setSound(v => !v)}/>
        </SettingsRow>
      </div>

      <div className="s-section">
        <h2>{t("notifications.mentions")}</h2>
        <SettingsRow label={t("notifications.mentions")} hint="">
          <div className="s-seg">
            <button className={mentions === 'all' ? 'on' : ''} onClick={() => setMentions('all')}>{t("notifications.mentionsAll")}</button>
            <button className={mentions === 'direct' ? 'on' : ''} onClick={() => setMentions('direct')}>{t("notifications.mentionsMentions")}</button>
            <button className={mentions === 'none' ? 'on' : ''} onClick={() => setMentions('none')}>{t("notifications.mentionsNothing")}</button>
          </div>
        </SettingsRow>
      </div>

      <div className="s-section">
        <h2>Quiet Hours</h2>
        <SettingsRow label="Enable quiet hours" hint="Mute all notifications on a schedule.">
          <Switch on={false} onClick={() => {}}/>
        </SettingsRow>
        <div className="s-row">
          <div className="s-row-text">
            <div className="lbl">Schedule</div>
            <div className="hint">Notifications will be silenced during this window.</div>
          </div>
          <div className="s-row-control" style={{ display: 'flex', gap: 8, alignItems: 'center' }}>
            <input className="s-input" style={{ width: 90 }} defaultValue="22:00"/>
            <span style={{ color: 'var(--fg-4)' }}>→</span>
            <input className="s-input" style={{ width: 90 }} defaultValue="08:00"/>
          </div>
        </div>
      </div>
    </>
  );
}

function ShortcutsSection() {
  const groups = [
    { title: 'Navigation', items: [
      { name: 'Quick switcher',     keys: ['⌘', 'K'] },
      { name: 'Next channel',       keys: ['⌥', '↓'] },
      { name: 'Previous channel',   keys: ['⌥', '↑'] },
      { name: 'Mark all as read',   keys: ['⇧', 'Esc'] },
      { name: 'Open settings',      keys: ['⌘', ','] },
    ]},
    { title: 'Messaging', items: [
      { name: 'New line',           keys: ['⇧', '↵'] },
      { name: 'Edit last message',  keys: ['↑'] },
      { name: 'React to last',      keys: ['⌘', 'E'] },
      { name: 'Reply in thread',    keys: ['T'] },
      { name: 'Toggle markdown preview', keys: ['⌘', '/'] },
    ]},
    { title: 'Voice', items: [
      { name: 'Toggle mute',        keys: ['⌘', 'M'] },
      { name: 'Toggle deafen',      keys: ['⌘', '⇧', 'D'] },
      { name: 'Push-to-talk',       keys: ['Space'] },
      { name: 'Leave voice',        keys: ['⌘', '⇧', 'L'] },
    ]},
  ];
  const { t } = useTranslation("settings");
  return (
    <>
      <h1>{t("shortcuts.title")}</h1>
      <p className="lead">{t("shortcuts.lead")}</p>
      {groups.map(g => (
        <div className="s-section" key={g.title}>
          <h2>{g.title}</h2>
          {g.items.map(item => (
            <div className="kbd-row" key={item.name}>
              <span className="kbd-name">{item.name}</span>
              <span className="kbd-keys">
                {item.keys.map((k, i) => <span className="kbd" key={i}>{k}</span>)}
              </span>
            </div>
          ))}
        </div>
      ))}
    </>
  );
}

function LanguageSection() {
  const { t, i18n } = useTranslation("settings");
  void i18n.language;
  const active = currentLocale();
  const langs: { v: "en" | "ru"; n: string }[] = [
    { v: "en", n: "English" },
    { v: "ru", n: "Русский" },
  ];

  const apply = (next: "en" | "ru") => {
    setLocale(next);
    void (async () => {
      try {
        const { authAPI } = await import("../../api/auth");
        await authAPI.updateLocale(next);
      } catch (err) {
        console.error("[Settings] updateLocale failed:", err);
      }
    })();
  };

  return (
    <>
      <h1>{t("language.title")}</h1>
      <p className="lead">{t("language.lead")}</p>
      <div className="s-section">
        <h2>{t("language.displayLanguage")}</h2>
        {langs.map((l) => (
          <button
            key={l.v}
            className="s-row"
            onClick={() => apply(l.v)}
            style={{
              width: "100%",
              border: 0,
              background: "transparent",
              cursor: "pointer",
              textAlign: "left",
            }}
          >
            <div className="s-row-text">
              <div className="lbl">{l.n}</div>
              <div className="hint">{l.v.toUpperCase()}</div>
            </div>
            <div className="s-row-control">
              {active === l.v && (
                <Icon name="check" size={18} style={{ color: "var(--accent)" }} />
              )}
            </div>
          </button>
        ))}
      </div>
    </>
  );
}

const ALL_RAIL_IDS: SectionId[] = SETTINGS_GROUPS.flatMap((g) => g.items.map((i) => i.id));

function SettingsView({ open, onClose, onSignOut }: SettingsViewProps) {
  const { t } = useTranslation("settings");
  const [activeId, setActiveId] = React.useState<SectionId>("account");
  const overlayRef = React.useRef<HTMLDivElement>(null);
  const railButtonRefs = React.useRef<Record<string, HTMLButtonElement | null>>({});

  React.useEffect(() => {
    if (!open) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') { onClose(); return; }
      if (e.key !== 'Tab' || !overlayRef.current) return;
      const focusables = overlayRef.current.querySelectorAll<HTMLElement>(
        'a, button:not([tabindex="-1"]), input, textarea, select, [tabindex]:not([tabindex="-1"])',
      );
      if (focusables.length === 0) return;
      const first = focusables[0];
      const last = focusables[focusables.length - 1];
      const active = document.activeElement as HTMLElement | null;
      if (e.shiftKey && active === first) {
        e.preventDefault();
        last.focus();
      } else if (!e.shiftKey && active === last) {
        e.preventDefault();
        first.focus();
      } else if (active && !overlayRef.current.contains(active)) {
        e.preventDefault();
        first.focus();
      }
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [open, onClose]);

  React.useEffect(() => {
    if (!open) return;
    railButtonRefs.current[activeId]?.focus();
  }, [open, activeId]);

  if (!open) return null;

  let body: React.ReactNode;
  switch (activeId) {
    case 'account':       body = <AccountSection onAccountGone={() => { onClose(); useAuthStore.getState().logout(); }}/>; break;
    case 'privacy':       body = <PrivacySection/>; break;
    case 'connections':   body = <ConnectionsSection/>; break;
    case 'subscription':  body = <SubscriptionSection/>; break;
    case 'appearance':    body = <AppearanceSection/>; break;
    case 'voice':         body = <VoiceSection/>; break;
    case 'notifications': body = <NotificationsSection/>; break;
    case 'shortcuts':     body = <ShortcutsSection/>; break;
    case 'language':      body = <LanguageSection/>; break;
    default: body = null;
  }

  const onRailKeyDown = (e: React.KeyboardEvent) => {
    if (e.key !== 'ArrowDown' && e.key !== 'ArrowUp' && e.key !== 'Home' && e.key !== 'End') return;
    e.preventDefault();
    const idx = ALL_RAIL_IDS.indexOf(activeId);
    let nextIdx = idx;
    if (e.key === 'ArrowDown') nextIdx = (idx + 1) % ALL_RAIL_IDS.length;
    if (e.key === 'ArrowUp') nextIdx = (idx - 1 + ALL_RAIL_IDS.length) % ALL_RAIL_IDS.length;
    if (e.key === 'Home') nextIdx = 0;
    if (e.key === 'End') nextIdx = ALL_RAIL_IDS.length - 1;
    const nextId = ALL_RAIL_IDS[nextIdx];
    setActiveId(nextId);
    railButtonRefs.current[nextId]?.focus();
  };

  return (
    <div ref={overlayRef} className="settings-overlay">
      <nav className="settings-rail" role="tablist" aria-orientation="vertical" onKeyDown={onRailKeyDown}>
        {SETTINGS_GROUPS.map((group) => (
          <React.Fragment key={group.groupKey}>
            <div className="rail-section">{t(group.groupKey)}</div>
            {group.items.map((item) => (
              <button
                key={item.id}
                ref={(el) => { railButtonRefs.current[item.id] = el; }}
                role="tab"
                aria-selected={activeId === item.id}
                className={"rail-item" + (activeId === item.id ? " active" : "")}
                onClick={() => setActiveId(item.id)}
              >
                <span className="ic"><Icon name={item.icon} size={15} /></span>
                {t(`sections.${item.id}` as const)}
              </button>
            ))}
          </React.Fragment>
        ))}
        <div style={{ height: 1, background: "var(--hairline)", margin: "12px 8px" }} />
        <button className="rail-item danger" onClick={onSignOut}>
          <span className="ic"><Icon name="logout" size={15} /></span>
          {t("logOut")}
        </button>
      </nav>

      <div className="settings-pane">
        <button className="settings-close" onClick={onClose} title="Close (Esc)">
          <Icon name="x" size={16} />
          <span className="esc">{t("esc")}</span>
        </button>
        <div className="settings-content">
          {body}
        </div>
      </div>
    </div>
  );
}

function TweaksPanel({ theme, onTheme }: TweaksPanelProps) {
  return (
    <div className="tweaks-panel">
      <div className="t-title">Tweaks</div>
      <div className="t-row">
        <div className="t-label">Appearance</div>
        <div className="t-seg">
          <button className={theme === 'light' ? 'on' : ''} onClick={() => onTheme('light')}>Light</button>
          <button className={theme === 'dark'  ? 'on' : ''} onClick={() => onTheme('dark')}>Dark</button>
        </div>
      </div>
    </div>
  );
}

export default SettingsView;
export { TweaksPanel };
