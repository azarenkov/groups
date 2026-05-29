import React, { useEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { useTranslation } from "react-i18next";
import Icon from "./Icon";
import { useLiveKit } from "../../hooks/useLiveKit";
import { useLocalTrack } from "../../hooks/useLocalTrack";
import { getLiveKitService } from "../../services/livekit";
import { qualityClass, qualityLabel } from "../../utils/quality";
import { useVoicePresenceStore } from "../../store/voicePresenceStore";

interface VoiceViewProps {
  channelId: string;
  serverId: string;
  channelName: string;
  password?: string;
  onLeave: () => void;
  onJoinFailed?: (message: string) => void;
}

const VoiceView: React.FC<VoiceViewProps> = ({
  channelId,
  serverId,
  channelName,
  password,
  onLeave,
  onJoinFailed,
}) => {
  const { t } = useTranslation(["voice", "common"]);
  const onLeaveRef = useRef(onLeave);
  onLeaveRef.current = onLeave;
  const {
    participants,
    isConnected,
    isMuted,
    error,
    localQuality,
    joinChannel,
    leaveChannel,
    toggleMute,
    setNoiseSuppression,
    setNoiseStrength,
    getNoiseSuppressionMode,
  } = useLiveKit({
    onUnexpectedDisconnect: (reason) => {
      console.warn("[VoiceView] unexpected disconnect, leaving:", reason);
      onLeaveRef.current();
    },
  });

  const hasJoinedRef = useRef(false);
  const currentChannelRef = useRef<string | null>(null);
  const isJoiningRef = useRef(false);
  const [mountPoint, setMountPoint] = useState<HTMLElement | null>(null);

  useEffect(() => {
    setMountPoint(document.getElementById("voice-dock-slot"));
  }, []);

  useEffect(() => {
    const shouldJoin =
      (!hasJoinedRef.current || currentChannelRef.current !== channelId) &&
      !isJoiningRef.current;

    if (shouldJoin) {
      hasJoinedRef.current = true;
      currentChannelRef.current = channelId;
      isJoiningRef.current = true;
      joinChannel(channelId, serverId, password)
        .then(() => {
          isJoiningRef.current = false;
        })
        .catch((err) => {
          console.error("[VoiceView] join failed:", err);
          hasJoinedRef.current = false;
          isJoiningRef.current = false;
          const message =
            err instanceof Error ? err.message : "Failed to join voice channel";
          onJoinFailed?.(message);
        });
    }

    return () => {
      hasJoinedRef.current = false;
      currentChannelRef.current = null;
      isJoiningRef.current = false;
      leaveChannel();
    };
  }, [channelId, serverId, password, joinChannel, leaveChannel, onJoinFailed]);

  const handleLeave = async () => {
    await leaveChannel();
    onLeave();
  };

  const [open, setOpen] = React.useState(false);
  const [noise, setNoise] = React.useState<"off" | "standard" | "high">(() =>
    getNoiseSuppressionMode(),
  );
  const [noiseStrengthPct, setNoiseStrengthPct] = React.useState(50);
  const localTrack = useLocalTrack();
  const trackId = localTrack?.id ?? "";

  const handleNoiseChange = async (next: "off" | "standard" | "high") => {
    setNoise(next);
    if (next === "high") {
      setNoiseStrengthPct(50);
      setNoiseStrength(0.5);
    }
    try {
      await setNoiseSuppression(next);
    } catch (e) {
      console.warn("[VoiceView] noise suppression toggle failed:", e);
    }
    const actual = getNoiseSuppressionMode();
    if (actual !== next) {
      setNoise(actual);
    }
  };

  const handleStrengthChange = (pct: number) => {
    setNoiseStrengthPct(pct);
    setNoiseStrength(pct / 100);
  };
  const rootRef = useRef<HTMLDivElement>(null);

  const speakingPerson = participants.find((p) => p.isSpeaking);

  const volIcon = "speaker-hi";

  React.useEffect(() => {
    if (!open) return;
    const onDoc = (e: MouseEvent) => {
      if (rootRef.current && !rootRef.current.contains(e.target as Node)) setOpen(false);
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") setOpen(false);
    };
    document.addEventListener("mousedown", onDoc);
    document.addEventListener("keydown", onKey);
    return () => {
      document.removeEventListener("mousedown", onDoc);
      document.removeEventListener("keydown", onKey);
    };
  }, [open]);

  const BARS = 12;
  const [meter, setMeter] = React.useState<number[]>(() =>
    new Array(BARS).fill(0),
  );
  const openRef = useRef(open);
  useEffect(() => {
    openRef.current = open;
  }, [open]);

  useEffect(() => {
    const service = getLiveKitService();
    if (isMuted || !isConnected || !localTrack) {
      setMeter(new Array(BARS).fill(0));
      service.setLocalSpeaking(false);
      return;
    }

    let cancelled = false;
    let ctx: AudioContext | null = null;
    let source: MediaStreamAudioSourceNode | null = null;
    let analyser: AnalyserNode | null = null;
    let raf = 0;
    let lastSpeaking = false;
    let lastSpeakingChangeAt = 0;

    const teardown = () => {
      if (raf) cancelAnimationFrame(raf);
      raf = 0;
      try {
        source?.disconnect();
      } catch {}
      source = null;
      analyser = null;
      const oldCtx = ctx;
      ctx = null;
      oldCtx?.close().catch(() => {});
    };

    const start = () => {
      if (cancelled) return;
      const track = localTrack;
      if (!track) return;

      const AudioCtx =
        window.AudioContext ||
        (window as unknown as { webkitAudioContext: typeof AudioContext })
          .webkitAudioContext;
      try {
        ctx = new AudioCtx();
        source = ctx.createMediaStreamSource(new MediaStream([track]));
        analyser = ctx.createAnalyser();
        analyser.fftSize = 256;
        analyser.smoothingTimeConstant = 0.75;
        source.connect(analyser);
      } catch (e) {
        console.error("[meter] failed to set up audio graph:", e);
        return;
      }

      void ctx.resume().catch(() => {});

      const bins = analyser.frequencyBinCount;
      const data = new Uint8Array(bins);
      const usable = Math.floor(bins * 0.6);
      const perBar = Math.max(1, Math.floor(usable / BARS));
      const SPEAK_ON = 22;
      const SPEAK_OFF = 12;
      const SPEAK_RELEASE_MS = 250;

      const tick = () => {
        if (!analyser || cancelled) return;
        analyser.getByteFrequencyData(data);
        let maxSample = 0;
        const next = new Array(BARS).fill(0) as number[];
        for (let b = 0; b < BARS; b++) {
          let sum = 0;
          const offset = b * perBar;
          for (let j = 0; j < perBar; j++) {
            const v = data[offset + j] || 0;
            sum += v;
            if (v > maxSample) maxSample = v;
          }
          next[b] = Math.min(1, sum / perBar / 200);
        }
        if (openRef.current) setMeter(next);

        const now = performance.now();
        const wantSpeaking = lastSpeaking
          ? maxSample > SPEAK_OFF ||
            now - lastSpeakingChangeAt < SPEAK_RELEASE_MS
          : maxSample > SPEAK_ON;
        if (wantSpeaking !== lastSpeaking) {
          lastSpeaking = wantSpeaking;
          lastSpeakingChangeAt = now;
          service.setLocalSpeaking(wantSpeaking);
        }
        raf = requestAnimationFrame(tick);
      };
      raf = requestAnimationFrame(tick);
    };

    start();

    return () => {
      cancelled = true;
      teardown();
      service.setLocalSpeaking(false);
    };
  }, [isMuted, isConnected, trackId, localTrack]);

  const presenceForChannel = useVoicePresenceStore(
    (s) => s.byChannel[channelId],
  );
  const livekitCount = participants.length + 1;
  const presenceCount = presenceForChannel?.length ?? 0;
  const totalConnected = isConnected
    ? Math.max(livekitCount, presenceCount)
    : 0;

  if (!mountPoint) return null;

  if (error) {
    return createPortal(
      <div
        className="voice-pill in-dock"
        style={{ background: "var(--danger-tint)", color: "var(--danger)" }}
      >
        <span>{error}</span>
        <button className="vp-btn leave" onClick={handleLeave} title={t("leave")}>
          <Icon name="logout" size={15} />
        </button>
      </div>,
      mountPoint,
    );
  }

  if (!isConnected) {
    return createPortal(
      <div className="voice-pill in-dock connecting">
        <div className="vp-meta-wrap">
          <div className="vp-title">{channelName}</div>
          <div className="vp-meta">{t("connecting")}</div>
        </div>
        <button className="vp-btn leave" onClick={handleLeave} title={t("common:cancel")}>
          <Icon name="logout" size={15} />
        </button>
      </div>,
      mountPoint,
    );
  }

  return createPortal(
    <div
      className={`voice-pill in-dock connected ${qualityClass(localQuality)}`}
      ref={rootRef}
      title={`${t("connected")}: ${qualityLabel(localQuality)}`}
    >
      <div className="vp-meta-wrap">
        <div className="vp-title">{channelName}</div>
        <div className="vp-meta">
          {speakingPerson
            ? `${speakingPerson.name || speakingPerson.identity} — ${t("speaking")}`
            : `${totalConnected} ${t("connected").toLowerCase()}`}
        </div>
      </div>
      <div className="vp-actions">
        <button
          className={"vp-btn" + (open ? " on" : "")}
          onClick={() => setOpen((v) => !v)}
          title={t("settings")}
        >
          <Icon name={volIcon} size={15} />
        </button>
        <button
          className={"vp-btn" + (isMuted ? " muted" : "")}
          onClick={toggleMute}
          title={isMuted ? t("unmuteSelf") : t("muteSelf")}
        >
          <Icon name={isMuted ? "mic-off" : "mic"} size={15} />
        </button>
      </div>
      <button className="vp-btn leave" onClick={handleLeave} title={t("leave")}>
        <Icon name="logout" size={15} />
      </button>

      {open && (
        <div className="vp-popover" onMouseDown={(e) => e.stopPropagation()}>
          <div className="vp-pop-row">
            <div className="vp-pop-label">
              <span className="k">{t("noiseSuppression")}</span>
            </div>
            <div className="vp-seg">
              <button
                className={noise === "off" ? "on" : ""}
                onClick={() => handleNoiseChange("off")}
              >
                {t("noiseOff")}
              </button>
              <button
                className={noise === "standard" ? "on" : ""}
                onClick={() => handleNoiseChange("standard")}
              >
                {t("noiseStandard")}
              </button>
              <button
                className={noise === "high" ? "on" : ""}
                onClick={() => handleNoiseChange("high")}
              >
                <Icon name="sparkles" size={11} />
                {t("noiseHigh")}
              </button>
            </div>
          </div>

          {noise === "high" && (
            <div className="vp-pop-row">
              <div className="vp-pop-label">
                <span className="k">{t("noiseSuppression")}</span>
                <span className="v">{noiseStrengthPct}%</span>
              </div>
              <div className="vp-slider accent">
                <input
                  type="range"
                  min={0}
                  max={100}
                  step={1}
                  value={noiseStrengthPct}
                  onChange={(e) => handleStrengthChange(Number(e.target.value))}
                  style={{ ["--pct" as unknown as string]: `${noiseStrengthPct}%` } as React.CSSProperties}
                  title={`${noiseStrengthPct}%`}
                />
              </div>
            </div>
          )}

          <div className="vp-pop-row">
            <div className="vp-pop-label">
              <span className="k">{t("yourMicrophone")}</span>
              <span className="v">{isMuted ? t("muted") : t("live")}</span>
            </div>
            <div className="vp-meter" aria-hidden="true">
              {meter.map((v, i) => (
                <i
                  key={i}
                  className={v > 0.85 ? "hot" : v > 0.08 ? "on" : ""}
                  style={{ height: `${Math.max(10, v * 100)}%` }}
                />
              ))}
            </div>
          </div>
        </div>
      )}
    </div>,
    mountPoint,
  );
};

export default VoiceView;
