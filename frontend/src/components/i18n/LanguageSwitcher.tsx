import React, { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { SUPPORTED_LOCALES, type Locale, setLocale, currentLocale } from "../../i18n";
import Icon from "../rift/Icon";

interface Props {
  onChange?: (locale: Locale) => void;
}

export const LanguageSwitcher: React.FC<Props> = ({ onChange }) => {
  const { i18n } = useTranslation();
  const active = currentLocale();
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

  void i18n.language;

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
        <span>{active.toUpperCase()}</span>
        <Icon name="chev" size={10} className={"auth-lang-chev" + (open ? " up" : "")} />
      </button>
      {open && (
        <div className="auth-lang-menu" role="listbox">
          {SUPPORTED_LOCALES.map((l) => (
            <button
              key={l}
              type="button"
              role="option"
              aria-selected={active === l}
              className={"auth-lang-item" + (active === l ? " on" : "")}
              onClick={() => {
                setLocale(l);
                setOpen(false);
                onChange?.(l);
              }}
            >
              <span>{l.toUpperCase()}</span>
              {active === l && <Icon name="check" size={12} />}
            </button>
          ))}
        </div>
      )}
    </div>
  );
};

export default LanguageSwitcher;
