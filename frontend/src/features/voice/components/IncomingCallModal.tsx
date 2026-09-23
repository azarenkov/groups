import React, { useEffect, useRef } from "react";
import { useTranslation } from "react-i18next";
import Icon from "../../../shared/ui/Icon";
import { useCallStore } from "../store/callStore";
import { useVoiceChannelStore } from "../store/voiceChannelStore";
import {
  callAlertsAllowed,
  desktopNotifAllowed,
  isQuietNow,
  useNotificationSettingsStore,
} from "../../settings/store/notificationSettingsStore";
import { showDesktopNotification } from "../../../shared/lib/desktopNotification";
import { UserAvatar } from "../../../shared/ui/Avatar";

const IncomingCallModal: React.FC = () => {
  const incoming = useCallStore((s) => s.incoming);
  const accept = useCallStore((s) => s.accept);
  const decline = useCallStore((s) => s.decline);
  const dismissIncoming = useCallStore((s) => s.dismissIncoming);
  const channelName = useVoiceChannelStore((s) => s.name);
  const { t } = useTranslation("call");
  const callsEnabled = useNotificationSettingsStore((s) => s.callsEnabled);
  const quietHoursEnabled = useNotificationSettingsStore((s) => s.quietHoursEnabled);
  const quietStart = useNotificationSettingsStore((s) => s.quietStart);
  const quietEnd = useNotificationSettingsStore((s) => s.quietEnd);
  const syncedOwner = useNotificationSettingsStore((s) => s.syncedOwner);
  const desktopShownForRef = useRef<string | null>(null);

  useEffect(() => {
    if (!incoming) return;
    if (!callsEnabled || !callAlertsAllowed()) return;
    const notifyDesktop = () => {
      if (desktopShownForRef.current === incoming.callId) return;
      if (!desktopNotifAllowed()) return;
      desktopShownForRef.current = incoming.callId;
      showDesktopNotification(incoming.callerName, t("incoming"));
    };
    notifyDesktop();
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
      notifyDesktop();
      if (isQuietNow()) return;
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
  }, [incoming, t, callsEnabled, quietHoursEnabled, quietStart, quietEnd, syncedOwner]);

  useEffect(() => {
    if (!incoming) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key !== "Escape") return;
      e.preventDefault();
      e.stopImmediatePropagation();
      dismissIncoming();
    };
    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
  }, [incoming, dismissIncoming]);

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
          {incoming.video ? t("incomingVideo") : t("incoming")}
        </div>
      </div>
      {channelName && (
        <div className="ic-note">{t("leavesChannel", { channel: channelName })}</div>
      )}
      <div className="ic-actions">
        <button
          type="button"
          className="ic-btn ignore"
          onClick={() => dismissIncoming()}
          title={t("dismiss")}
          aria-label={t("dismiss")}
        >
          <Icon name="bell-off" size={17} />
        </button>
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
          className={"ic-btn accept" + (incoming.video ? " secondary" : "")}
          onClick={() => accept(false)}
          title={
            channelName
              ? incoming.video
                ? t("acceptAudioAndLeave", { channel: channelName })
                : t("acceptAndLeave", { channel: channelName })
              : incoming.video
                ? t("acceptAudioOnly")
                : t("accept")
          }
        >
          <Icon name="phone" size={17} />
        </button>
        {incoming.video && (
          <button
            type="button"
            className="ic-btn accept"
            onClick={() => accept(true)}
            title={
              channelName
                ? t("acceptVideoAndLeave", { channel: channelName })
                : t("acceptWithVideo")
            }
          >
            <Icon name="video" size={17} />
          </button>
        )}
      </div>
    </div>
  );
};

export default IncomingCallModal;
