import React, { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import Icon from "./Icon";
import { useLocalTrack } from "../../hooks/useLocalTrack";
import { getLiveKitService, NoiseSuppressionMode } from "../../services/livekit";
import { useVoiceSettingsStore } from "../../store/voiceSettingsStore";
import { UserAvatar } from "./Avatar";

export interface VoicePillInvite {
  candidates: { id: string; name: string }[];
  onAdd: (ids: string[]) => void;
  labels: { title: string; add: string; cancel: string; empty: string };
}

interface VoicePillProps {
  title: string;
  meta: string;
  isMuted: boolean;
  muteLocked?: boolean;
  onToggleMute: () => void;
  onLeave: () => void;
  className?: string;
  titleAttr?: string;
  invite?: VoicePillInvite;
  onOpen?: () => void;
  openTitle?: string;
}

const BARS = 12;

const VoicePill: React.FC<VoicePillProps> = ({
  title,
  meta,
  isMuted,
  muteLocked,
  onToggleMute,
  onLeave,
  className,
  titleAttr,
  invite,
  onOpen,
  openTitle,
}) => {
  const { t } = useTranslation("voice");
  const rootRef = useRef<HTMLDivElement>(null);
  const [open, setOpen] = useState(false);
  const [inviteOpen, setInviteOpen] = useState(false);
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [noise, setNoise] = useState<NoiseSuppressionMode>(() =>
    getLiveKitService().getNoiseSuppressionMode(),
  );
  const [noiseStrengthPct, setNoiseStrengthPct] = useState(50);
  const [meter, setMeter] = useState<number[]>(() => new Array(BARS).fill(0));
  const localTrack = useLocalTrack();
  const trackId = localTrack?.id ?? "";
  const openRef = useRef(open);
  openRef.current = open;

  const handleNoiseChange = async (next: NoiseSuppressionMode) => {
    setNoise(next);
    useVoiceSettingsStore.getState().update({ noiseMode: next });
    if (next === "high") {
      setNoiseStrengthPct(50);
      getLiveKitService().setNoiseStrength(0.5);
    }
    try {
      await getLiveKitService().setNoiseSuppressionMode(next);
    } catch (e) {
      console.warn("[VoicePill] noise suppression toggle failed:", e);
    }
    const actual = getLiveKitService().getNoiseSuppressionMode();
    if (actual !== next) setNoise(actual);
  };

  const handleStrengthChange = (pct: number) => {
    setNoiseStrengthPct(pct);
    getLiveKitService().setNoiseStrength(pct / 100);
  };

  useEffect(() => {
    if (open) setNoise(getLiveKitService().getNoiseSuppressionMode());
  }, [open]);

  const anyOpen = open || inviteOpen;
  useEffect(() => {
    if (!anyOpen) return;
    const onDoc = (e: MouseEvent) => {
      if (rootRef.current && !rootRef.current.contains(e.target as Node)) {
        setOpen(false);
        setInviteOpen(false);
      }
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        setOpen(false);
        setInviteOpen(false);
      }
    };
    document.addEventListener("mousedown", onDoc);
    document.addEventListener("keydown", onKey);
    return () => {
      document.removeEventListener("mousedown", onDoc);
      document.removeEventListener("keydown", onKey);
    };
  }, [anyOpen]);

  useEffect(() => {
    const service = getLiveKitService();
    if (isMuted || !localTrack) {
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
      if (cancelled || !localTrack) return;
      const AudioCtx =
        window.AudioContext ||
        (window as unknown as { webkitAudioContext: typeof AudioContext })
          .webkitAudioContext;
      try {
        ctx = new AudioCtx();
        source = ctx.createMediaStreamSource(new MediaStream([localTrack]));
        analyser = ctx.createAnalyser();
        analyser.fftSize = 256;
        analyser.smoothingTimeConstant = 0.75;
        source.connect(analyser);
      } catch (e) {
        console.error("[VoicePill] failed to set up audio graph:", e);
        return;
      }

      void ctx.resume().catch(() => {});

      const bins = analyser.frequencyBinCount;
      const data = new Uint8Array(bins);
      const usable = Math.floor(bins * 0.6);
      const perBar = Math.max(1, Math.floor(usable / BARS));
      const speakThresholds = () => {
        const { autoSensitivity, sensitivity } =
          useVoiceSettingsStore.getState();
        const on = autoSensitivity ? 22 : 45 - 0.4 * sensitivity;
        return { on, off: autoSensitivity ? 12 : on * 0.55 };
      };
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
          ? maxSample > speakThresholds().off ||
            now - lastSpeakingChangeAt < SPEAK_RELEASE_MS
          : maxSample > speakThresholds().on;
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
  }, [isMuted, trackId, localTrack]);

  const toggleSelected = (id: string) =>
    setSelected((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });

  const confirmAdd = () => {
    const ids = Array.from(selected);
    setInviteOpen(false);
    setSelected(new Set());
    if (ids.length > 0) invite?.onAdd(ids);
  };

  return (
    <div
      className={`voice-pill in-dock connected${className ? ` ${className}` : ""}`}
      ref={rootRef}
      title={titleAttr}
    >
      <div
        className={"vp-meta-wrap" + (onOpen ? " clickable" : "")}
        onClick={onOpen}
        title={onOpen ? openTitle : undefined}
        role={onOpen ? "button" : undefined}
      >
        <div className="vp-title">{title}</div>
        <div className="vp-meta">{meta}</div>
      </div>
      <div className="vp-actions">
        {invite && (
          <button
            type="button"
            className={"vp-btn" + (inviteOpen ? " on" : "")}
            onClick={() => {
              setInviteOpen((v) => !v);
              setOpen(false);
            }}
            title={invite.labels.title}
          >
            <Icon name="user-plus" size={15} />
          </button>
        )}
        <button
          type="button"
          className={"vp-btn" + (open ? " on" : "")}
          onClick={() => {
            setOpen((v) => !v);
            setInviteOpen(false);
          }}
          title={t("settings")}
        >
          <Icon name="speaker-hi" size={15} />
        </button>
        <button
          type="button"
          className={"vp-btn" + (isMuted ? " muted" : "")}
          onClick={onToggleMute}
          disabled={muteLocked}
          title={
            muteLocked
              ? t("mutedByModerator")
              : isMuted
                ? t("unmuteSelf")
                : t("muteSelf")
          }
        >
          <Icon name={isMuted ? "mic-off" : "mic"} size={15} />
        </button>
      </div>
      <button
        type="button"
        className="vp-btn leave"
        onClick={onLeave}
        title={t("leave")}
      >
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
                  style={
                    {
                      ["--pct" as unknown as string]: `${noiseStrengthPct}%`,
                    } as React.CSSProperties
                  }
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

      {invite && inviteOpen && (
        <div className="acb-add" onMouseDown={(e) => e.stopPropagation()}>
          <div className="acb-add-title">{invite.labels.title}</div>
          <div className="acb-add-list">
            {invite.candidates.length === 0 && (
              <div className="acb-add-empty">{invite.labels.empty}</div>
            )}
            {invite.candidates.map((c) => (
              <button
                key={c.id}
                type="button"
                className={"acb-add-item" + (selected.has(c.id) ? " on" : "")}
                onClick={() => toggleSelected(c.id)}
              >
                <UserAvatar
                  userId={c.id}
                  name={c.name}
                  className="acb-add-ava"
                  style={{ display: "inline-flex" }}
                />
                <span className="acb-add-name">{c.name}</span>
                {selected.has(c.id) && <Icon name="check" size={14} />}
              </button>
            ))}
          </div>
          <div className="acb-add-foot">
            <button
              type="button"
              className="btn"
              onClick={() => setInviteOpen(false)}
            >
              {invite.labels.cancel}
            </button>
            <button
              type="button"
              className="btn primary"
              disabled={selected.size === 0}
              onClick={confirmAdd}
            >
              {invite.labels.add}
            </button>
          </div>
        </div>
      )}
    </div>
  );
};

export default VoicePill;
