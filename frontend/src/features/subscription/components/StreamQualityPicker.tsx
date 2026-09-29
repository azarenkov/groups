import React from "react";
import { useTranslation } from "react-i18next";

import Icon from "../../../shared/ui/Icon";
import { useVoiceSettingsStore } from "../../voice/store/voiceSettingsStore";
import {
  STREAM_QUALITIES,
  bestAllowedQuality,
  ceilingFor,
  hasTier,
} from "../lib/plans";
import { useEntitlementsStore } from "../store/entitlementsStore";
import TrialOffer from "./TrialOffer";

const StreamQualityPicker: React.FC = () => {
  const { t } = useTranslation("subscription");
  const tier = useEntitlementsStore((s) => s.tier);
  const wanted = useVoiceSettingsStore((s) => s.streamQuality);
  const update = useVoiceSettingsStore((s) => s.update);
  const [open, setOpen] = React.useState(false);
  const wrapRef = React.useRef<HTMLDivElement>(null);
  const auto = wanted === "auto";
  const active = bestAllowedQuality(wanted, tier);

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
                {t("streamQualityAutoHint", { label: ceilingFor(tier).label })}
              </div>
            </span>
          </button>
          {STREAM_QUALITIES.map((quality) => {
            const allowed = hasTier(tier, quality.requires);
            return (
              <button
                key={quality.id}
                type="button"
                role="menuitemradio"
                aria-checked={!auto && quality.id === active.id}
                className={"pop-item" + (allowed ? "" : " locked")}
                disabled={!allowed}
                title={allowed ? undefined : t(`needs.${quality.requires}`)}
                onClick={() => {
                  update({ streamQuality: quality.id });
                  setOpen(false);
                }}
              >
                <span className="check">
                  {!auto && quality.id === active.id && <Icon name="check" size={13} />}
                </span>
                <span className="body">{quality.label}</span>
                {!allowed && (
                  <span className="stream-quality-lock">
                    <Icon name="lock" size={11} />
                    {quality.requires === "premium" ? "PREMIUM" : "PRO"}
                  </span>
                )}
              </button>
            );
          })}
          {tier === "free" && <TrialOffer compact reason={t("trial.streamReason")} />}
        </div>
      )}
    </div>
  );
};

export default StreamQualityPicker;
