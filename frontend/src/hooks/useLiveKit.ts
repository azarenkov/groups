import { useState, useEffect, useCallback, useRef } from "react";
import {
  ConnectionQuality,
  DisconnectReason,
  getLiveKitService,
  LiveKitService,
  NoiseSuppressionMode,
  ParticipantInfo,
} from "../services/livekit";
import { channelAPI } from "../api/channel";
import { useAuthStore } from "../store/authStore";
import { decodeAccessToken } from "../utils/jwt";
import { useLocalQuality } from "./useLocalQuality";

interface UseLiveKitResult {
  participants: ParticipantInfo[];
  isConnected: boolean;
  isMuted: boolean;
  isConnecting: boolean;
  error: string | null;
  localQuality: ConnectionQuality;
  getNoiseSuppressionMode: () => NoiseSuppressionMode;
  joinChannel: (
    channelId: string,
    serverId: string,
    password?: string,
  ) => Promise<void>;
  leaveChannel: () => Promise<void>;
  toggleMute: () => void;
  getLocalAudioMediaStreamTrack: () => MediaStreamTrack | null;
  setNoiseSuppression: (mode: NoiseSuppressionMode) => Promise<void>;
  setNoiseStrength: (value: number) => void;
  getLocalSessionId: () => string | null;
}

interface UseLiveKitOptions {
  onUnexpectedDisconnect?: (reason: DisconnectReason) => void;
}

export function useLiveKit(options?: UseLiveKitOptions): UseLiveKitResult {
  const [participants, setParticipants] = useState<ParticipantInfo[]>([]);
  const [isConnected, setIsConnected] = useState(false);
  const [isMuted, setIsMuted] = useState(false);
  const [isConnecting, setIsConnecting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const localQuality = useLocalQuality();
  const activeChannelRef = useRef<{ serverId: string; channelId: string } | null>(null);
  const isMutedRef = useRef(false);
  const isConnectedRef = useRef(false);

  isMutedRef.current = isMuted;
  isConnectedRef.current = isConnected;

  const onUnexpectedDisconnectRef = useRef(options?.onUnexpectedDisconnect);
  onUnexpectedDisconnectRef.current = options?.onUnexpectedDisconnect;

  const serviceRef = useRef<LiveKitService | null>(null);
  serviceRef.current = getLiveKitService(
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
    (reason) => {
      onUnexpectedDisconnectRef.current?.(reason);
    },
  );

  const joinChannel = useCallback(
    async (channelId: string, serverId: string, password?: string) => {
      const service = serviceRef.current;
      if (!service) return;
      try {
        if (service.isConnectingNow()) {
          console.log("[useLiveKit] Already connecting, skipping join request");
          return;
        }

        if (service.isConnected()) {
          console.log("[useLiveKit] Already connected, disconnecting first");
          await service.disconnect();
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

        await service.connect(response.token, response.url);

        activeChannelRef.current = { serverId, channelId };
        setIsConnecting(false);
      } catch (err) {
        console.error("[useLiveKit] Failed to join channel:", err);
        setError(err instanceof Error ? err.message : "Failed to join channel");
        setIsConnecting(false);
        throw err;
      }
    },
    [],
  );

  const leaveChannel = useCallback(async () => {
    const service = serviceRef.current;
    if (!service) return;
    try {
      console.log("[useLiveKit] Leaving channel...");
      await service.disconnect();
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
    const service = serviceRef.current;
    if (!service) return;
    const next = !isMutedRef.current;
    service.setMicrophoneMuted(next);
    setIsMuted(next);

    const active = activeChannelRef.current;
    const auth = useAuthStore.getState();
    const userId = auth.user?.id || decodeAccessToken(auth.tokens?.accessToken ?? "")?.sub;
    console.log("[useLiveKit] toggleMute", { next, active, userId });
    if (active && userId) {
      const call = next
        ? channelAPI.mute(active.serverId, active.channelId, userId)
        : channelAPI.unmute(active.serverId, active.channelId, userId);
      call.catch((err) => {
        console.error("[useLiveKit] Failed to sync mute state with backend:", err);
      });
    } else {
      console.warn("[useLiveKit] Skipping mute API: missing active channel or user id");
    }
  }, []);

  useEffect(() => {
    return () => {
      if (isConnectedRef.current) {
        serviceRef.current?.disconnect();
      }
    };
  }, []);

  const getLocalAudioMediaStreamTrack = useCallback(
    () => serviceRef.current?.getLocalAudioMediaStreamTrack() ?? null,
    [],
  );

  const setNoiseSuppression = useCallback(
    async (mode: NoiseSuppressionMode) => {
      await serviceRef.current?.setNoiseSuppressionMode(mode);
    },
    [],
  );

  const setNoiseStrength = useCallback((value: number) => {
    serviceRef.current?.setNoiseStrength(value);
  }, []);

  const getNoiseSuppressionMode = useCallback(
    (): NoiseSuppressionMode =>
      serviceRef.current?.getNoiseSuppressionMode() ?? "standard",
    [],
  );

  const getLocalSessionId = useCallback(
    () => serviceRef.current?.getLocalSessionId() ?? null,
    [],
  );

  return {
    participants,
    isConnected,
    isMuted,
    isConnecting,
    error,
    localQuality,
    joinChannel,
    leaveChannel,
    toggleMute,
    getLocalAudioMediaStreamTrack,
    setNoiseSuppression,
    setNoiseStrength,
    getNoiseSuppressionMode,
    getLocalSessionId,
  };
}
