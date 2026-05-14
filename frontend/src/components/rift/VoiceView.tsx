import React, { useEffect, useRef } from "react";
import Icon from "./Icon";
import { useLiveKit } from "../../hooks/useLiveKit";

const initials = (s: string) => (s ? s.charAt(0).toUpperCase() : "?");

const AVA_COLORS = ["#0071e3", "#ff6b35", "#30a46c", "#bf5af2", "#ff9f0a"];
const colorFor = (id: string): string => {
  let h = 0;
  for (let i = 0; i < id.length; i++) h = (h * 31 + id.charCodeAt(i)) >>> 0;
  return AVA_COLORS[h % AVA_COLORS.length];
};

interface VoiceViewProps {
  channelId: string;
  serverId: string;
  channelName: string;
  currentUserName: string;
  password?: string;
  onLeave: () => void;
}

const VoiceView: React.FC<VoiceViewProps> = ({
  channelId,
  serverId,
  channelName,
  currentUserName,
  password,
  onLeave,
}) => {
  const {
    participants,
    isConnected,
    isMuted,
    isConnecting,
    error,
    joinChannel,
    leaveChannel,
    toggleMute,
    getLocalAudioMediaStreamTrack,
    setNoiseSuppression,
  } = useLiveKit();

  const hasJoinedRef = useRef(false);
  const currentChannelRef = useRef<string | null>(null);
  const isJoiningRef = useRef(false);

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
        });
    }

    return () => {
      hasJoinedRef.current = false;
      currentChannelRef.current = null;
      isJoiningRef.current = false;
      leaveChannel();
    };
  }, [channelId, serverId, password]);

  const handleLeave = async () => {
    await leaveChannel();
    onLeave();
  };

  const [open, setOpen] = React.useState(false);
  const [volumes, setVolumes] = React.useState<Record<string, number>>({});
  const [noise, setNoise] = React.useState<"off" | "standard">("standard");

  const handleNoiseChange = (next: "off" | "standard") => {
    setNoise(next);
    setNoiseSuppression(next === "standard").catch((e) =>
      console.warn("[VoiceView] noise suppression toggle failed:", e),
    );
  };
  const rootRef = useRef<HTMLDivElement>(null);

  const speakingPerson = participants.find((p) => p.isSpeaking);

  const getVol = (id: string) => volumes[id] ?? 100;
  const setVol = (id: string, v: number) =>
    setVolumes((prev) => ({ ...prev, [id]: v }));

  const avgVol =
    participants.length > 0
      ? Math.round(
          participants.reduce((s, p) => s + getVol(p.identity), 0) /
            participants.length,
        )
      : 100;

  const volIcon =
    avgVol === 0 ? "speaker-off" : avgVol < 40 ? "speaker-lo" : "speaker-hi";

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

  useEffect(() => {
    console.log("[meter] effect run", { open, isMuted, isConnected });
    if (!open || isMuted || !isConnected) {
      setMeter(new Array(BARS).fill(0));
      return;
    }

    const track = getLocalAudioMediaStreamTrack();
    console.log("[meter] got track:", track, "readyState:", track?.readyState);
    if (!track) {
      console.warn("[meter] no local audio track");
      return;
    }

    const AudioCtx =
      window.AudioContext ||
      (window as unknown as { webkitAudioContext: typeof AudioContext })
        .webkitAudioContext;
    let ctx: AudioContext;
    let source: MediaStreamAudioSourceNode;
    let analyser: AnalyserNode;
    try {
      ctx = new AudioCtx();
      source = ctx.createMediaStreamSource(new MediaStream([track]));
      analyser = ctx.createAnalyser();
      analyser.fftSize = 256;
      analyser.smoothingTimeConstant = 0.75;
      source.connect(analyser);
      console.log("[meter] audio graph ready, ctx state:", ctx.state);
    } catch (e) {
      console.error("[meter] failed to set up audio graph:", e);
      return;
    }

    if (ctx.state === "suspended") {
      ctx.resume().catch((e) => console.warn("[meter] resume failed", e));
    }

    const bins = analyser.frequencyBinCount;
    const data = new Uint8Array(bins);
    const usable = Math.floor(bins * 0.6);
    const perBar = Math.max(1, Math.floor(usable / BARS));

    let raf = 0;
    let logged = 0;
    const tick = () => {
      analyser.getByteFrequencyData(data);
      const next = new Array(BARS).fill(0) as number[];
      for (let b = 0; b < BARS; b++) {
        let sum = 0;
        const start = b * perBar;
        for (let j = 0; j < perBar; j++) sum += data[start + j] || 0;
        next[b] = Math.min(1, sum / perBar / 200);
      }
      if (logged < 3) {
        console.log("[meter] frame", logged, "max bar:", Math.max(...next).toFixed(2));
        logged++;
      }
      setMeter(next);
      raf = requestAnimationFrame(tick);
    };
    raf = requestAnimationFrame(tick);

    return () => {
      cancelAnimationFrame(raf);
      try {
        source.disconnect();
      } catch {}
      ctx.close().catch(() => {});
    };
  }, [open, isMuted, isConnected, getLocalAudioMediaStreamTrack]);

  const totalConnected = isConnected ? participants.length + 1 : 0;
  const youColor = "#0071e3";
  const allParticipants = [
    {
      id: "you",
      name: `${currentUserName} (you)`,
      initials: initials(currentUserName),
      color: youColor,
    },
    ...participants.map((p) => ({
      id: p.identity,
      name: p.name || p.identity,
      initials: initials(p.name || p.identity),
      color: colorFor(p.identity),
    })),
  ];

  if (error) {
    return (
      <div
        className="voice-pill"
        style={{ background: "var(--danger-tint)", color: "var(--danger)" }}
      >
        <span>{error}</span>
        <button className="vp-btn leave" onClick={handleLeave} title="Leave">
          <Icon name="logout" size={15} />
        </button>
      </div>
    );
  }

  if (isConnecting && !isConnected) {
    return (
      <div className="voice-pill">
        <span className="vp-dot" />
        <div>
          <div className="vp-title">{channelName}</div>
          <div className="vp-meta">Connecting…</div>
        </div>
        <button className="vp-btn leave" onClick={handleLeave} title="Cancel">
          <Icon name="logout" size={15} />
        </button>
      </div>
    );
  }

  return (
    <div className="voice-pill" ref={rootRef}>
      <span className="vp-dot" />
      <div>
        <div className="vp-title">{channelName}</div>
        <div className="vp-meta">
          {speakingPerson
            ? `${speakingPerson.name || speakingPerson.identity} is speaking`
            : `${totalConnected} connected`}
        </div>
      </div>
      <div className="vp-avas">
        {allParticipants.slice(0, 4).map((p) => (
          <div key={p.id} className="ava" style={{ background: p.color }}>
            {p.initials}
          </div>
        ))}
      </div>
      <button
        className={"vp-btn" + (open ? " on" : "")}
        onClick={() => setOpen((v) => !v)}
        title="Audio settings"
      >
        <Icon name={volIcon} size={15} />
      </button>
      <button
        className={"vp-btn" + (isMuted ? " muted" : "")}
        onClick={toggleMute}
        title={isMuted ? "Unmute" : "Mute"}
      >
        <Icon name={isMuted ? "mic-off" : "mic"} size={15} />
      </button>
      <button className="vp-btn leave" onClick={handleLeave} title="Leave">
        <Icon name="logout" size={15} />
      </button>

      {open && (
        <div className="vp-popover" onMouseDown={(e) => e.stopPropagation()}>
          <div className="vp-pop-title">Listening to</div>
          <div className="vp-people">
            {participants.length === 0 && (
              <div style={{ padding: "10px 8px", fontSize: 12, color: "var(--fg-3)" }}>
                No one else is here yet.
              </div>
            )}
            {participants.map((p) => {
              const v = getVol(p.identity);
              return (
                <div key={p.identity} className="vp-person">
                  <div
                    className={"ava" + (v === 0 ? " muted" : "")}
                    style={{ background: colorFor(p.identity) }}
                  >
                    {initials(p.name || p.identity)}
                  </div>
                  <div className="name">{p.name || p.identity}</div>
                  <div className="vp-slider">
                    <input
                      type="range"
                      min={0}
                      max={200}
                      step={1}
                      value={v}
                      onChange={(e) => setVol(p.identity, Number(e.target.value))}
                      style={{ ["--pct" as unknown as string]: `${v / 2}%` } as React.CSSProperties}
                      title={`${v}%`}
                    />
                  </div>
                  <div className="pct">{v}%</div>
                </div>
              );
            })}
          </div>

          <div className="vp-pop-row">
            <div className="vp-pop-label">
              <span className="k">Noise suppression</span>
            </div>
            <div className="vp-seg">
              <button
                className={noise === "off" ? "on" : ""}
                onClick={() => handleNoiseChange("off")}
              >
                Off
              </button>
              <button
                className={noise === "standard" ? "on" : ""}
                onClick={() => handleNoiseChange("standard")}
              >
                Standard
              </button>
              <button disabled title="Coming soon">
                <Icon name="sparkles" size={11} />
                High
              </button>
            </div>
          </div>

          <div className="vp-pop-row">
            <div className="vp-pop-label">
              <span className="k">Your microphone</span>
              <span className="v">{isMuted ? "Muted" : "Live"}</span>
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
    </div>
  );
};

export default VoiceView;
