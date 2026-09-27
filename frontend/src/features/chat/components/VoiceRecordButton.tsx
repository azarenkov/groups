import React from "react";

import i18n from "../../../i18n";
import Icon from "../../../shared/ui/Icon";
import { maxVoiceMessageMs } from "../api/attachment";
import { formatVoiceDuration } from "../lib/voice";
import { VoiceRecorder, voiceRecordingSupported } from "../lib/voiceRecorder";
import type { VoiceRecording } from "../lib/voiceRecorder";
import { useVoiceSettingsStore } from "../../voice/store/voiceSettingsStore";

type Mode = "idle" | "starting" | "holding" | "locked";

const CANCEL_SWIPE_PX = 90;
const LOCK_SWIPE_PX = 70;
const MIN_HOLD_MS = 700;
const LIVE_BARS = 48;
const TICK_MS = 100;

interface VoiceRecordButtonProps {
  disabled: boolean;
  onSend: (recording: VoiceRecording) => Promise<boolean>;
  onError: (message: string | null) => void;
  onActiveChange?: (active: boolean) => void;
}

const errorMessage = (err: unknown): string => {
  const name = (err as { name?: string } | null)?.name;
  if (name === "NotAllowedError" || name === "SecurityError" || name === "NotFoundError") {
    return i18n.t("voiceNoMic", { ns: "chat" });
  }
  if ((err as Error | null)?.message === "unsupported") {
    return i18n.t("voiceUnsupported", { ns: "chat" });
  }
  return i18n.t("voiceNoMic", { ns: "chat" });
};

const VoiceRecordButton: React.FC<VoiceRecordButtonProps> = ({
  disabled,
  onSend,
  onError,
  onActiveChange,
}) => {
  const [mode, setMode] = React.useState<Mode>("idle");
  const [elapsed, setElapsed] = React.useState(0);
  const [levels, setLevels] = React.useState<number[]>([]);
  const [paused, setPaused] = React.useState(false);
  const [previewing, setPreviewing] = React.useState(false);
  const [drag, setDrag] = React.useState({ x: 0, y: 0 });
  const recorderRef = React.useRef<VoiceRecorder | null>(null);
  const modeRef = React.useRef<Mode>("idle");
  const originRef = React.useRef({ x: 0, y: 0 });
  const releasedRef = React.useRef(false);
  const finishingRef = React.useRef(false);
  const previewRef = React.useRef<{ audio: HTMLAudioElement; url: string } | null>(null);

  const setModeBoth = (next: Mode) => {
    modeRef.current = next;
    setMode(next);
  };

  React.useEffect(() => {
    onActiveChange?.(mode !== "idle");
  }, [mode, onActiveChange]);

  const stopPreview = React.useCallback(() => {
    const preview = previewRef.current;
    if (!preview) return;
    preview.audio.pause();
    URL.revokeObjectURL(preview.url);
    previewRef.current = null;
    setPreviewing(false);
  }, []);

  const reset = React.useCallback(() => {
    stopPreview();
    recorderRef.current = null;
    releasedRef.current = false;
    finishingRef.current = false;
    setPaused(false);
    setElapsed(0);
    setLevels([]);
    setDrag({ x: 0, y: 0 });
    setModeBoth("idle");
  }, [stopPreview]);

  const cancel = React.useCallback(() => {
    recorderRef.current?.cancel();
    reset();
  }, [reset]);

  const finish = React.useCallback(async () => {
    const recorder = recorderRef.current;
    if (!recorder || finishingRef.current) return;
    finishingRef.current = true;
    stopPreview();
    const recording = await recorder.stop();
    reset();
    if (recording.durationMs < MIN_HOLD_MS || recording.blob.size === 0) {
      onError(i18n.t("voiceTooShort", { ns: "chat" }));
      return;
    }
    await onSend(recording);
  }, [onError, onSend, reset, stopPreview]);

  React.useEffect(() => () => recorderRef.current?.cancel(), []);

  React.useEffect(() => {
    if (mode !== "holding" && mode !== "locked") return;
    const timer = setInterval(() => {
      const recorder = recorderRef.current;
      if (!recorder) return;
      const ms = recorder.elapsedMs();
      setElapsed(ms);
      if (ms >= maxVoiceMessageMs()) {
        onError(i18n.t("voiceLimitReached", { ns: "chat" }));
        void finish();
      }
    }, TICK_MS);
    return () => clearInterval(timer);
  }, [mode, finish, onError]);

  React.useEffect(() => {
    if (mode === "idle") return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key !== "Escape") return;
      e.preventDefault();
      e.stopPropagation();
      cancel();
    };
    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
  }, [mode, cancel]);

  const begin = async (locked: boolean) => {
    if (!voiceRecordingSupported()) {
      onError(i18n.t("voiceUnsupported", { ns: "chat" }));
      return;
    }
    onError(null);
    const recorder = new VoiceRecorder();
    recorderRef.current = recorder;
    releasedRef.current = false;
    setModeBoth("starting");
    recorder.onLevel = (level) =>
      setLevels((prev) => {
        const next = prev.length >= LIVE_BARS ? prev.slice(prev.length - LIVE_BARS + 1) : prev.slice();
        next.push(level);
        return next;
      });
    try {
      await recorder.start(useVoiceSettingsStore.getState().micDeviceId || undefined);
    } catch (err) {
      if (recorderRef.current === recorder) reset();
      onError(errorMessage(err));
      return;
    }
    if (recorderRef.current !== recorder) {
      recorder.cancel();
      return;
    }
    if (!locked && releasedRef.current) {
      cancel();
      return;
    }
    setModeBoth(locked ? "locked" : "holding");
  };

  const onPointerDown = (e: React.PointerEvent<HTMLButtonElement>) => {
    if (disabled || modeRef.current !== "idle") return;
    e.preventDefault();
    if (e.pointerType === "mouse") {
      void begin(true);
      return;
    }
    e.currentTarget.setPointerCapture(e.pointerId);
    originRef.current = { x: e.clientX, y: e.clientY };
    void begin(false);
  };

  const onPointerMove = (e: React.PointerEvent<HTMLButtonElement>) => {
    const current = modeRef.current;
    if (current !== "holding" && current !== "starting") return;
    const dx = Math.min(0, e.clientX - originRef.current.x);
    const dy = Math.min(0, e.clientY - originRef.current.y);
    setDrag({ x: dx, y: dy });
    if (dx < -CANCEL_SWIPE_PX) {
      cancel();
      return;
    }
    if (dy < -LOCK_SWIPE_PX && current === "holding") {
      setDrag({ x: 0, y: 0 });
      setModeBoth("locked");
    }
  };

  const onPointerUp = () => {
    const current = modeRef.current;
    if (current === "starting") {
      releasedRef.current = true;
      return;
    }
    if (current === "holding") void finish();
  };

  const togglePause = () => {
    const recorder = recorderRef.current;
    if (!recorder) return;
    if (recorder.isPaused()) {
      stopPreview();
      recorder.resume();
      setPaused(false);
    } else {
      recorder.pause();
      setPaused(true);
      setElapsed(recorder.elapsedMs());
    }
  };

  const togglePreview = async () => {
    if (previewRef.current) {
      stopPreview();
      return;
    }
    const recorder = recorderRef.current;
    if (!recorder) return;
    const blob = await recorder.snapshot();
    const url = URL.createObjectURL(blob);
    const audio = new Audio(url);
    previewRef.current = { audio, url };
    audio.addEventListener("ended", stopPreview);
    setPreviewing(true);
    void audio.play().catch(stopPreview);
  };

  const active = mode !== "idle";
  const locked = mode === "locked";

  return (
    <>
      {active && (
        <div className={"voice-rec-bar" + (locked ? " locked" : "")}>
          {locked && (
            <button
              type="button"
              className="voice-rec-icon danger"
              title={i18n.t("voiceCancel", { ns: "chat" })}
              aria-label={i18n.t("voiceCancel", { ns: "chat" })}
              onClick={cancel}
            >
              <Icon name="trash" size={16} />
            </button>
          )}
          <span className={"voice-rec-dot" + (paused ? " paused" : "")} />
          <span className="voice-rec-time">{formatVoiceDuration(elapsed)}</span>
          <div className="voice-rec-wave" aria-hidden="true">
            {levels.map((l, i) => (
              <span key={i} style={{ height: `${Math.max(10, Math.round(l * 100))}%` }} />
            ))}
          </div>
          {locked ? (
            <>
              {paused && (
                <button
                  type="button"
                  className="voice-rec-icon"
                  title={previewing ? i18n.t("voiceStop", { ns: "chat" }) : i18n.t("voicePlay", { ns: "chat" })}
                  aria-label={previewing ? i18n.t("voiceStop", { ns: "chat" }) : i18n.t("voicePlay", { ns: "chat" })}
                  onClick={() => void togglePreview()}
                >
                  <Icon name={previewing ? "pause" : "play"} size={14} />
                </button>
              )}
              <button
                type="button"
                className="voice-rec-icon"
                title={paused ? i18n.t("voiceResume", { ns: "chat" }) : i18n.t("voicePause", { ns: "chat" })}
                aria-label={paused ? i18n.t("voiceResume", { ns: "chat" }) : i18n.t("voicePause", { ns: "chat" })}
                onClick={togglePause}
              >
                <Icon name={paused ? "mic" : "pause"} size={14} />
              </button>
            </>
          ) : (
            <span className="voice-rec-hint" style={{ transform: `translateX(${drag.x}px)` }}>
              <Icon name="chevron" size={12} />
              {i18n.t("voiceSlideToCancel", { ns: "chat" })}
            </span>
          )}
        </div>
      )}
      {mode === "holding" && (
        <span className="voice-rec-lock" style={{ transform: `translateY(${Math.max(drag.y, -LOCK_SWIPE_PX)}px)` }}>
          <Icon name="lock" size={13} />
        </span>
      )}
      {locked ? (
        <button
          type="button"
          className="send-btn enabled voice-rec-send"
          title={i18n.t("voiceSend", { ns: "chat" })}
          aria-label={i18n.t("voiceSend", { ns: "chat" })}
          onClick={() => void finish()}
        >
          <Icon name="send" size={14} />
        </button>
      ) : (
        <button
          type="button"
          className={"send-btn enabled voice-rec-btn" + (active ? " recording" : "")}
          title={i18n.t("voiceRecordHint", { ns: "chat" })}
          aria-label={i18n.t("voiceRecord", { ns: "chat" })}
          disabled={disabled && !active}
          onPointerDown={onPointerDown}
          onPointerMove={onPointerMove}
          onPointerUp={onPointerUp}
          onPointerCancel={() => {
            if (modeRef.current === "holding" || modeRef.current === "starting") cancel();
          }}
          onContextMenu={(e) => e.preventDefault()}
        >
          <Icon name="mic" size={16} />
        </button>
      )}
    </>
  );
};

export default VoiceRecordButton;
