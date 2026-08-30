import React from "react";
import { useTranslation } from "react-i18next";
import Icon, { type IconName } from "../rift/Icon";
import { currentLocale, setLocale } from "../../i18n";
import { useAuthStore } from "../../store/authStore";
import { userAPI } from "../../api/user";
import { ApiError } from "../../utils/apiError";
import { UserAvatar } from "../rift/Avatar";
import { bustAvatar, useAvatarUrl } from "../../utils/avatarCache";
import { validateUsername, type UsernameIssue } from "../../utils/username";
import { ChangeEmailDialog, ChangePasswordDialog, DangerPasswordDialog } from "./AccountModals";
import { profileVersion, useUserProfileStore } from "../../store/userProfileStore";

type Theme = "light" | "dark";

interface SettingsViewProps {
  open: boolean;
  theme: Theme;
  onTheme: (t: Theme) => void;
  accent: string;
  onAccent: (a: string) => void;
  onClose: () => void;
  onSignOut: () => void;
}

interface TweaksPanelProps {
  theme: Theme;
  onTheme: (t: Theme) => void;
}

interface AppearanceSectionProps {
  theme: Theme;
  onTheme: (t: Theme) => void;
  accent: string;
  onAccent: (a: string) => void;
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

interface SwitchProps { on: boolean; onClick: () => void; }

function Switch({ on, onClick }: SwitchProps) {
  return <button className={'s-switch' + (on ? ' on' : '')} onClick={onClick} aria-pressed={on}/>;
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
  const [activityBusy, setActivityBusy] = React.useState(false);
  React.useEffect(() => {
    setBio(user?.bio ?? "");
  }, [user?.bio]);
  const bioDirty = bio.trim() !== (user?.bio ?? "");
  const profileQueue = React.useRef<Promise<void>>(Promise.resolve());
  const runProfileUpdate = (patch: { bio?: string; showActivity?: boolean }) => {
    const initiator = useAuthStore.getState().user?.id;
    const next = profileQueue.current.then(async () => {
      const saved = await userAPI.updateProfile(patch);
      const current = useAuthStore.getState().user;
      if (!current || !initiator || current.id !== initiator) return;
      const version = profileVersion(saved.profileUpdatedAt);
      const cached = useUserProfileStore.getState().bios[current.id];
      if (cached && cached.at > version) return;
      useUserProfileStore.getState().apply(current.id, saved.bio, version);
      useAuthStore.getState().setUser({ ...current, bio: saved.bio, showActivity: saved.showActivity });
    });
    profileQueue.current = next.catch(() => {});
    return next;
  };
  const saveBio = async () => {
    if (!user || bioBusy || !bioDirty) return;
    setBioBusy(true);
    setBioError(null);
    try {
      await runProfileUpdate({ bio: bio.trim() });
    } catch (err) {
      const apiErr = err instanceof ApiError ? err : null;
      setBioError(apiErr?.message || t("account.bioError"));
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
        <div className="s-field-row">
          <div className="field-lbl">{t("account.about")}</div>
          <textarea
            className="s-input"
            placeholder={t("account.aboutPlaceholder")}
            value={bio}
            disabled={bioBusy}
            onChange={(e) => {
              const next = Array.from(e.target.value).slice(0, 190).join("");
              setBio(next);
              setBioError(null);
            }}
          />
          <div className="s-field-foot">
            <span className="s-field-count">{Array.from(bio).length}/190</span>
            {bioDirty && (
              <span className="s-field-actions">
                <button className="s-btn secondary" disabled={bioBusy} onClick={() => setBio(user?.bio ?? "")}>
                  {t("actions.cancel")}
                </button>
                <button className="s-btn primary" disabled={bioBusy} onClick={() => void saveBio()}>
                  {t("actions.save")}
                </button>
              </span>
            )}
          </div>
          {bioError && <div className="s-error">{bioError}</div>}
        </div>
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
  const [dms, setDms] = React.useState('friends');
  const [requests, setRequests] = React.useState(true);
  const [scan, setScan] = React.useState(true);
  return (
    <>
      <h1>{t("privacy.title")}</h1>
      <p className="lead">{t("privacy.lead")}</p>

      <div className="s-section">
        <h2>{t("privacy.directMessages")}</h2>
        <SettingsRow label={t("privacy.whoCanDm")} hint={t("privacy.whoCanDmHint")}>
          <div className="s-seg">
            <button className={dms === 'everyone' ? 'on' : ''} onClick={() => setDms('everyone')}>{t("privacy.everyone")}</button>
            <button className={dms === 'friends'  ? 'on' : ''} onClick={() => setDms('friends')}>{t("privacy.friends")}</button>
            <button className={dms === 'nobody'   ? 'on' : ''} onClick={() => setDms('nobody')}>{t("privacy.noOne")}</button>
          </div>
        </SettingsRow>
        <SettingsRow label={t("privacy.allowRequests")} hint={t("privacy.allowRequestsHint")}>
          <Switch on={requests} onClick={() => setRequests(v => !v)}/>
        </SettingsRow>
        <SettingsRow label={t("privacy.scanMessages")} hint={t("privacy.scanMessagesHint")}>
          <Switch on={scan} onClick={() => setScan(v => !v)}/>
        </SettingsRow>
      </div>
    </>
  );
}

function ConnectionsSection() {
  const conns: { id: string; name: string; meta: string; icon: IconName; connected: boolean }[] = [
    { id: 'github',  name: 'GitHub',  meta: 'Connected as @you', icon: 'globe', connected: true },
    { id: 'spotify', name: 'Spotify', meta: 'Show what you\'re listening to', icon: 'volume', connected: false },
    { id: 'figma',   name: 'Figma',   meta: 'Drop links that auto-expand', icon: 'palette', connected: false },
    { id: 'cal',     name: 'Calendar', meta: 'Sync busy/free for status', icon: 'bell', connected: false },
    { id: 'gh-iss',  name: 'Linear',  meta: 'Convert messages to issues', icon: 'check', connected: false },
  ];
  const { t } = useTranslation("settings");
  return (
    <>
      <h1>{t("connections.title")}</h1>
      <p className="lead">{t("connections.lead")}</p>
      <div className="s-section">
        <h2>{t("connections.available")}</h2>
        {conns.map(c => (
          <div className="conn-row" key={c.id}>
            <div className="conn-ico"><Icon name={c.icon} size={18}/></div>
            <div className="conn-info">
              <div className="conn-name">{c.name}</div>
              <div className="conn-meta">{c.meta}</div>
            </div>
            <button className={'conn-action' + (c.connected ? '' : ' connect')}>
              {c.connected ? t("connections.disconnect") : t("connections.connect")}
            </button>
          </div>
        ))}
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

function AppearanceSection({ theme, onTheme, accent, onAccent }: AppearanceSectionProps) {
  const { t } = useTranslation("settings");
  const [density, setDensity] = React.useState('cozy');
  const [size, setSize] = React.useState(14);
  return (
    <>
      <h1>{t("appearance.title")}</h1>
      <p className="lead">{t("appearance.lead")}</p>

      <div className="s-section">
        <h2>{t("appearance.theme")}</h2>
        <div className="theme-grid">
          <button className={'theme-card' + (theme === 'light' ? ' on' : '')} onClick={() => onTheme('light')}>
            <div className="preview"><div className="pp-side"/><div className="pp-main"/></div>
            {t("appearance.light")}
          </button>
          <button className={'theme-card dark-pv' + (theme === 'dark' ? ' on' : '')} onClick={() => onTheme('dark')}>
            <div className="preview"><div className="pp-side"/><div className="pp-main"/></div>
            {t("appearance.dark")}
          </button>
          <button className={'theme-card system-pv'} onClick={() => onTheme(window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light')}>
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
                onClick={() => onAccent(a.value)}
                title={a.name}/>
            ))}
          </div>
        </SettingsRow>
      </div>

      <div className="s-section">
        <h2>Layout</h2>
        <SettingsRow label="Message density" hint="Cozy adds breathing room. Compact fits more on screen.">
          <div className="s-seg">
            <button className={density === 'compact' ? 'on' : ''} onClick={() => setDensity('compact')}>Compact</button>
            <button className={density === 'cozy' ? 'on' : ''} onClick={() => setDensity('cozy')}>Cozy</button>
          </div>
        </SettingsRow>
        <SettingsRow label="Chat font size" hint={`${size}px`}>
          <input type="range" min="12" max="18" value={size} onChange={e => setSize(Number(e.target.value))}/>
        </SettingsRow>
        <SettingsRow label="Reduce motion" hint="Minimize transitions and animations.">
          <Switch on={false} onClick={() => {}}/>
        </SettingsRow>
      </div>
    </>
  );
}

function VoiceSection() {
  const [input, setInput] = React.useState('MacBook Pro Microphone');
  const [output, setOutput] = React.useState('AirPods Pro');
  const [inVol, setInVol] = React.useState(70);
  const [outVol, setOutVol] = React.useState(85);
  const [push, setPush] = React.useState(false);
  const { t } = useTranslation("settings");
  return (
    <>
      <h1>{t("voice.title")}</h1>
      <p className="lead">{t("voice.lead")}</p>

      <div className="s-section">
        <h2>{t("voice.inputDevice")}</h2>
        <div className="s-field-row">
          <div className="field-lbl">{t("voice.microphone")}</div>
          <input className="s-input" value={input} onChange={e => setInput(e.target.value)}/>
        </div>
        <SettingsRow label={t("voice.inputVolume")} hint={`${inVol}%`}>
          <input type="range" min="0" max="100" value={inVol} onChange={e => setInVol(Number(e.target.value))}/>
        </SettingsRow>
      </div>

      <div className="s-section">
        <h2>{t("voice.outputDevice")}</h2>
        <div className="s-field-row">
          <div className="field-lbl">{t("voice.speakers")}</div>
          <input className="s-input" value={output} onChange={e => setOutput(e.target.value)}/>
        </div>
        <SettingsRow label={t("voice.outputVolume")} hint={`${outVol}%`}>
          <input type="range" min="0" max="100" value={outVol} onChange={e => setOutVol(Number(e.target.value))}/>
        </SettingsRow>
      </div>

      <div className="s-section">
        <h2>{t("voice.pushToTalk")}</h2>
        <SettingsRow label={t("voice.pushToTalk")} hint={t("voice.pushToTalkHint")}>
          <Switch on={push} onClick={() => setPush(v => !v)}/>
        </SettingsRow>
        <SettingsRow label={t("voice.noiseSuppression")} hint="">
          <Switch on={true} onClick={() => {}}/>
        </SettingsRow>
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

function SettingsView({ open, theme, onTheme, accent, onAccent, onClose, onSignOut }: SettingsViewProps) {
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
    case 'appearance':    body = <AppearanceSection theme={theme} onTheme={onTheme} accent={accent} onAccent={onAccent}/>; break;
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
