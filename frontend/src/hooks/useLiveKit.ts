import { useState, useEffect, useCallback, useRef } from "react";
import {
  ConnectionQuality,
  DisconnectReason,
  getLiveKitService,
  LiveKitService,
  NoiseSuppressionMode,
  ParticipantInfo,
  VideoTile,
} from "../services/livekit";
import { channelAPI } from "../api/channel";
import { useAuthStore } from "../store/authStore";
import { useVoicePresenceStore } from "../store/voicePresenceStore";
import { decodeAccessToken } from "../utils/jwt";
import { useLocalQuality } from "./useLocalQuality";

interface UseLiveKitResult {
  participants: ParticipantInfo[];
  videoTiles: VideoTile[];
  isCameraEnabled: boolean;
  isScreenShareEnabled: boolean;
  toggleCamera: () => Promise<void>;
  toggleScreenShare: () => Promise<void>;
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
  const [videoTiles, setVideoTiles] = useState<VideoTile[]>([]);
  const [isCameraEnabled, setIsCameraEnabled] = useState(false);
  const [isScreenShareEnabled, setIsScreenShareEnabled] = useState(false);
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
      const service = serviceRef.current;
      if (service) {
        setVideoTiles(service.getVideoTiles());
        setIsCameraEnabled(service.isCameraEnabled());
        setIsScreenShareEnabled(service.isScreenShareEnabled());
      }
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
        activeChannelRef.current = { serverId, channelId };
        moderatorLockedRef.current = false;
        selfMutedBeforeLockRef.current = false;

        console.log(
          `[useLiveKit] Joining channel ${channelId} on server ${serverId}`,
        );

        const response = await channelAPI.joinVoice(serverId, channelId, {
          password,
        });

        console.log(`[useLiveKit] Connecting to LiveKit at ${response.url}`);

        if (response.muted) {
          const auth = useAuthStore.getState();
          const selfId =
            auth.user?.id || decodeAccessToken(auth.tokens?.accessToken ?? "")?.sub;
          moderatorLockedRef.current =
            !!response.mutedBy && response.mutedBy !== selfId;
          setIsMuted(true);
        }
        await service.connect(
          response.token,
          response.url,
          channelId,
          !!response.muted,
        );

        setIsConnecting(false);
      } catch (err) {
        console.error("[useLiveKit] Failed to join channel:", err);
        activeChannelRef.current = null;
        moderatorLockedRef.current = false;
        selfMutedBeforeLockRef.current = false;
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
      const active = activeChannelRef.current;
      const ownsRoom = !active || service.getActiveRoomName() === active.channelId;
      if (ownsRoom) {
        console.log("[useLiveKit] Leaving channel...");
        await service.disconnect();
      } else {
        console.log("[useLiveKit] Room superseded, skipping disconnect");
      }
      activeChannelRef.current = null;
      setParticipants([]);
      setVideoTiles([]);
      setIsCameraEnabled(false);
      setIsScreenShareEnabled(false);
      setIsConnected(false);
      setIsMuted(false);
      setError(null);
    } catch (err) {
      console.error("[useLiveKit] Failed to leave channel:", err);
    }
  }, []);

  const toggleCamera = useCallback(async () => {
    const service = serviceRef.current;
    if (!service) return;
    try {
      await service.setCameraEnabled(!service.isCameraEnabled());
    } catch (err) {
      console.error("[useLiveKit] Failed to toggle camera:", err);
    }
  }, []);

  const toggleScreenShare = useCallback(async () => {
    const service = serviceRef.current;
    if (!service) return;
    try {
      await service.setScreenShareEnabled(!service.isScreenShareEnabled());
    } catch (err) {
      console.error("[useLiveKit] Failed to toggle screen share:", err);
    }
  }, []);

  const moderatorLockedRef = useRef(false);
  const selfMutedBeforeLockRef = useRef(false);

  const restoreSelfMute = () => {
    const active = activeChannelRef.current;
    const auth = useAuthStore.getState();
    const userId =
      auth.user?.id || decodeAccessToken(auth.tokens?.accessToken ?? "")?.sub;
    if (!active || !userId) return;
    channelAPI.mute(active.serverId, active.channelId, userId).catch((err) => {
      console.warn("[useLiveKit] failed to restore self-mute:", err);
    });
  };

  const liftModeratorLock = () => {
    moderatorLockedRef.current = false;
    if (selfMutedBeforeLockRef.current) {
      selfMutedBeforeLockRef.current = false;
      restoreSelfMute();
      return;
    }
    serviceRef.current?.setMicrophoneMuted(false);
    setIsMuted(false);
  };
  useEffect(() => {
    const reconcile = () => {
      const active = activeChannelRef.current;
      if (!active || !isConnectedRef.current) return;
      const auth = useAuthStore.getState();
      const selfId =
        auth.user?.id || decodeAccessToken(auth.tokens?.accessToken ?? "")?.sub;
      if (!selfId) return;
      const me = useVoicePresenceStore
        .getState()
        .byChannel[active.channelId]?.find((m) => m.id === selfId);
      if (!me) return;
      const moderatorMuted =
        me.isMuted && !!me.mutedBy && me.mutedBy !== selfId;
      if (moderatorMuted && !moderatorLockedRef.current) {
        selfMutedBeforeLockRef.current = isMutedRef.current;
        moderatorLockedRef.current = true;
        serviceRef.current?.setMicrophoneMuted(true);
        setIsMuted(true);
      } else if (!moderatorMuted && moderatorLockedRef.current) {
        if (me.isMuted) {
          moderatorLockedRef.current = false;
          selfMutedBeforeLockRef.current = false;
        } else {
          liftModeratorLock();
        }
      }
    };
    const unsub = useVoicePresenceStore.subscribe(reconcile);
    return unsub;
  }, []);
  useEffect(() => {
    const onModeratorMute = (e: Event) => {
      const detail = (e as CustomEvent<{ channelId?: string }>).detail;
      const active = activeChannelRef.current;
      if (!detail?.channelId || !active || active.channelId !== detail.channelId) {
        return;
      }
      if (!moderatorLockedRef.current) {
        selfMutedBeforeLockRef.current = isMutedRef.current;
      }
      serviceRef.current?.setMicrophoneMuted(true);
      setIsMuted(true);
      moderatorLockedRef.current = true;
    };
    const onModeratorUnmute = (e: Event) => {
      const detail = (e as CustomEvent<{ channelId?: string }>).detail;
      const active = activeChannelRef.current;
      if (!detail?.channelId || !active || active.channelId !== detail.channelId) {
        return;
      }
      if (moderatorLockedRef.current) {
        liftModeratorLock();
      }
    };
    window.addEventListener("rift:moderator-mute", onModeratorMute);
    window.addEventListener("rift:moderator-unmute", onModeratorUnmute);
    return () => {
      window.removeEventListener("rift:moderator-mute", onModeratorMute);
      window.removeEventListener("rift:moderator-unmute", onModeratorUnmute);
    };
  }, []);

  const toggleMute = useCallback(() => {
    const service = serviceRef.current;
    if (!service) return;
    const next = !isMutedRef.current;
    if (!next && moderatorLockedRef.current) {
      const active = activeChannelRef.current;
      const auth = useAuthStore.getState();
      const userId =
        auth.user?.id || decodeAccessToken(auth.tokens?.accessToken ?? "")?.sub;
      if (!active || !userId) return;
      channelAPI
        .unmute(active.serverId, active.channelId, userId)
        .then(() => {
          moderatorLockedRef.current = false;
          selfMutedBeforeLockRef.current = false;
          serviceRef.current?.setMicrophoneMuted(false);
          setIsMuted(false);
        })
        .catch((err) => {
          console.warn("[useLiveKit] moderator mute is still active:", err);
        });
      return;
    }
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
      const active = activeChannelRef.current;
      const service = serviceRef.current;
      if (
        isConnectedRef.current &&
        service &&
        (!active || service.getRoomName() === active.channelId)
      ) {
        service.disconnect();
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
    videoTiles,
    isCameraEnabled,
    isScreenShareEnabled,
    toggleCamera,
    toggleScreenShare,
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
