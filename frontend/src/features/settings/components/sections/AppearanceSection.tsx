import React from "react";
import { useTranslation } from "react-i18next";
import { useAppearanceStore } from "../../store/appearanceStore";

import { ACCENT_OPTIONS, SettingsRow, Switch } from "../controls";

export function AppearanceSection() {
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
