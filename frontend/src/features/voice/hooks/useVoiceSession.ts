import { useCallback, useEffect, useRef, useState } from "react";

import { communityAPI } from "../../communities/api/community";
import { getLiveKitService } from "../lib/livekit";
import { useCallStore } from "../store/callStore";
import { RECONNECT_WINDOW_MS } from "../lib/sessionHint";
import type { Channel } from "../../../shared/types/channel";
import type { Community } from "../../../shared/types/community";

const REJOIN_EVERY_MS = 3000;

export function useVoiceSession() {
  const [joinedChannel, setJoinedChannel] = useState<{
    channel: Channel;
    communityId: string;
    password?: string;
  } | null>(null);
  const [voiceJoinFailed, setVoiceJoinFailed] = useState(false);
  const [voiceJoinAttempt, setVoiceJoinAttempt] = useState(0);
  const [voiceReconnecting, setVoiceReconnecting] = useState(false);
  const reconnectSinceRef = useRef<number | null>(null);
  const rejoinTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const stopReconnecting = useCallback((reconnecting = false) => {
    if (rejoinTimerRef.current) clearTimeout(rejoinTimerRef.current);
    rejoinTimerRef.current = null;
    reconnectSinceRef.current = reconnecting ? Date.now() : null;
    setVoiceReconnecting(reconnecting);
  }, []);
  useEffect(() => () => stopReconnecting(), [stopReconnecting]);
  const [passwordPrompt, setPasswordPrompt] = useState<{
    community: Community;
    channel: Channel;
  } | null>(null);
  const handleVoiceLeave = useCallback(() => {
    setJoinedChannel(null);
    setVoiceJoinFailed(false);
    stopReconnecting();
  }, [stopReconnecting]);

  const reconnectVoice = useCallback(() => {
    if (reconnectSinceRef.current === null) stopReconnecting(true);
    setVoiceJoinFailed(false);
    setVoiceJoinAttempt((n) => n + 1);
  }, [stopReconnecting]);

  const handleVoiceReconnected = useCallback(() => {
    stopReconnecting();
  }, [stopReconnecting]);


  const handleVoiceJoinFailed = useCallback(() => {
    const since = reconnectSinceRef.current;
    if (since !== null && Date.now() - since < RECONNECT_WINDOW_MS) {
      rejoinTimerRef.current = setTimeout(() => {
        rejoinTimerRef.current = null;
        setVoiceJoinAttempt((n) => n + 1);
      }, REJOIN_EVERY_MS);
      return;
    }
    stopReconnecting();
    setVoiceJoinFailed(true);
  }, [stopReconnecting]);

  const activeCallId = useCallStore((s) => s.activeCallId);
  const prevActiveCallRef = useRef<string | null>(null);
  useEffect(() => {
    const prev = prevActiveCallRef.current;
    prevActiveCallRef.current = activeCallId;
    if (!prev && activeCallId && joinedChannel) {
      setJoinedChannel(null);
      setVoiceJoinFailed(false);
      stopReconnecting();
    }
  }, [activeCallId, joinedChannel, stopReconnecting]);


  const startVoiceJoin = useCallback(
    (community: Community, channel: Channel, password?: string, reconnecting = false) => {
      void useCallStore.getState().leave(true);
      setJoinedChannel({ channel, communityId: community.id, password });
      setVoiceJoinFailed(false);
      stopReconnecting(reconnecting);
      setVoiceJoinAttempt((n) => n + 1);
    },
    [stopReconnecting],
  );

  const watchCommunityId = joinedChannel?.communityId;
  const watchChannelId = joinedChannel?.channel.id;
  const watchStateRef = useRef(
    new Map<string, { desired: boolean; running: boolean }>(),
  );
  const reportStreamWatch = useCallback(
    (streamerId: string, watching: boolean) => {
      if (!watchCommunityId || !watchChannelId) return;
      const key = `${watchCommunityId}:${watchChannelId}:${streamerId}`;
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
            await communityAPI.updateStreamViewer(
              watchCommunityId,
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
    [watchCommunityId, watchChannelId],
  );


  return {
    joinedChannel,
    setJoinedChannel,
    voiceJoinFailed,
    setVoiceJoinFailed,
    voiceJoinAttempt,
    voiceReconnecting,
    reconnectVoice,
    handleVoiceReconnected,
    passwordPrompt,
    setPasswordPrompt,
    handleVoiceLeave,
    handleVoiceJoinFailed,
    startVoiceJoin,
    reportStreamWatch,
  };
}
