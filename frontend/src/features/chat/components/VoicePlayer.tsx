import React from "react";

import i18n from "../../../i18n";
import Icon from "../../../shared/ui/Icon";
import type { Attachment } from "../../../shared/types/channel";
import { attachmentAPI } from "../api/attachment";
import { formatVoiceDuration } from "../lib/voice";
import { useVoiceSettingsStore } from "../../voice/store/voiceSettingsStore";
import {
  nextAutoplayAfter,
  registerVoicePlayer,
  useVoicePlaybackStore,
} from "../store/voicePlaybackStore";
import LoadingDots from "../../../shared/ui/LoadingDots";

type SinkAudio = HTMLAudioElement & { setSinkId?: (id: string) => Promise<void> };

interface VoicePlayerProps {
  messageId: string;
  playbackId?: string;
  autoplay?: boolean;
  attachment: Attachment;
  isMine: boolean;
  listened: boolean;
  showListenDot: boolean;
  onListened?: () => Promise<void>;
}

const VoicePlayer: React.FC<VoicePlayerProps> = ({
  messageId,
  playbackId,
  autoplay = true,
  attachment,
  isMine,
  listened,
  showListenDot,
  onListened,
}) => {
  const playId = playbackId ?? messageId;
  const durationMs = attachment.voice?.durationMs ?? 0;
  const waveform = attachment.voice?.waveform ?? [];
  const rootRef = React.useRef<HTMLDivElement>(null);
  const audioRef = React.useRef<SinkAudio | null>(null);
  const urlRef = React.useRef<string | null>(null);
  const loadingRef = React.useRef<Promise<SinkAudio> | null>(null);
  const pendingSeekRef = React.useRef<number | null>(null);
  const reportedRef = React.useRef(listened);
  const disposedRef = React.useRef(false);
  const [playing, setPlaying] = React.useState(false);
  const [loading, setLoading] = React.useState(false);
  const [failed, setFailed] = React.useState(false);
  const [progress, setProgress] = React.useState(0);
  const [dragging, setDragging] = React.useState(false);
  const currentId = useVoicePlaybackStore((s) => s.currentId);
  const speed = useVoicePlaybackStore((s) => s.speed);
  const speakerDeviceId = useVoiceSettingsStore((s) => s.speakerDeviceId);

  React.useEffect(() => {
    if (listened) reportedRef.current = true;
  }, [listened]);

  const tick = React.useCallback(() => {
    const audio = audioRef.current;
    if (!audio || durationMs <= 0) return;
    setProgress(Math.min(1, (audio.currentTime * 1000) / durationMs));
  }, [durationMs]);

  React.useEffect(() => {
    if (!playing) return;
    let frame = 0;
    const loop = () => {
      tick();
      frame = requestAnimationFrame(loop);
    };
    frame = requestAnimationFrame(loop);
    return () => cancelAnimationFrame(frame);
  }, [playing, tick]);

  React.useEffect(() => {
    if (audioRef.current) audioRef.current.playbackRate = speed;
  }, [speed]);

  React.useEffect(() => {
    const audio = audioRef.current;
    if (audio?.setSinkId) void audio.setSinkId(speakerDeviceId || "").catch(() => {});
  }, [speakerDeviceId]);

  const unloadAudio = React.useCallback(() => {
    const audio = audioRef.current;
    if (!audio) return;
    const at = durationMs > 0 ? (audio.currentTime * 1000) / durationMs : 0;
    pendingSeekRef.current = at > 0 && at < 1 ? at : null;
    audioRef.current = null;
    loadingRef.current = null;
    audio.pause();
    audio.removeAttribute("src");
    audio.load();
    if (urlRef.current) URL.revokeObjectURL(urlRef.current);
    urlRef.current = null;
    setPlaying(false);
  }, [durationMs]);

  React.useEffect(() => {
    if (currentId !== playId) unloadAudio();
  }, [currentId, playId, unloadAudio]);

  React.useEffect(() => {
    disposedRef.current = false;
    return () => {
      disposedRef.current = true;
      audioRef.current?.pause();
      if (urlRef.current) URL.revokeObjectURL(urlRef.current);
      useVoicePlaybackStore.getState().release(playId);
    };
  }, [playId]);

  const ensureAudio = React.useCallback((): Promise<SinkAudio> => {
    if (audioRef.current) return Promise.resolve(audioRef.current);
    if (loadingRef.current) return loadingRef.current;
    setLoading(true);
    const promise = attachmentAPI
      .download(attachment.id, attachment.sizeBytes)
      .then(async (blob) => {
        if (disposedRef.current) throw new Error("disposed");
        const url = URL.createObjectURL(blob);
        urlRef.current = url;
        const audio: SinkAudio = new Audio(url);
        audio.preload = "auto";
        audio.playbackRate = useVoicePlaybackStore.getState().speed;
        const sink = useVoiceSettingsStore.getState().speakerDeviceId;
        if (sink && audio.setSinkId) await audio.setSinkId(sink).catch(() => {});
        if (disposedRef.current) {
          URL.revokeObjectURL(url);
          urlRef.current = null;
          throw new Error("disposed");
        }
        audio.addEventListener("play", () => setPlaying(true));
        audio.addEventListener("pause", () => {
          setPlaying(false);
          tick();
        });
        audio.addEventListener("ended", () => {
          setPlaying(false);
          setProgress(0);
          audio.currentTime = 0;
          useVoicePlaybackStore.getState().release(playId);
          nextAutoplayAfter(playId)?.play();
        });
        audioRef.current = audio;
        setLoading(false);
        return audio;
      })
      .catch((err) => {
        loadingRef.current = null;
        setLoading(false);
        setFailed(true);
        throw err;
      });
    loadingRef.current = promise;
    return promise;
  }, [attachment.id, attachment.sizeBytes, playId, tick]);

  const play = React.useCallback(() => {
    useVoicePlaybackStore.getState().setCurrent(playId);
    setFailed(false);
    void ensureAudio()
      .then((audio) => {
        if (useVoicePlaybackStore.getState().currentId !== playId) {
          unloadAudio();
          return;
        }
        if (pendingSeekRef.current !== null) {
          audio.currentTime = (pendingSeekRef.current * durationMs) / 1000;
          pendingSeekRef.current = null;
        }
        audio.playbackRate = useVoicePlaybackStore.getState().speed;
        return audio.play().then(() => {
          if (!isMine && !reportedRef.current && onListened) {
            reportedRef.current = true;
            void onListened().catch(() => {
              reportedRef.current = false;
            });
          }
        });
      })
      .catch(() => setFailed(true));
  }, [durationMs, ensureAudio, isMine, playId, onListened, unloadAudio]);

  const autoplayableRef = React.useRef(false);
  autoplayableRef.current = autoplay && !isMine && !listened && !reportedRef.current;
  React.useEffect(
    () =>
      registerVoicePlayer(playId, {
        element: () => rootRef.current,
        autoplayable: () => autoplayableRef.current,
        play,
      }),
    [playId, play],
  );

  const toggle = () => {
    if (playing) {
      audioRef.current?.pause();
      return;
    }
    play();
  };

  const seekTo = (fraction: number) => {
    const clamped = Math.min(1, Math.max(0, fraction));
    setProgress(clamped);
    const audio = audioRef.current;
    if (audio) audio.currentTime = (clamped * durationMs) / 1000;
    else pendingSeekRef.current = clamped;
  };

  const fractionAt = (e: React.PointerEvent<HTMLDivElement>) => {
    const rect = e.currentTarget.getBoundingClientRect();
    return rect.width > 0 ? (e.clientX - rect.left) / rect.width : 0;
  };

  const played = Math.round(progress * waveform.length);
  const shownMs = playing || progress > 0 ? progress * durationMs : durationMs;

  return (
    <div
      ref={rootRef}
      className={"voice-msg" + (isMine ? " mine" : "") + (playing ? " playing" : "")}
    >
      <button
        type="button"
        className="voice-msg-play"
        onClick={toggle}
        aria-label={playing ? i18n.t("voicePause", { ns: "chat" }) : i18n.t("voicePlay", { ns: "chat" })}
        title={failed ? i18n.t("voiceInvalid", { ns: "chat" }) : undefined}
      >
        {loading ? (
          <LoadingDots className="voice-msg-spinner" />
        ) : (
          <Icon name={playing ? "pause" : "play"} size={14} />
        )}
      </button>
      <div className="voice-msg-body">
        <div
          className={"voice-msg-wave" + (dragging ? " dragging" : "")}
          role="slider"
          tabIndex={0}
          aria-valuemin={0}
          aria-valuemax={Math.round(durationMs / 1000)}
          aria-valuenow={Math.round((progress * durationMs) / 1000)}
          onPointerDown={(e) => {
            e.preventDefault();
            e.currentTarget.setPointerCapture(e.pointerId);
            setDragging(true);
            seekTo(fractionAt(e));
          }}
          onPointerMove={(e) => {
            if (dragging) seekTo(fractionAt(e));
          }}
          onPointerUp={(e) => {
            if (!dragging) return;
            setDragging(false);
            seekTo(fractionAt(e));
          }}
          onPointerCancel={() => setDragging(false)}
          onKeyDown={(e) => {
            if (e.key === "ArrowRight") seekTo(progress + 0.05);
            else if (e.key === "ArrowLeft") seekTo(progress - 0.05);
            else if (e.key === " " || e.key === "Enter") {
              e.preventDefault();
              toggle();
            }
          }}
        >
          {waveform.map((v, i) => (
            <span
              key={i}
              className={"voice-msg-bar" + (i < played ? " played" : "")}
              style={{ height: `${Math.max(12, Math.round((v / 255) * 100))}%` }}
            />
          ))}
        </div>
        <div className="voice-msg-meta">
          <span className="voice-msg-time">{formatVoiceDuration(shownMs)}</span>
          {showListenDot && !listened && (
            <span
              className="voice-msg-dot"
              title={i18n.t("voiceUnlistened", { ns: "chat" })}
            />
          )}
          <button
            type="button"
            className="voice-msg-speed"
            title={i18n.t("voiceSpeed", { ns: "chat" })}
            onClick={() => useVoicePlaybackStore.getState().cycleSpeed()}
          >
            {speed}×
          </button>
        </div>
      </div>
    </div>
  );
};

export default VoicePlayer;
