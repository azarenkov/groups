import React, { useEffect, useRef, useState } from "react";
import { useI18nStore } from "../../store/i18nStore";
import { SUPPORTED_LOCALES, type Locale } from "../../i18n/strings";
import Icon from "../rift/Icon";

interface Props {
  onChange?: (locale: Locale) => void;
}

export const LanguageSwitcher: React.FC<Props> = ({ onChange }) => {
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

  return (
    <div className="auth-lang" ref={ref}>
      <button
        type="button"
        className={"auth-lang-btn" + (open ? " open" : "")}
        onClick={() => setOpen((v) => !v)}
        aria-haspopup="listbox"
        aria-expanded={open}
      >
        <Icon name="globe" size={14} />
        <span>{locale.toUpperCase()}</span>
        <Icon name="chev" size={10} className={"auth-lang-chev" + (open ? " up" : "")} />
      </button>
      {open && (
        <div className="auth-lang-menu" role="listbox">
          {SUPPORTED_LOCALES.map((l) => (
            <button
              key={l}
              type="button"
              role="option"
              aria-selected={locale === l}
              className={"auth-lang-item" + (locale === l ? " on" : "")}
              onClick={() => {
                setLocale(l);
                setOpen(false);
                onChange?.(l);
              }}
            >
              <span>{l.toUpperCase()}</span>
              {locale === l && <Icon name="check" size={12} />}
            </button>
          ))}
        </div>
      )}
    </div>
  );
};

export default LanguageSwitcher;
