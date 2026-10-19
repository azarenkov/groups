import React from "react";
import { useTranslation } from "react-i18next";

import Icon from "../../../shared/ui/Icon";
import { MAX_STREAM_QUALITY, STREAM_QUALITIES, resolveStreamQuality } from "../lib/streamQuality";
import { useVoiceSettingsStore } from "../store/voiceSettingsStore";

const StreamQualityPicker: React.FC = () => {
  const { t } = useTranslation("voice");
  const wanted = useVoiceSettingsStore((s) => s.streamQuality);
  const update = useVoiceSettingsStore((s) => s.update);
  const [open, setOpen] = React.useState(false);
  const wrapRef = React.useRef<HTMLDivElement>(null);
  const auto = wanted === "auto";
  const active = resolveStreamQuality(wanted);

  React.useEffect(() => {
    if (!open) return;
    const onDoc = (e: MouseEvent) => {
      if (wrapRef.current && !wrapRef.current.contains(e.target as Node)) setOpen(false);
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") setOpen(false);
    };
    document.addEventListener("mousedown", onDoc);
    window.addEventListener("keydown", onKey);
    return () => {
      document.removeEventListener("mousedown", onDoc);
      window.removeEventListener("keydown", onKey);
    };
  }, [open]);

  return (
    <div className="call-invite" ref={wrapRef}>
      <button
        type="button"
        className={"cp-btn stream-quality-btn" + (open ? " active" : "")}
        title={t("streamQuality")}
        aria-haspopup="menu"
        aria-expanded={open}
        onClick={() => setOpen((o) => !o)}
      >
        <span className="stream-quality-label">
          {auto ? t("streamQualityAuto") : active.label}
        </span>
      </button>
      {open && (
        <div className="popover call-invite-pop stream-quality-pop" role="menu">
          <div className="pop-head">{t("streamQuality")}</div>
          <button
            type="button"
            role="menuitemradio"
            aria-checked={auto}
            className="pop-item"
            onClick={() => {
              update({ streamQuality: "auto" });
              setOpen(false);
            }}
          >
            <span className="check">{auto && <Icon name="check" size={13} />}</span>
            <span className="body">
              <div className="ttl">{t("streamQualityAuto")}</div>
              <div className="sub">
                {t("streamQualityAutoHint", { label: MAX_STREAM_QUALITY.label })}
              </div>
            </span>
          </button>
          {STREAM_QUALITIES.map((quality) => (
            <button
              key={quality.id}
              type="button"
              role="menuitemradio"
              aria-checked={!auto && quality.id === active.id}
              className="pop-item"
              onClick={() => {
                update({ streamQuality: quality.id });
                setOpen(false);
              }}
            >
              <span className="check">
                {!auto && quality.id === active.id && <Icon name="check" size={13} />}
              </span>
              <span className="body">{quality.label}</span>
            </button>
          ))}
        </div>
      )}
    </div>
  );
};

export default StreamQualityPicker;
