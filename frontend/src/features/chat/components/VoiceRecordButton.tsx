import React from "react";
import { createPortal } from "react-dom";

import i18n from "../../../i18n";
import Icon from "../../../shared/ui/Icon";
import { maxVoiceMessageMs, videoNoteSize } from "../api/attachment";
import { formatVoiceDuration } from "../lib/voice";
import { VoiceRecorder, voiceRecordingSupported } from "../lib/voiceRecorder";
import { VideoNoteRecorder, videoNoteRecordingSupported } from "../lib/videoNoteRecorder";
import { readRecordKind, saveRecordKind } from "../lib/recording";
import type { MediaRecording, RecordKind } from "../lib/recording";
import { VIDEO_NOTE_MAX_MS } from "../../subscription/lib/plans";
import { useVoiceSettingsStore } from "../../voice/store/voiceSettingsStore";
import { useVoiceDraftStore } from "../store/voiceDraftStore";

type Mode = "idle" | "starting" | "holding" | "locked" | "failed" | "retrying";
type AnyRecorder = VoiceRecorder | VideoNoteRecorder;

const CANCEL_SWIPE_PX = 90;
const LOCK_SWIPE_PX = 70;
const MIN_HOLD_MS = 700;
const HOLD_START_MS = 250;
const LIVE_BARS = 48;
const TICK_MS = 100;
const LIMIT_MARGIN_MS = 500;
const RING_RADIUS = 47;
const RING_LENGTH = 2 * Math.PI * RING_RADIUS;

interface VoiceRecordButtonProps {
  disabled: boolean;
  onSend: (recording: MediaRecording) => Promise<boolean>;
  onError: (message: string | null) => void;
  onActiveChange?: (active: boolean) => void;
  draftKey: string;
}

const limitFor = (kind: RecordKind) => (kind === "video" ? VIDEO_NOTE_MAX_MS : maxVoiceMessageMs());

const errorMessage = (err: unknown, kind: RecordKind): string => {
  const name = (err as { name?: string } | null)?.name;
  if ((err as Error | null)?.message === "unsupported") {
    return i18n.t(kind === "video" ? "videoNoteUnsupported" : "voiceUnsupported", { ns: "chat" });
  }
  if (kind === "video" && (name === "NotReadableError" || name === "AbortError")) {
    return i18n.t("videoNoteCameraBusy", { ns: "chat" });
  }
  return i18n.t(kind === "video" ? "videoNoteNoCamera" : "voiceNoMic", { ns: "chat" });
};

const VoiceRecordButton: React.FC<VoiceRecordButtonProps> = ({
  disabled,
  onSend,
  onError,
  onActiveChange,
  draftKey,
}) => {
  const failed = useVoiceDraftStore((s) => s.failed[draftKey] ?? null);
  const [mode, setMode] = React.useState<Mode>(() =>
    useVoiceDraftStore.getState().failed[draftKey] ? "failed" : "idle",
  );
  const [kind, setKind] = React.useState<RecordKind>(readRecordKind);
  const [recordingKind, setRecordingKind] = React.useState<RecordKind>(kind);
  const [elapsed, setElapsed] = React.useState(0);
  const [levels, setLevels] = React.useState<number[]>([]);
  const [paused, setPaused] = React.useState(false);
  const [previewing, setPreviewing] = React.useState(false);
  const [previewUrl, setPreviewUrl] = React.useState<string | null>(null);
  const [drag, setDrag] = React.useState({ x: 0, y: 0 });
  const [canSwitch, setCanSwitch] = React.useState(false);
  const [mirrored, setMirrored] = React.useState(true);
  const recorderRef = React.useRef<AnyRecorder | null>(null);
  const kindRef = React.useRef<RecordKind>(kind);
  const modeRef = React.useRef<Mode>(mode);
  const originRef = React.useRef({ x: 0, y: 0 });
  const releasedRef = React.useRef(false);
  const lockRequestedRef = React.useRef(false);
  const finishingRef = React.useRef(false);
  const pressTimerRef = React.useRef<ReturnType<typeof setTimeout> | null>(null);
  const previewRef = React.useRef<{ media: HTMLMediaElement | null; url: string } | null>(null);
  const liveRef = React.useRef<HTMLDivElement>(null);
  const previewVideoRef = React.useRef<HTMLVideoElement>(null);

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
    preview.media?.pause();
    URL.revokeObjectURL(preview.url);
    previewRef.current = null;
    setPreviewing(false);
    setPreviewUrl(null);
  }, []);

  const reset = React.useCallback(() => {
    stopPreview();
    recorderRef.current = null;
    releasedRef.current = false;
    lockRequestedRef.current = false;
    finishingRef.current = false;
    setPaused(false);
    setElapsed(0);
    setLevels([]);
    setDrag({ x: 0, y: 0 });
    setCanSwitch(false);
    useVoiceDraftStore.getState().drop(draftKey);
    setModeBoth("idle");
  }, [draftKey, stopPreview]);

  const cancel = React.useCallback(() => {
    recorderRef.current?.cancel();
    reset();
  }, [reset]);

  const finish = React.useCallback(async () => {
    const recorder = recorderRef.current;
    if (!recorder || finishingRef.current) return;
    finishingRef.current = true;
    stopPreview();
    const stopped = await recorder.stop();
    const recording: MediaRecording = {
      ...stopped,
      durationMs: Math.min(stopped.durationMs, limitFor(stopped.kind)),
    };
    reset();
    if (recording.durationMs < MIN_HOLD_MS || recording.blob.size === 0) {
      onError(i18n.t("voiceTooShort", { ns: "chat" }));
      return;
    }
    if (await onSend(recording)) return;
    useVoiceDraftStore.getState().keep(draftKey, recording);
    setModeBoth("failed");
  }, [draftKey, onError, onSend, reset, stopPreview]);

  const retry = async () => {
    if (!failed || modeRef.current !== "failed") return;
    setModeBoth("retrying");
    if (await onSend(failed)) {
      reset();
      return;
    }
    setModeBoth("failed");
  };

  const onActiveChangeRef = React.useRef(onActiveChange);
  onActiveChangeRef.current = onActiveChange;
  React.useEffect(
    () => () => {
      if (pressTimerRef.current) clearTimeout(pressTimerRef.current);
      recorderRef.current?.cancel();
      recorderRef.current = null;
      const preview = previewRef.current;
      if (preview) {
        preview.media?.pause();
        URL.revokeObjectURL(preview.url);
        previewRef.current = null;
      }
      onActiveChangeRef.current?.(false);
    },
    [],
  );

  React.useEffect(() => {
    if (mode !== "holding" && mode !== "locked") return;
    const timer = setInterval(() => {
      const recorder = recorderRef.current;
      if (!recorder) return;
      const ms = recorder.elapsedMs();
      setElapsed(ms);
      if (ms >= limitFor(recordingKind) - LIMIT_MARGIN_MS) {
        onError(i18n.t("voiceLimitReached", { ns: "chat" }));
        void finish();
      }
    }, TICK_MS);
    return () => clearInterval(timer);
  }, [mode, finish, onError, recordingKind]);

  React.useEffect(() => {
    if (mode !== "holding" && mode !== "locked") return;
    const onHidden = () => {
      if (document.visibilityState !== "hidden") return;
      const recorder = recorderRef.current;
      if (!recorder || recorder.isPaused()) return;
      recorder.pause();
      setPaused(true);
      setElapsed(recorder.elapsedMs());
      if (modeRef.current === "holding") setModeBoth("locked");
    };
    onHidden();
    document.addEventListener("visibilitychange", onHidden);
    return () => document.removeEventListener("visibilitychange", onHidden);
  }, [mode]);

  React.useEffect(() => {
    if (mode === "idle") return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key !== "Escape") return;
      e.preventDefault();
      e.stopPropagation();
      if (modeRef.current === "retrying") return;
      cancel();
    };
    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
  }, [mode, cancel]);

  const videoRecording =
    recordingKind === "video" && (mode === "starting" || mode === "holding" || mode === "locked");

  React.useEffect(() => {
    const host = liveRef.current;
    const recorder = recorderRef.current;
    if (!videoRecording || previewing || !host || !(recorder instanceof VideoNoteRecorder)) return;
    host.appendChild(recorder.canvas);
    return () => {
      if (recorder.canvas.parentElement === host) host.removeChild(recorder.canvas);
    };
  }, [videoRecording, previewing, mode]);

  React.useEffect(() => {
    const video = previewVideoRef.current;
    if (!video || !previewUrl) return;
    if (previewRef.current) previewRef.current.media = video;
    void video.play().catch(stopPreview);
  }, [previewUrl, stopPreview]);

  const begin = async (locked: boolean) => {
    const current = kindRef.current;
    const supported = current === "video" ? videoNoteRecordingSupported() : voiceRecordingSupported();
    if (!supported) {
      onError(i18n.t(current === "video" ? "videoNoteUnsupported" : "voiceUnsupported", { ns: "chat" }));
      return;
    }
    onError(null);
    const recorder: AnyRecorder =
      current === "video" ? new VideoNoteRecorder(videoNoteSize()) : new VoiceRecorder();
    recorderRef.current = recorder;
    releasedRef.current = false;
    lockRequestedRef.current = false;
    setRecordingKind(current);
    setModeBoth("starting");
    if (recorder instanceof VoiceRecorder) {
      recorder.onLevel = (level) =>
        setLevels((prev) => {
          const next = prev.length >= LIVE_BARS ? prev.slice(prev.length - LIVE_BARS + 1) : prev.slice();
          next.push(level);
          return next;
        });
    }
    try {
      await recorder.start(useVoiceSettingsStore.getState().micDeviceId || undefined);
    } catch (err) {
      if (recorderRef.current === recorder) reset();
      onError(errorMessage(err, current));
      return;
    }
    if (recorderRef.current !== recorder) {
      recorder.cancel();
      return;
    }
    if (recorder instanceof VideoNoteRecorder) {
      setCanSwitch(recorder.canSwitchCamera());
      setMirrored(recorder.facingUser);
    }
    if (!locked && releasedRef.current && !lockRequestedRef.current) {
      cancel();
      return;
    }
    setModeBoth(locked || lockRequestedRef.current ? "locked" : "holding");
    if (lockRequestedRef.current) setDrag({ x: 0, y: 0 });
  };

  const toggleKind = () => {
    const next: RecordKind = kindRef.current === "voice" ? "video" : "voice";
    kindRef.current = next;
    setKind(next);
    saveRecordKind(next);
  };

  const clearPress = () => {
    if (!pressTimerRef.current) return false;
    clearTimeout(pressTimerRef.current);
    pressTimerRef.current = null;
    return true;
  };

  const onPointerDown = (e: React.PointerEvent<HTMLButtonElement>) => {
    if (disabled || modeRef.current !== "idle" || pressTimerRef.current) return;
    e.preventDefault();
    e.currentTarget.setPointerCapture(e.pointerId);
    originRef.current = { x: e.clientX, y: e.clientY };
    pressTimerRef.current = setTimeout(() => {
      pressTimerRef.current = null;
      void begin(false);
    }, HOLD_START_MS);
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
    if (dy < -LOCK_SWIPE_PX) {
      if (current === "starting") {
        lockRequestedRef.current = true;
        return;
      }
      setDrag({ x: 0, y: 0 });
      setModeBoth("locked");
    }
  };

  const onPointerUp = () => {
    if (clearPress()) {
      toggleKind();
      return;
    }
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
    if (recorderRef.current !== recorder || !recorder.isPaused() || previewRef.current) return;
    const url = URL.createObjectURL(blob);
    if (recorder instanceof VideoNoteRecorder) {
      previewRef.current = { media: null, url };
      setPreviewUrl(url);
      setPreviewing(true);
      return;
    }
    const audio = new Audio(url);
    previewRef.current = { media: audio, url };
    audio.addEventListener("ended", stopPreview);
    setPreviewing(true);
    void audio.play().catch(stopPreview);
  };

  const switchCamera = async () => {
    const recorder = recorderRef.current;
    if (!(recorder instanceof VideoNoteRecorder)) return;
    try {
      await recorder.switchCamera();
      setMirrored(recorder.facingUser);
    } catch (err) {
      onError(errorMessage(err, "video"));
    }
  };

  const active = mode !== "idle";
  const locked = mode === "locked";

  if ((mode === "failed" || mode === "retrying") && failed) {
    return (
      <>
        <div className="voice-rec-bar locked failed">
          <button
            type="button"
            className="voice-rec-icon danger"
            title={i18n.t("voiceDiscard", { ns: "chat" })}
            aria-label={i18n.t("voiceDiscard", { ns: "chat" })}
            disabled={mode === "retrying"}
            onClick={cancel}
          >
            <Icon name="trash" size={16} />
          </button>
          <Icon name={failed.kind === "video" ? "camera" : "mic"} size={14} />
          <span className="voice-rec-time">{formatVoiceDuration(failed.durationMs)}</span>
          <span className="voice-rec-hint">{i18n.t("voiceNotSent", { ns: "chat" })}</span>
        </div>
        <button
          type="button"
          className="send-btn enabled voice-rec-send"
          title={i18n.t("voiceRetry", { ns: "chat" })}
          aria-label={i18n.t("voiceRetry", { ns: "chat" })}
          disabled={mode === "retrying" || disabled}
          onClick={() => void retry()}
        >
          <Icon name="refresh" size={14} />
        </button>
      </>
    );
  }

  const progress = Math.min(1, elapsed / VIDEO_NOTE_MAX_MS);

  return (
    <>
      {videoRecording &&
        createPortal(
          <div className="vnote-rec" role="dialog" aria-label={i18n.t("videoNoteRecording", { ns: "chat" })}>
            <div className="vnote-rec-circle">
              {previewUrl ? (
                <video
                  ref={previewVideoRef}
                  className="vnote-rec-preview"
                  src={previewUrl}
                  playsInline
                  onEnded={stopPreview}
                />
              ) : (
                <div ref={liveRef} className={"vnote-rec-live" + (mirrored ? " mirror" : "")} />
              )}
              <svg className="vnote-ring" viewBox="0 0 100 100" aria-hidden="true">
                <circle className="vnote-ring-track" cx="50" cy="50" r={RING_RADIUS} />
                <circle
                  className="vnote-ring-fill"
                  cx="50"
                  cy="50"
                  r={RING_RADIUS}
                  strokeDasharray={RING_LENGTH}
                  strokeDashoffset={RING_LENGTH * (1 - progress)}
                />
              </svg>
            </div>
            {canSwitch && !previewUrl && (
              <button
                type="button"
                className="vnote-rec-switch"
                title={i18n.t("videoNoteSwitchCamera", { ns: "chat" })}
                aria-label={i18n.t("videoNoteSwitchCamera", { ns: "chat" })}
                onClick={() => void switchCamera()}
              >
                <Icon name="refresh" size={16} />
              </button>
            )}
          </div>,
          document.body,
        )}
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
                <Icon name={paused ? (recordingKind === "video" ? "camera" : "mic") : "pause"} size={14} />
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
          className={
            "send-btn enabled voice-rec-btn" + (active ? " recording" : "") + (kind === "video" ? " video" : "")
          }
          title={i18n.t(kind === "video" ? "videoNoteRecordHint" : "voiceRecordHint", { ns: "chat" })}
          aria-label={i18n.t(kind === "video" ? "videoNoteRecord" : "voiceRecord", { ns: "chat" })}
          disabled={disabled && !active}
          onPointerDown={onPointerDown}
          onPointerMove={onPointerMove}
          onPointerUp={onPointerUp}
          onPointerCancel={() => {
            clearPress();
            if (modeRef.current === "holding" || modeRef.current === "starting") cancel();
          }}
          onClick={(e) => {
            if (e.detail !== 0 || disabled || modeRef.current !== "idle") return;
            void begin(true);
          }}
          onContextMenu={(e) => e.preventDefault()}
        >
          <Icon name={kind === "video" ? "camera" : "mic"} size={16} />
        </button>
      )}
    </>
  );
};

export default VoiceRecordButton;
