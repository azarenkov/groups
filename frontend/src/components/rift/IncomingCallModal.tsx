import React, { useEffect } from "react";
import { useTranslation } from "react-i18next";
import Icon from "./Icon";
import { useCallStore } from "../../store/callStore";
import { colorForId, initialFor } from "../../utils/avatar";

const IncomingCallModal: React.FC = () => {
  const incoming = useCallStore((s) => s.incoming);
  const ringtoneMuted = useCallStore((s) => s.ringtoneMuted);
  const accept = useCallStore((s) => s.accept);
  const decline = useCallStore((s) => s.decline);
  const muteRingtone = useCallStore((s) => s.muteRingtone);
  const { t } = useTranslation("call");

  useEffect(() => {
    if (!incoming || ringtoneMuted) return;
    const AudioCtx =
      window.AudioContext ||
      (window as unknown as { webkitAudioContext: typeof AudioContext })
        .webkitAudioContext;
    if (!AudioCtx) return;
    const ctx = new AudioCtx();
    let stopped = false;
    const beep = () => {
      if (stopped) return;
      const osc = ctx.createOscillator();
      const gain = ctx.createGain();
      osc.type = "sine";
      osc.frequency.value = 480;
      const now = ctx.currentTime;
      gain.gain.setValueAtTime(0.0001, now);
      gain.gain.exponentialRampToValueAtTime(0.12, now + 0.05);
      gain.gain.exponentialRampToValueAtTime(0.0001, now + 0.9);
      osc.connect(gain);
      gain.connect(ctx.destination);
      osc.start(now);
      osc.stop(now + 1);
    };
    beep();
    const id = setInterval(beep, 2500);
    return () => {
      stopped = true;
      clearInterval(id);
      ctx.close().catch(() => {});
    };
  }, [incoming, ringtoneMuted]);

  if (!incoming) return null;

  return (
    <div className="incoming-call">
      <div
        className="incoming-call-ava"
        style={{ background: colorForId(incoming.callerId) }}
      >
        {initialFor(incoming.callerName)}
      </div>
      <div className="incoming-call-name">{incoming.callerName}</div>
      <div className="incoming-call-sub">{t("incoming")}</div>
      <div className="incoming-call-actions">
        <button
          type="button"
          className="ic-btn decline"
          onClick={() => decline()}
          title={t("decline")}
        >
          <Icon name="phone" size={18} />
        </button>
        <button
          type="button"
          className={"ic-btn mute" + (ringtoneMuted ? " on" : "")}
          onClick={() => muteRingtone()}
          title={t("mute")}
        >
          <Icon name="mic-off" size={15} />
        </button>
        <button
          type="button"
          className="ic-btn accept"
          onClick={() => accept()}
          title={t("accept")}
        >
          <Icon name="phone" size={18} />
        </button>
      </div>
    </div>
  );
};

export default IncomingCallModal;
