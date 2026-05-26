import React from "react";
import Icon, { type IconName } from "../rift/Icon";
import { useI18nStore } from "../../store/i18nStore";

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

const SETTINGS_SECTIONS: { group: string; items: { id: string; label: string; icon: IconName }[] }[] = [
  { group: 'User Settings', items: [
    { id: 'account',       label: 'My Account',       icon: 'user' },
    { id: 'profile',       label: 'Profile',          icon: 'edit' },
    { id: 'privacy',       label: 'Privacy & Safety', icon: 'shield' },
    { id: 'connections',   label: 'Connections',      icon: 'link' },
    { id: 'subscription',  label: 'Subscription',     icon: 'credit-card' },
  ]},
  { group: 'App Settings', items: [
    { id: 'appearance',    label: 'Appearance',       icon: 'palette' },
    { id: 'voice',         label: 'Voice & Video',    icon: 'mic' },
    { id: 'notifications', label: 'Notifications',    icon: 'bell' },
    { id: 'shortcuts',     label: 'Keyboard Shortcuts', icon: 'keyboard' },
    { id: 'language',      label: 'Language',         icon: 'globe' },
  ]},
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


function AccountSection() {
  return (
    <>
      <h1>My Account</h1>
      <p className="lead">How you appear to others on Rift.</p>

      <div className="profile-card">
        <div className="ava-lg">
          Y
          <span className="edit-badge"><Icon name="camera" size={12}/></span>
        </div>
        <div className="pc-info">
          <div className="pc-name">You</div>
          <div className="pc-handle">@you · Rift since Mar 2024</div>
        </div>
        <button className="s-btn secondary">Edit profile</button>
      </div>

      <div className="s-section">
        <h2>Account Information</h2>
        <SettingsRow label="Display Name" hint="This is how everyone sees you in chat.">
          <button className="s-btn secondary">Edit</button>
        </SettingsRow>
        <SettingsRow label="Username" hint="Your unique handle. You.">
          <button className="s-btn secondary">Edit</button>
        </SettingsRow>
        <SettingsRow label="Email" hint="you@example.com (verified)">
          <button className="s-btn secondary">Edit</button>
        </SettingsRow>
        <SettingsRow label="Phone Number" hint="Add for account recovery and 2FA.">
          <button className="s-btn secondary">Add</button>
        </SettingsRow>
      </div>

      <div className="s-section">
        <h2>Password & Authentication</h2>
        <SettingsRow label="Password" hint="Last changed 3 months ago.">
          <button className="s-btn secondary">Change</button>
        </SettingsRow>
        <SettingsRow label="Two-Factor Authentication" hint="Add an extra layer of protection.">
          <button className="s-btn primary">Enable</button>
        </SettingsRow>
      </div>

      <div className="s-section">
        <h2>Account Removal</h2>
        <SettingsRow label="Disable Account" hint="Hide your profile and stop notifications. You can reactivate anytime.">
          <button className="s-btn danger">Disable</button>
        </SettingsRow>
        <SettingsRow label="Delete Account" hint="Permanently remove your data. This cannot be undone.">
          <button className="s-btn danger">Delete</button>
        </SettingsRow>
      </div>
    </>
  );
}

function ProfileSection() {
  const [bio, setBio] = React.useState("Building things at the intersection of design and language.");
  return (
    <>
      <h1>Profile</h1>
      <p className="lead">Customize how others see your identity card.</p>

      <div className="s-section">
        <h2>Display</h2>
        <div className="s-field-row">
          <div className="field-lbl">Display Name</div>
          <input className="s-input" defaultValue="You"/>
        </div>
        <div className="s-field-row">
          <div className="field-lbl">Pronouns</div>
          <input className="s-input" placeholder="they/them" defaultValue=""/>
        </div>
        <div className="s-field-row">
          <div className="field-lbl">About me</div>
          <textarea className="s-input" value={bio} onChange={e => setBio(e.target.value)}/>
          <div style={{ fontSize: 11, color: 'var(--fg-4)', textAlign: 'right' }}>{bio.length}/190</div>
        </div>
      </div>

      <div className="s-section">
        <h2>Status</h2>
        <SettingsRow label="Custom Status" hint="Show a short message next to your name.">
          <input className="s-input" style={{ width: 200 }} placeholder="e.g. Heads down ✦" />
        </SettingsRow>
        <SettingsRow label="Show activity" hint="Let others see what app or game you're using.">
          <Switch on={true} onClick={() => {}}/>
        </SettingsRow>
      </div>
    </>
  );
}

function PrivacySection() {
  const [dms, setDms] = React.useState('friends');
  const [requests, setRequests] = React.useState(true);
  const [scan, setScan] = React.useState(true);
  return (
    <>
      <h1>Privacy & Safety</h1>
      <p className="lead">Control who can reach you and how your data is used.</p>

      <div className="s-section">
        <h2>Direct Messages</h2>
        <SettingsRow label="Who can DM me" hint="Limit unsolicited messages.">
          <div className="s-seg">
            <button className={dms === 'everyone' ? 'on' : ''} onClick={() => setDms('everyone')}>Everyone</button>
            <button className={dms === 'friends'  ? 'on' : ''} onClick={() => setDms('friends')}>Friends</button>
            <button className={dms === 'nobody'   ? 'on' : ''} onClick={() => setDms('nobody')}>Nobody</button>
          </div>
        </SettingsRow>
        <SettingsRow label="Friend requests" hint="Accept new connections from anyone, or only friends-of-friends.">
          <Switch on={requests} onClick={() => setRequests(v => !v)}/>
        </SettingsRow>
        <SettingsRow label="Scan media in DMs" hint="Auto-blur potentially sensitive images you receive.">
          <Switch on={scan} onClick={() => setScan(v => !v)}/>
        </SettingsRow>
      </div>

      <div className="s-section">
        <h2>Data</h2>
        <SettingsRow label="Use data to improve Rift" hint="Anonymous telemetry. We never read message contents.">
          <Switch on={false} onClick={() => {}}/>
        </SettingsRow>
        <SettingsRow label="Personalized suggestions" hint="Show servers and people you might know.">
          <Switch on={true} onClick={() => {}}/>
        </SettingsRow>
        <SettingsRow label="Request my data" hint="Get a downloadable archive of everything you've sent.">
          <button className="s-btn secondary"><Icon name="download" size={13}/> Request</button>
        </SettingsRow>
      </div>

      <div className="s-section">
        <h2>Blocked</h2>
        <div className="hint" style={{ fontSize: 13, color: 'var(--fg-3)' }}>You haven't blocked anyone.</div>
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
  return (
    <>
      <h1>Connections</h1>
      <p className="lead">Link other accounts to enrich your messages and presence.</p>
      <div className="s-section">
        <h2>Available</h2>
        {conns.map(c => (
          <div className="conn-row" key={c.id}>
            <div className="conn-ico"><Icon name={c.icon} size={18}/></div>
            <div className="conn-info">
              <div className="conn-name">{c.name}</div>
              <div className="conn-meta">{c.meta}</div>
            </div>
            <button className={'conn-action' + (c.connected ? '' : ' connect')}>
              {c.connected ? 'Disconnect' : 'Connect'}
            </button>
          </div>
        ))}
      </div>
    </>
  );
}

function SubscriptionSection() {
  return (
    <>
      <h1>Subscription</h1>
      <p className="lead">Your plan, billing, and usage.</p>

      <div className="s-section">
        <h2>Current Plan</h2>
        <div className="plan-card current">
          <div className="plan-info">
            <div className="plan-name">Rift Free</div>
            <div className="plan-price">No payment method on file.</div>
          </div>
          <span className="plan-badge">Current</span>
        </div>
        <div className="plan-card">
          <div className="plan-info">
            <div className="plan-name">Rift Pro</div>
            <div className="plan-price">$8/month · Unlimited file uploads, HD voice, custom themes</div>
          </div>
          <button className="s-btn primary">Upgrade</button>
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
  const [density, setDensity] = React.useState('cozy');
  const [size, setSize] = React.useState(14);
  return (
    <>
      <h1>Appearance</h1>
      <p className="lead">Theme, color, and how dense the UI feels.</p>

      <div className="s-section">
        <h2>Theme</h2>
        <div className="theme-grid">
          <button className={'theme-card' + (theme === 'light' ? ' on' : '')} onClick={() => onTheme('light')}>
            <div className="preview"><div className="pp-side"/><div className="pp-main"/></div>
            Light
          </button>
          <button className={'theme-card dark-pv' + (theme === 'dark' ? ' on' : '')} onClick={() => onTheme('dark')}>
            <div className="preview"><div className="pp-side"/><div className="pp-main"/></div>
            Dark
          </button>
          <button className={'theme-card system-pv'} onClick={() => onTheme(window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light')}>
            <div className="preview"><div className="pp-side"/><div className="pp-main"/></div>
            System
          </button>
        </div>
      </div>

      <div className="s-section">
        <h2>Accent Color</h2>
        <SettingsRow label="Highlight color" hint="Used for active items, mentions, and links.">
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
  return (
    <>
      <h1>Voice & Video</h1>
      <p className="lead">Set up the devices and behavior for voice channels.</p>

      <div className="s-section">
        <h2>Input Device</h2>
        <div className="s-field-row">
          <div className="field-lbl">Microphone</div>
          <input className="s-input" value={input} onChange={e => setInput(e.target.value)}/>
        </div>
        <SettingsRow label="Input volume" hint={`${inVol}%`}>
          <input type="range" min="0" max="100" value={inVol} onChange={e => setInVol(Number(e.target.value))}/>
        </SettingsRow>
      </div>

      <div className="s-section">
        <h2>Output Device</h2>
        <div className="s-field-row">
          <div className="field-lbl">Speaker</div>
          <input className="s-input" value={output} onChange={e => setOutput(e.target.value)}/>
        </div>
        <SettingsRow label="Output volume" hint={`${outVol}%`}>
          <input type="range" min="0" max="100" value={outVol} onChange={e => setOutVol(Number(e.target.value))}/>
        </SettingsRow>
      </div>

      <div className="s-section">
        <h2>Voice Activity</h2>
        <SettingsRow label="Voice mode" hint="Push-to-talk uses a hotkey instead of detecting your voice.">
          <div className="s-seg">
            <button className={!push ? 'on' : ''} onClick={() => setPush(false)}>Voice activity</button>
            <button className={push ? 'on' : ''} onClick={() => setPush(true)}>Push-to-talk</button>
          </div>
        </SettingsRow>
        <SettingsRow label="Noise suppression" hint="Filter typing, fans, and street sounds in real time.">
          <Switch on={true} onClick={() => {}}/>
        </SettingsRow>
        <SettingsRow label="Echo cancellation" hint="Recommended when not using headphones.">
          <Switch on={true} onClick={() => {}}/>
        </SettingsRow>
        <SettingsRow label="Auto gain control" hint="Keep your voice at a steady level.">
          <Switch on={false} onClick={() => {}}/>
        </SettingsRow>
      </div>

      <div className="s-section">
        <h2>Video</h2>
        <div className="s-field-row">
          <div className="field-lbl">Camera</div>
          <input className="s-input" defaultValue="FaceTime HD Camera"/>
        </div>
        <SettingsRow label="Mirror my video" hint="Flip your preview horizontally so it matches a mirror.">
          <Switch on={true} onClick={() => {}}/>
        </SettingsRow>
      </div>
    </>
  );
}

function NotificationsSection() {
  const [desk, setDesk] = React.useState(true);
  const [sound, setSound] = React.useState(true);
  const [mentions, setMentions] = React.useState('all');
  return (
    <>
      <h1>Notifications</h1>
      <p className="lead">Choose what reaches you, and when it stays quiet.</p>

      <div className="s-section">
        <h2>Desktop</h2>
        <SettingsRow label="Enable desktop notifications" hint="System banners for new messages.">
          <Switch on={desk} onClick={() => setDesk(v => !v)}/>
        </SettingsRow>
        <SettingsRow label="Notification sound" hint="Subtle chime on new messages and mentions.">
          <Switch on={sound} onClick={() => setSound(v => !v)}/>
        </SettingsRow>
        <SettingsRow label="Show badge on app icon" hint="Unread count in the dock.">
          <Switch on={true} onClick={() => {}}/>
        </SettingsRow>
      </div>

      <div className="s-section">
        <h2>What to notify on</h2>
        <SettingsRow label="Mentions" hint="When someone @you's or replies in a thread.">
          <div className="s-seg">
            <button className={mentions === 'all' ? 'on' : ''} onClick={() => setMentions('all')}>All</button>
            <button className={mentions === 'direct' ? 'on' : ''} onClick={() => setMentions('direct')}>Direct only</button>
            <button className={mentions === 'none' ? 'on' : ''} onClick={() => setMentions('none')}>None</button>
          </div>
        </SettingsRow>
        <SettingsRow label="Direct messages" hint="Always notify me of new DMs.">
          <Switch on={true} onClick={() => {}}/>
        </SettingsRow>
        <SettingsRow label="Voice channel joins" hint="When someone joins a voice channel I'm in.">
          <Switch on={false} onClick={() => {}}/>
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
  return (
    <>
      <h1>Keyboard Shortcuts</h1>
      <p className="lead">Get around faster. Most actions have a key.</p>
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
  const locale = useI18nStore((s) => s.locale);
  const setLocale = useI18nStore((s) => s.setLocale);
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
      <h1>Language</h1>
      <p className="lead">Pick the language Rift uses for menus and system messages.</p>
      <div className="s-section">
        <h2>Display Language</h2>
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
              <div className="hint">{l.v}</div>
            </div>
            <div className="s-row-control">
              {locale === l.v && (
                <Icon name="check" size={18} style={{ color: "var(--accent)" }} />
              )}
            </div>
          </button>
        ))}
      </div>
    </>
  );
}

const ALL_RAIL_IDS = SETTINGS_SECTIONS.flatMap(g => g.items.map(i => i.id));

function SettingsView({ open, theme, onTheme, accent, onAccent, onClose, onSignOut }: SettingsViewProps) {
  const [activeId, setActiveId] = React.useState('account');
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
    case 'account':       body = <AccountSection/>; break;
    case 'profile':       body = <ProfileSection/>; break;
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
        {SETTINGS_SECTIONS.map(group => (
          <React.Fragment key={group.group}>
            <div className="rail-section">{group.group}</div>
            {group.items.map(item => (
              <button key={item.id}
                ref={(el) => { railButtonRefs.current[item.id] = el; }}
                role="tab"
                aria-selected={activeId === item.id}
                className={'rail-item' + (activeId === item.id ? ' active' : '')}
                onClick={() => setActiveId(item.id)}>
                <span className="ic"><Icon name={item.icon} size={15}/></span>
                {item.label}
              </button>
            ))}
          </React.Fragment>
        ))}
        <div style={{ height: 1, background: 'var(--hairline)', margin: '12px 8px' }}/>
        <button className="rail-item danger" onClick={onSignOut}>
          <span className="ic"><Icon name="logout" size={15}/></span>
          Log Out
        </button>
      </nav>

      <div className="settings-pane">
        <button className="settings-close" onClick={onClose} title="Close (Esc)">
          <Icon name="x" size={16}/>
          <span className="esc">ESC</span>
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
