import React from "react";
import { useTranslation } from "react-i18next";
import Icon, { type IconName } from "../../../shared/ui/Icon";
import { useAuthStore } from "../../auth/store/authStore";
import type { SectionId } from "./types";

import { AccountSection } from "./sections/AccountSection";
import { PrivacySection } from "./sections/PrivacySection";
import { ConnectionsSection } from "./sections/ConnectionsSection";
import { SubscriptionSection } from "./sections/SubscriptionSection";
import { AppearanceSection } from "./sections/AppearanceSection";
import { VoiceSection } from "./sections/VoiceSection";
import { NotificationsSection } from "./sections/NotificationsSection";
import { ShortcutsSection } from "./sections/ShortcutsSection";
import { LanguageSection } from "./sections/LanguageSection";
type Theme = "light" | "dark";

interface SettingsViewProps {
  open: boolean;
  onClose: () => void;
  onSignOut: () => void;
  initialSection?: SectionId;
}

interface TweaksPanelProps {
  theme: Theme;
  onTheme: (t: Theme) => void;
}


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


const ALL_RAIL_IDS: SectionId[] = SETTINGS_GROUPS.flatMap((g) => g.items.map((i) => i.id));

function SettingsView({ open, onClose, onSignOut, initialSection = "account" }: SettingsViewProps) {
  const { t } = useTranslation("settings");
  const [activeId, setActiveId] = React.useState<SectionId>(initialSection);
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
    case 'shortcuts':     body = <ShortcutsSection onNavigate={setActiveId}/>; break;
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
        <div id="settings-voice-dock" className="settings-voice-dock" />
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
