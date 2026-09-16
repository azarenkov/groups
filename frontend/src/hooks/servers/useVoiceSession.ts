import { useCallback, useEffect, useRef, useState } from "react";

import { serverAPI } from "../../api/server";
import { getLiveKitService } from "../../services/livekit";
import { useCallStore } from "../../store/callStore";
import type { Channel } from "../../types/channel";
import type { Server } from "../../types/server";

export function useVoiceSession() {
  const [joinedChannel, setJoinedChannel] = useState<{
    channel: Channel;
    serverId: string;
    password?: string;
  } | null>(null);
  const [voiceJoinFailed, setVoiceJoinFailed] = useState(false);
  const [voiceJoinAttempt, setVoiceJoinAttempt] = useState(0);
  const [passwordPrompt, setPasswordPrompt] = useState<{
    server: Server;
    channel: Channel;
  } | null>(null);
  const handleVoiceLeave = useCallback(() => {
    setJoinedChannel(null);
    setVoiceJoinFailed(false);
  }, []);

  const handleVoiceJoinFailed = useCallback(() => {
    setVoiceJoinFailed(true);
  }, []);

  const activeCallId = useCallStore((s) => s.activeCallId);
  const prevActiveCallRef = useRef<string | null>(null);
  useEffect(() => {
    const prev = prevActiveCallRef.current;
    prevActiveCallRef.current = activeCallId;
    if (!prev && activeCallId && joinedChannel) {
      setJoinedChannel(null);
      setVoiceJoinFailed(false);
    }
  }, [activeCallId, joinedChannel]);


  const startVoiceJoin = useCallback(
    (server: Server, channel: Channel, password?: string) => {
      void useCallStore.getState().leave(true);
      setJoinedChannel({ channel, serverId: server.id, password });
      setVoiceJoinFailed(false);
      setVoiceJoinAttempt((n) => n + 1);
    },
    [],
  );

  const watchServerId = joinedChannel?.serverId;
  const watchChannelId = joinedChannel?.channel.id;
  const watchStateRef = useRef(
    new Map<string, { desired: boolean; running: boolean }>(),
  );
  const reportStreamWatch = useCallback(
    (streamerId: string, watching: boolean) => {
      if (!watchServerId || !watchChannelId) return;
      const key = `${watchServerId}:${watchChannelId}:${streamerId}`;
      const states = watchStateRef.current;
      const entry = states.get(key) ?? { desired: watching, running: false };
      entry.desired = watching;
      states.set(key, entry);
      if (entry.running) return;
      entry.running = true;
      void (async () => {
        let lastSent: boolean | undefined;
        while (states.get(key) && states.get(key)!.desired !== lastSent) {
          const target = states.get(key)!.desired;
          const sessionId = getLiveKitService().getLocalSessionId();
          if (!sessionId) break;
          try {
            await serverAPI.updateStreamViewer(
              watchServerId,
              watchChannelId,
              streamerId,
              sessionId,
              target,
            );
            lastSent = target;
          } catch (err) {
            console.warn("[stream-viewers] report failed:", err);
            await new Promise((resolve) => setTimeout(resolve, 2000));
          }
        }
        const e = states.get(key);
        if (e) e.running = false;
      })();
    },
    [watchServerId, watchChannelId],
  );


  return {
    joinedChannel,
    setJoinedChannel,
    voiceJoinFailed,
    setVoiceJoinFailed,
    voiceJoinAttempt,
    passwordPrompt,
    setPasswordPrompt,
    handleVoiceLeave,
    handleVoiceJoinFailed,
    startVoiceJoin,
    reportStreamWatch,
  };
}
