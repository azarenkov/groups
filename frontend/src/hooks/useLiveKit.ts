import { useState, useEffect, useCallback, useRef } from "react";
import { getLiveKitService, ParticipantInfo } from "../services/livekit";
import { channelAPI } from "../api/channel";
import { useAuthStore } from "../store/authStore";
import { decodeAccessToken } from "../utils/jwt";

interface UseLiveKitResult {
  participants: ParticipantInfo[];
  isConnected: boolean;
  isMuted: boolean;
  isConnecting: boolean;
  error: string | null;
  joinChannel: (
    channelId: string,
    serverId: string,
    password?: string,
  ) => Promise<void>;
  leaveChannel: () => Promise<void>;
  toggleMute: () => void;
  getLocalAudioMediaStreamTrack: () => MediaStreamTrack | null;
  setNoiseSuppression: (enabled: boolean) => Promise<void>;
  getLocalSessionId: () => string | null;
}

export function useLiveKit(): UseLiveKitResult {
  const [participants, setParticipants] = useState<ParticipantInfo[]>([]);
  const [isConnected, setIsConnected] = useState(false);
  const [isMuted, setIsMuted] = useState(false);
  const [isConnecting, setIsConnecting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const activeChannelRef = useRef<{ serverId: string; channelId: string } | null>(null);

  const livekitService = getLiveKitService(
    (updatedParticipants) => {
      setParticipants(updatedParticipants);
    },
    (connected) => {
      setIsConnected(connected);
      if (!connected) {
        setIsConnecting(false);
      }
    },
    (err) => {
      setError(err.message);
      setIsConnecting(false);
    },
  );

  const joinChannel = useCallback(
    async (channelId: string, serverId: string, password?: string) => {
      try {
        if (livekitService.isConnectingNow()) {
          console.log("[useLiveKit] Already connecting, skipping join request");
          return;
        }

        if (livekitService.isConnected()) {
          console.log("[useLiveKit] Already connected, disconnecting first");
          await livekitService.disconnect();
          await new Promise((resolve) => setTimeout(resolve, 500));
        }

        setIsConnecting(true);
        setError(null);

        console.log(
          `[useLiveKit] Joining channel ${channelId} on server ${serverId}`,
        );

        const response = await channelAPI.joinVoice(serverId, channelId, {
          password,
        });

        console.log(`[useLiveKit] Connecting to LiveKit at ${response.url}`);

        await livekitService.connect(response.token, response.url);

        activeChannelRef.current = { serverId, channelId };
        setIsConnecting(false);
      } catch (err) {
        console.error("[useLiveKit] Failed to join channel:", err);
        setError(err instanceof Error ? err.message : "Failed to join channel");
        setIsConnecting(false);
        throw err;
      }
    },
    [livekitService],
  );

  const leaveChannel = useCallback(async () => {
    try {
      console.log("[useLiveKit] Leaving channel...");
      await livekitService.disconnect();
      activeChannelRef.current = null;
      setParticipants([]);
      setIsConnected(false);
      setIsMuted(false);
      setError(null);
    } catch (err) {
      console.error("[useLiveKit] Failed to leave channel:", err);
    }
  }, []);

  const toggleMute = useCallback(() => {
    const newMutedState = !isMuted;
    livekitService.setMicrophoneMuted(newMutedState);
    setIsMuted(newMutedState);

    const active = activeChannelRef.current;
    const auth = useAuthStore.getState();
    const userId = auth.user?.id || decodeAccessToken(auth.tokens?.accessToken ?? "")?.sub;
    console.log("[useLiveKit] toggleMute", { newMutedState, active, userId });
    if (active && userId) {
      const call = newMutedState
        ? channelAPI.mute(active.serverId, active.channelId, userId)
        : channelAPI.unmute(active.serverId, active.channelId, userId);
      call.catch((err) => {
        console.error("[useLiveKit] Failed to sync mute state with backend:", err);
      });
    } else {
      console.warn("[useLiveKit] Skipping mute API: missing active channel or user id");
    }
  }, [isMuted, livekitService]);

  // Clean up on unmount
  useEffect(() => {
    return () => {
      if (isConnected) {
        livekitService.disconnect();
      }
    };
  }, [isConnected, livekitService]);

  const getLocalAudioMediaStreamTrack = useCallback(
    () => livekitService.getLocalAudioMediaStreamTrack(),
    [livekitService],
  );

  const setNoiseSuppression = useCallback(
    (enabled: boolean) => livekitService.setNoiseSuppression(enabled),
    [livekitService],
  );

  const getLocalSessionId = useCallback(
    () => livekitService.getLocalSessionId(),
    [livekitService],
  );

  return {
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
    getLocalSessionId,
  };
}
