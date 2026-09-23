import React, { useEffect, useRef, useState } from "react";
import { useEntitlementsStore } from "../../subscription/store/entitlementsStore";
import { useTranslation } from "react-i18next";
import Icon from "../../../shared/ui/Icon";
import { useLocalTrack } from "../hooks/useLocalTrack";
import { usePushToTalkHeld } from "../hooks/usePushToTalkHeld";
import { getLiveKitService, NoiseSuppressionMode } from "../lib/livekit";
import { useVoiceSettingsStore } from "../store/voiceSettingsStore";
import { UserAvatar } from "../../../shared/ui/Avatar";

export interface VoicePillInvite {
  candidates: { id: string; name: string }[];
  onAdd: (ids: string[]) => void;
  onOpen?: () => void;
  labels: { title: string; add: string; cancel: string; empty: string };
}

interface VoicePillProps {
  title: string;
  meta: string;
  isMuted: boolean;
  muteLocked?: boolean;
  onToggleMute: () => void;
  isDeafened?: boolean;
  deafenLocked?: boolean;
  onToggleDeafen?: () => void;
  onLeave: () => void;
  className?: string;
  titleAttr?: string;
  invite?: VoicePillInvite;
  onOpen?: () => void;
  openTitle?: string;
  active?: boolean;
  badge?: boolean;
}

const BARS = 12;

const VoicePill: React.FC<VoicePillProps> = ({
  title,
  meta,
  isMuted,
  muteLocked,
  onToggleMute,
  isDeafened,
  deafenLocked,
  onToggleDeafen,
  onLeave,
  className,
  titleAttr,
  invite,
  onOpen,
  openTitle,
  active,
  badge,
}) => {
  const { t } = useTranslation("voice");
  const { t: tSubscription } = useTranslation("subscription");
  const paid = useEntitlementsStore((state) => state.tier !== "free");
  const rootRef = useRef<HTMLDivElement>(null);
  const [open, setOpen] = useState(false);
  const [inviteOpen, setInviteOpen] = useState(false);
  const onInviteOpen = invite?.onOpen;
  useEffect(() => {
    if (inviteOpen) onInviteOpen?.();
  }, [inviteOpen, onInviteOpen]);
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [noise, setNoise] = useState<NoiseSuppressionMode>(() =>
    getLiveKitService().getNoiseSuppressionMode(),
  );
  const [meter, setMeter] = useState<number[]>(() => new Array(BARS).fill(0));
  const localTrack = useLocalTrack();
  const pushToTalk = useVoiceSettingsStore((state) => state.pushToTalk);
  const pttHeld = usePushToTalkHeld();
  const pttLive = pushToTalk && pttHeld;
  const trackId = localTrack?.id ?? "";
  const openRef = useRef(open);
  openRef.current = open;

  const handleNoiseChange = async (next: NoiseSuppressionMode) => {
    setNoise(next);
    useVoiceSettingsStore.getState().update({ noiseMode: next });
    try {
      await getLiveKitService().setNoiseSuppressionMode(next);
    } catch (e) {
      console.warn("[VoicePill] noise suppression toggle failed:", e);
    }
    const actual = getLiveKitService().getNoiseSuppressionMode();
    if (actual !== next) {
      setNoise(actual);
      const store = useVoiceSettingsStore.getState();
      if (store.noiseMode === next) store.update({ noiseMode: actual });
    }
  };

  useEffect(() => {
    if (!open) return;
    const service = getLiveKitService();
    setNoise(service.getNoiseSuppressionMode());
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
      if (e.key !== "Escape") return;
      e.preventDefault();
      e.stopPropagation();
      setOpen(false);
      setInviteOpen(false);
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
      if (!service.micGateActive()) service.setLocalSpeaking(false);
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
      const tdata = new Uint8Array(analyser.fftSize);
      const usable = Math.floor(bins * 0.6);
      const perBar = Math.max(1, Math.floor(usable / BARS));
      const speakLevels = (maxSample: number) => {
        const { autoSensitivity, sensitivity } =
          useVoiceSettingsStore.getState();
        if (autoSensitivity || !analyser) {
          return { above: maxSample > 22, aboveOff: maxSample > 12 };
        }
        analyser.getByteTimeDomainData(tdata);
        let sum = 0;
        for (let i = 0; i < tdata.length; i++) {
          const d = (tdata[i] - 128) / 128;
          sum += d * d;
        }
        const db =
          20 * Math.log10(Math.max(Math.sqrt(sum / tdata.length), 1e-6));
        const thr = -72 + 0.72 * sensitivity;
        return { above: db > thr, aboveOff: db > thr - 6 };
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
        const lv = speakLevels(maxSample);
        const wantSpeaking = lastSpeaking
          ? lv.aboveOff || now - lastSpeakingChangeAt < SPEAK_RELEASE_MS
          : lv.above;
        if (wantSpeaking !== lastSpeaking) {
          lastSpeaking = wantSpeaking;
          lastSpeakingChangeAt = now;
          if (!service.micGateActive()) service.setLocalSpeaking(wantSpeaking);
        }
        raf = requestAnimationFrame(tick);
      };
      raf = requestAnimationFrame(tick);
    };

    start();

    return () => {
      cancelled = true;
      teardown();
      if (!service.micGateActive()) service.setLocalSpeaking(false);
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
        className={
          "vp-meta-wrap" +
          (onOpen ? " clickable" : "") +
          (active ? " active" : "")
        }
        onClick={onOpen}
        onKeyDown={(e) => {
          if (!onOpen) return;
          if (e.key !== "Enter" && e.key !== " ") return;
          e.preventDefault();
          onOpen();
        }}
        tabIndex={onOpen ? 0 : undefined}
        title={onOpen ? openTitle : undefined}
        role={onOpen ? "button" : undefined}
      >
        <div className="vp-title">
          {title}
          {badge && <span className="vp-badge" />}
        </div>
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
          className={
            "vp-btn" + (isMuted ? " muted" : "") + (pttLive ? " ptt" : "")
          }
          onClick={onToggleMute}
          disabled={muteLocked}
          title={
            pttLive
              ? t("pttHeld")
              : muteLocked
                ? t("mutedByModerator")
                : isMuted
                  ? t("unmuteSelf")
                  : t("muteSelf")
          }
        >
          <Icon name={isMuted && !pttLive ? "mic-off" : "mic"} size={15} />
        </button>
        {onToggleDeafen && (
          <button
            type="button"
            className={"vp-btn" + (isDeafened ? " muted" : "")}
            onClick={onToggleDeafen}
            disabled={deafenLocked}
            title={
              deafenLocked
                ? t("deafenedByModerator")
                : isDeafened
                  ? t("undeafenSelf")
                  : t("deafenSelf")
            }
          >
            <Icon name={isDeafened ? "headphones-off" : "headphones"} size={15} />
          </button>
        )}
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
                className={noise === "high" && paid ? "on" : ""}
                disabled={!paid}
                title={paid ? undefined : tSubscription("needs.pro")}
                onClick={() => handleNoiseChange("high")}
              >
                <Icon name="sparkles" size={11} />
                {t("noiseHigh")}
              </button>
            </div>
          </div>

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
