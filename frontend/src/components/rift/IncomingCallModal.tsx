import React, { useEffect } from "react";
import { useTranslation } from "react-i18next";
import Icon from "./Icon";
import { useCallStore } from "../../store/callStore";
import { UserAvatar } from "./Avatar";

const IncomingCallModal: React.FC = () => {
  const incoming = useCallStore((s) => s.incoming);
  const accept = useCallStore((s) => s.accept);
  const decline = useCallStore((s) => s.decline);
  const { t } = useTranslation("call");

  useEffect(() => {
    if (!incoming) return;
    const AudioCtx =
      window.AudioContext ||
      (window as unknown as { webkitAudioContext: typeof AudioContext })
        .webkitAudioContext;
    if (!AudioCtx) return;
    const ctx = new AudioCtx();
    let stopped = false;
    const notes = [
      { f: 587.33, at: 0, dur: 0.16 },
      { f: 783.99, at: 0.16, dur: 0.16 },
      { f: 1174.66, at: 0.32, dur: 0.3 },
    ];
    const ring = () => {
      if (stopped) return;
      const base = ctx.currentTime;
      for (const n of notes) {
        const osc = ctx.createOscillator();
        const gain = ctx.createGain();
        osc.type = "triangle";
        osc.frequency.value = n.f;
        const start = base + n.at;
        gain.gain.setValueAtTime(0.0001, start);
        gain.gain.exponentialRampToValueAtTime(0.13, start + 0.025);
        gain.gain.exponentialRampToValueAtTime(0.0001, start + n.dur);
        osc.connect(gain);
        gain.connect(ctx.destination);
        osc.start(start);
        osc.stop(start + n.dur + 0.05);
      }
    };
    ring();
    const id = setInterval(ring, 2200);
    return () => {
      stopped = true;
      clearInterval(id);
      ctx.close().catch(() => {});
    };
  }, [incoming]);

  if (!incoming) return null;

  return (
    <div className="incoming-call">
      <div className="ic-avatar-wrap">
        <UserAvatar
          userId={incoming.callerId}
          name={incoming.callerName}
          className="ic-ava"
        />
      </div>
      <div className="ic-text">
        <div className="ic-name">{incoming.callerName}</div>
        <div className="ic-sub">
          <span className="ic-sub-dot" />
          {t("incoming")}
        </div>
      </div>
      <div className="ic-actions">
        <button
          type="button"
          className="ic-btn decline"
          onClick={() => decline()}
          title={t("decline")}
        >
          <Icon name="phone" size={17} />
        </button>
        <button
          type="button"
          className="ic-btn accept"
          onClick={() => accept()}
          title={t("accept")}
        >
          <Icon name="phone" size={17} />
        </button>
      </div>
    </div>
  );
};

export default IncomingCallModal;
