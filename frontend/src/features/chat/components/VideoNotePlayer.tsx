import React from "react";

import i18n from "../../../i18n";
import type { Attachment } from "../../../shared/types/channel";
import { attachmentAPI } from "../api/attachment";
import { formatVoiceDuration } from "../lib/voice";
import { useVoiceSettingsStore } from "../../voice/store/voiceSettingsStore";
import {
  nextAutoplayAfter,
  registerVoicePlayer,
  useVoicePlaybackStore,
} from "../store/voicePlaybackStore";

type SinkVideo = HTMLVideoElement & { setSinkId?: (id: string) => Promise<void> };

const RING_RADIUS = 48.5;
const PREVIEW_MAX_BYTES = 6 * 1024 * 1024;
const PREVIEW_DWELL_MS = 800;
const RING_LENGTH = 2 * Math.PI * RING_RADIUS;

interface VideoNotePlayerProps {
  messageId: string;
  attachment: Attachment;
  isMine: boolean;
  listened: boolean;
  showListenDot: boolean;
  onListened?: () => Promise<void>;
}

const VideoNotePlayer: React.FC<VideoNotePlayerProps> = ({
  messageId,
  attachment,
  isMine,
  listened,
  showListenDot,
  onListened,
}) => {
  const note = attachment.videoNote;
  const durationMs = note?.durationMs ?? 0;
  const rootRef = React.useRef<HTMLDivElement>(null);
  const videoRef = React.useRef<SinkVideo>(null);
  const urlRef = React.useRef<string | null>(null);
  const loadingRef = React.useRef(false);
  const disposedRef = React.useRef(false);
  const reportedRef = React.useRef(listened);
  const [visible, setVisible] = React.useState(false);
  const [url, setUrl] = React.useState<string | null>(null);
  const [playing, setPlaying] = React.useState(false);
  const [progress, setProgress] = React.useState(0);
  const [failed, setFailed] = React.useState(false);
  const [dragging, setDragging] = React.useState(false);
  const currentId = useVoicePlaybackStore((s) => s.currentId);
  const speed = useVoicePlaybackStore((s) => s.speed);
  const speakerDeviceId = useVoiceSettingsStore((s) => s.speakerDeviceId);
  const active = currentId === messageId;
  const activeRef = React.useRef(active);
  activeRef.current = active;
  const visibleRef = React.useRef(visible);
  visibleRef.current = visible;

  React.useEffect(() => {
    if (listened) reportedRef.current = true;
  }, [listened]);

  React.useEffect(() => {
    const node = rootRef.current;
    if (!node) return;
    if (typeof IntersectionObserver === "undefined") {
      setVisible(true);
      return;
    }
    const observer = new IntersectionObserver(
      (entries) => setVisible(entries.some((e) => e.isIntersecting && e.intersectionRatio >= 0.4)),
      { threshold: [0, 0.4] },
    );
    observer.observe(node);
    return () => observer.disconnect();
  }, []);

  const unload = React.useCallback(() => {
    const video = videoRef.current;
    video?.pause();
    if (urlRef.current) URL.revokeObjectURL(urlRef.current);
    urlRef.current = null;
    setUrl(null);
    setPlaying(false);
  }, []);

  React.useEffect(() => {
    disposedRef.current = false;
    return () => {
      disposedRef.current = true;
      if (urlRef.current) URL.revokeObjectURL(urlRef.current);
      urlRef.current = null;
      useVoicePlaybackStore.getState().release(messageId);
    };
  }, [messageId]);

  const previewAllowed = attachment.sizeBytes <= PREVIEW_MAX_BYTES;

  React.useEffect(() => {
    const needed = active || (visible && previewAllowed);
    if (!needed) {
      if (!active) unload();
      return;
    }
    if (urlRef.current || loadingRef.current) return;
    const controller = new AbortController();
    let settled = false;
    const start = () => {
      loadingRef.current = true;
      setFailed(false);
      attachmentAPI
        .download(attachment.id, attachment.sizeBytes, controller.signal)
        .then((blob) => {
          settled = true;
          loadingRef.current = false;
          if (disposedRef.current || !(visibleRef.current || activeRef.current)) return;
          const next = URL.createObjectURL(blob);
          urlRef.current = next;
          setUrl(next);
        })
        .catch(() => {
          settled = true;
          loadingRef.current = false;
          if (controller.signal.aborted) return;
          setFailed(true);
          if (activeRef.current) useVoicePlaybackStore.getState().release(messageId);
        });
    };
    const timer = active ? null : setTimeout(start, PREVIEW_DWELL_MS);
    if (active) start();
    return () => {
      if (timer) clearTimeout(timer);
      if (!settled && loadingRef.current && !activeRef.current) {
        controller.abort();
        loadingRef.current = false;
      }
    };
  }, [visible, active, previewAllowed, attachment.id, attachment.sizeBytes, messageId, unload]);

  React.useEffect(() => {
    const video = videoRef.current;
    if (!video || !url) return;
    if (video.setSinkId) void video.setSinkId(speakerDeviceId || "").catch(() => {});
  }, [url, speakerDeviceId]);

  const onListenedRef = React.useRef(onListened);
  onListenedRef.current = onListened;

  React.useEffect(() => {
    const video = videoRef.current;
    if (!video || !url) return;
    if (active) {
      video.muted = false;
      video.loop = false;
      video.playbackRate = useVoicePlaybackStore.getState().speed;
      video.currentTime = 0;
      setProgress(0);
      void video
        .play()
        .then(() => {
          const report = onListenedRef.current;
          if (!isMine && !reportedRef.current && report) {
            reportedRef.current = true;
            void report().catch(() => {
              reportedRef.current = false;
            });
          }
        })
        .catch((err: unknown) => {
          if ((err as { name?: string } | null)?.name !== "NotAllowedError") setFailed(true);
        });
      return;
    }
    video.muted = true;
    video.loop = true;
    video.playbackRate = 1;
    setProgress(0);
  }, [active, url, isMine]);

  React.useEffect(() => {
    const video = videoRef.current;
    if (!video || !url || active) return;
    if (visible) void video.play().catch(() => {});
    else video.pause();
  }, [active, url, visible]);

  React.useEffect(() => {
    const video = videoRef.current;
    if (video && active) video.playbackRate = speed;
  }, [active, speed]);

  React.useEffect(() => {
    if (!active || !playing) return;
    let frame = 0;
    const loop = () => {
      const video = videoRef.current;
      if (video && durationMs > 0) setProgress(Math.min(1, (video.currentTime * 1000) / durationMs));
      frame = requestAnimationFrame(loop);
    };
    frame = requestAnimationFrame(loop);
    return () => cancelAnimationFrame(frame);
  }, [active, playing, durationMs]);

  const play = React.useCallback(() => {
    useVoicePlaybackStore.getState().setCurrent(messageId);
  }, [messageId]);

  const autoplayableRef = React.useRef(false);
  autoplayableRef.current = !isMine && !listened && !reportedRef.current;
  React.useEffect(
    () =>
      registerVoicePlayer(messageId, {
        element: () => rootRef.current,
        autoplayable: () => autoplayableRef.current,
        play,
      }),
    [messageId, play],
  );

  const toggle = () => {
    if (!active) {
      play();
      return;
    }
    const video = videoRef.current;
    if (!video) return;
    if (video.paused) void video.play().catch(() => setFailed(true));
    else video.pause();
  };

  const seekAt = (e: React.PointerEvent<SVGSVGElement>) => {
    const rect = e.currentTarget.getBoundingClientRect();
    const dx = e.clientX - (rect.left + rect.width / 2);
    const dy = e.clientY - (rect.top + rect.height / 2);
    const angle = Math.atan2(dx, -dy);
    const fraction = (angle < 0 ? angle + 2 * Math.PI : angle) / (2 * Math.PI);
    setProgress(fraction);
    const video = videoRef.current;
    if (video) video.currentTime = (fraction * durationMs) / 1000;
  };

  if (!note) return null;
  const poster = `data:image/jpeg;base64,${note.poster}`;
  const shownMs = active && progress > 0 ? progress * durationMs : durationMs;

  return (
    <div
      ref={rootRef}
      className={"vnote" + (active ? " active" : "") + (playing && active ? " playing" : "")}
    >
      <button
        type="button"
        className="vnote-face"
        onClick={toggle}
        aria-label={
          active && playing ? i18n.t("voicePause", { ns: "chat" }) : i18n.t("videoNotePlay", { ns: "chat" })
        }
        title={failed ? i18n.t("videoNoteInvalid", { ns: "chat" }) : undefined}
      >
        <img className="vnote-poster" src={poster} alt="" draggable={false} />
        {active && !playing && url && <span className="vnote-play" aria-hidden="true" />}
        {url && (
          <video
            ref={videoRef}
            className="vnote-video"
            src={url}
            muted
            loop
            playsInline
            poster={poster}
            onPlay={() => setPlaying(true)}
            onPause={() => setPlaying(false)}
            onEnded={() => {
              if (!activeRef.current) return;
              setPlaying(false);
              setProgress(0);
              useVoicePlaybackStore.getState().release(messageId);
              nextAutoplayAfter(messageId)?.play();
            }}
          />
        )}
      </button>
      {active && (
        <svg
          className={"vnote-progress" + (dragging ? " dragging" : "")}
          viewBox="0 0 100 100"
          onPointerDown={(e) => {
            e.preventDefault();
            e.currentTarget.setPointerCapture(e.pointerId);
            setDragging(true);
            seekAt(e);
          }}
          onPointerMove={(e) => {
            if (dragging) seekAt(e);
          }}
          onPointerUp={(e) => {
            if (!dragging) return;
            setDragging(false);
            seekAt(e);
          }}
          onPointerCancel={() => setDragging(false)}
        >
          <circle className="vnote-progress-hit" cx="50" cy="50" r={RING_RADIUS} />
          <circle
            className="vnote-progress-fill"
            cx="50"
            cy="50"
            r={RING_RADIUS}
            strokeDasharray={RING_LENGTH}
            strokeDashoffset={RING_LENGTH * (1 - progress)}
          />
        </svg>
      )}
      <span className="vnote-meta">
        {!active && <span className="vnote-muted" aria-hidden="true" />}
        {formatVoiceDuration(shownMs)}
        {showListenDot && !listened && (
          <span className="vnote-dot" title={i18n.t("voiceUnlistened", { ns: "chat" })} />
        )}
      </span>
    </div>
  );
};

export default VideoNotePlayer;
