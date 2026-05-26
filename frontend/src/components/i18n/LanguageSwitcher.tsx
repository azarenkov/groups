import React, { useEffect, useRef, useState } from "react";
import { useI18nStore } from "../../store/i18nStore";
import { LOCALE_LABELS, SUPPORTED_LOCALES, type Locale } from "../../i18n/strings";

interface Props {
  variant?: "minimal" | "settings";
  onChange?: (locale: Locale) => void;
}

export const LanguageSwitcher: React.FC<Props> = ({ variant = "minimal", onChange }) => {
  const locale = useI18nStore((s) => s.locale);
  const setLocale = useI18nStore((s) => s.setLocale);
  const [open, setOpen] = useState(false);
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!open) return;
    const onDoc = (e: MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) setOpen(false);
    };
    document.addEventListener("mousedown", onDoc);
    return () => document.removeEventListener("mousedown", onDoc);
  }, [open]);

  if (variant === "settings") {
    return (
      <div className="settings-lang-grid">
        {SUPPORTED_LOCALES.map((l) => (
          <button
            key={l}
            type="button"
            className={"theme-card" + (locale === l ? " on" : "")}
            onClick={() => {
              setLocale(l);
              onChange?.(l);
            }}
            style={{
              minHeight: 72,
              display: "flex",
              flexDirection: "column",
              alignItems: "center",
              justifyContent: "center",
              gap: 4,
            }}
          >
            <span style={{ fontSize: 22 }}>{l === "ru" ? "🇷🇺" : "🇺🇸"}</span>
            <span style={{ fontSize: 13 }}>{LOCALE_LABELS[l]}</span>
          </button>
        ))}
      </div>
    );
  }

  return (
    <div className="auth-lang" ref={ref} style={{ position: "relative" }}>
      <button
        type="button"
        className="auth-link"
        onClick={() => setOpen((v) => !v)}
        style={{
          display: "inline-flex",
          alignItems: "center",
          gap: 6,
          padding: "6px 10px",
          borderRadius: 8,
          background: "transparent",
          border: "1px solid var(--border, rgba(255,255,255,0.1))",
          fontSize: 12,
        }}
      >
        <span>{locale === "ru" ? "🇷🇺" : "🇺🇸"}</span>
        <span>{LOCALE_LABELS[locale]}</span>
        <span style={{ fontSize: 10 }}>▾</span>
      </button>
      {open && (
        <div
          style={{
            position: "absolute",
            right: 0,
            top: "calc(100% + 4px)",
            background: "var(--bg-2, #1a1a1a)",
            border: "1px solid var(--border, rgba(255,255,255,0.1))",
            borderRadius: 8,
            padding: 4,
            minWidth: 140,
            zIndex: 50,
            boxShadow: "0 6px 24px rgba(0,0,0,0.4)",
          }}
        >
          {SUPPORTED_LOCALES.map((l) => (
            <button
              key={l}
              type="button"
              onClick={() => {
                setLocale(l);
                setOpen(false);
                onChange?.(l);
              }}
              style={{
                display: "flex",
                width: "100%",
                alignItems: "center",
                gap: 8,
                padding: "8px 10px",
                background: locale === l ? "var(--bg-3, rgba(255,255,255,0.06))" : "transparent",
                border: "none",
                borderRadius: 6,
                cursor: "pointer",
                color: "inherit",
                fontSize: 13,
                textAlign: "left",
              }}
            >
              <span>{l === "ru" ? "🇷🇺" : "🇺🇸"}</span>
              <span>{LOCALE_LABELS[l]}</span>
            </button>
          ))}
        </div>
      )}
    </div>
  );
};

export default LanguageSwitcher;
