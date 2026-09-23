import { useState, useEffect, useCallback, useRef } from "react";
import {
  ConnectionQuality,
  DisconnectReason,
  getLiveKitService,
  LiveKitService,
  NoiseSuppressionMode,
  ParticipantInfo,
  VideoTile,
} from "../lib/livekit";
import { channelAPI } from "../../communities/api/channel";
import { useAuthStore } from "../../auth/store/authStore";
import { useVoicePresenceStore } from "../store/voicePresenceStore";
import { decodeAccessToken } from "../../../shared/lib/jwt";
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
  isModeratorMuted: boolean;
  moderatorMutedBy: string | null;
  isDeafened: boolean;
  isModeratorDeafened: boolean;
  moderatorDeafenedBy: string | null;
  isConnecting: boolean;
  error: string | null;
  localQuality: ConnectionQuality;
  getNoiseSuppressionMode: () => NoiseSuppressionMode;
  joinChannel: (
    channelId: string,
    communityId: string,
    password?: string,
  ) => Promise<void>;
  leaveChannel: () => Promise<void>;
  toggleMute: () => void;
  toggleDeafen: () => void;
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
  const activeChannelRef = useRef<{ communityId: string; channelId: string } | null>(null);
  const fallbackMutedRef = useRef(false);
  const fallbackReleasePendingRef = useRef(false);
  const isMutedRef = useRef(false);
  const isConnectedRef = useRef(false);
  const [isDeafened, setIsDeafened] = useState(false);
  const isDeafenedRef = useRef(false);
  const micMutedBeforeDeafenRef = useRef(false);
  const deafenLockedRef = useRef(false);
  const selfDeafenedBeforeLockRef = useRef(false);
  const selfMutedBeforeDeafenLockRef = useRef(false);
  const [isModeratorDeafened, setIsModeratorDeafened] = useState(false);
  const [moderatorDeafenedBy, setModeratorDeafenedBy] = useState<string | null>(null);

  isMutedRef.current = isMuted;
  isConnectedRef.current = isConnected;

  const setDeafenLock = (locked: boolean, by?: string | null) => {
    deafenLockedRef.current = locked;
    setIsModeratorDeafened(locked);
    setModeratorDeafenedBy(locked ? (by ?? null) : null);
  };
  const applySelfDeafened = (value: boolean) => {
    isDeafenedRef.current = value;
    setIsDeafened(value);
  };

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
    async (channelId: string, communityId: string, password?: string) => {
      const service = serviceRef.current;
      if (!service) return;
      let joinAttempt: { communityId: string; channelId: string } | null = null;
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
        activeChannelRef.current = { communityId, channelId };
        joinAttempt = activeChannelRef.current;
        const joinAuth = useAuthStore.getState();
        const joiningUserId =
          joinAuth.user?.id ||
          decodeAccessToken(joinAuth.tokens?.accessToken ?? "")?.sub ||
          null;
        setLock(false);
        selfMutedBeforeLockRef.current = false;
        setDeafenLock(false);
        selfDeafenedBeforeLockRef.current = false;

        console.log(
          `[useLiveKit] Joining channel ${channelId} on community ${communityId}`,
        );

        const response = await channelAPI.joinVoice(communityId, channelId, {
          password,
        });
        const postTokenAuth = useAuthStore.getState();
        const postTokenId =
          postTokenAuth.user?.id ||
          decodeAccessToken(postTokenAuth.tokens?.accessToken ?? "")?.sub ||
          null;
        if (
          activeChannelRef.current !== joinAttempt ||
          postTokenId !== joiningUserId
        ) {
          if (activeChannelRef.current === joinAttempt) {
            activeChannelRef.current = null;
            setIsConnecting(false);
          } else if (activeChannelRef.current === null) {
            setIsConnecting(false);
          }
          return;
        }

        console.log(`[useLiveKit] Connecting to LiveKit at ${response.url}`);

        if (response.muted) {
          const auth = useAuthStore.getState();
          const selfId =
            auth.user?.id || decodeAccessToken(auth.tokens?.accessToken ?? "")?.sub;
          const locked = !!response.mutedBy && response.mutedBy !== selfId;
          setLock(locked, response.mutedBy ?? null);
          selfMutedBeforeLockRef.current = locked && !!response.selfMuted;
          setIsMuted(true);
        }
        if (
          response.deafenedBy &&
          joiningUserId &&
          response.deafenedBy !== joiningUserId
        ) {
          selfDeafenedBeforeLockRef.current = false;
          const moderatorMutedOnJoin =
            !!response.mutedBy && response.mutedBy !== joiningUserId;
          selfMutedBeforeDeafenLockRef.current =
            !!response.muted && (!moderatorMutedOnJoin || !!response.selfMuted);
          setDeafenLock(true, response.deafenedBy);
          setIsMuted(true);
          useVoicePresenceStore
            .getState()
            .setChannelDeafen(channelId, joiningUserId, response.deafenedBy, communityId);
        }
        const { awaitVoiceSettingsSyncedFor } = await import(
          "../store/voiceSettingsStore"
        );
        const settingsSynced = await awaitVoiceSettingsSyncedFor(joiningUserId);
        const afterAuth = useAuthStore.getState();
        const afterId =
          afterAuth.user?.id ||
          decodeAccessToken(afterAuth.tokens?.accessToken ?? "")?.sub ||
          null;
        if (
          afterId !== joiningUserId ||
          activeChannelRef.current !== joinAttempt
        ) {
          if (activeChannelRef.current === joinAttempt) {
            activeChannelRef.current = null;
            setIsConnecting(false);
          }
          return;
        }
        if (!response.muted && !settingsSynced) {
          fallbackMutedRef.current = true;
          setIsMuted(true);
        }
        await service.connect(
          response.token,
          response.url,
          channelId,
          !!response.muted || deafenLockedRef.current || !settingsSynced,
        );
        const postAuth = useAuthStore.getState();
        const postId =
          postAuth.user?.id ||
          decodeAccessToken(postAuth.tokens?.accessToken ?? "")?.sub ||
          null;
        if (
          postId !== joiningUserId ||
          activeChannelRef.current !== joinAttempt
        ) {
          if (service.getActiveRoomName() === channelId) {
            await service.disconnect().catch(() => {});
          }
          if (activeChannelRef.current === joinAttempt) {
            activeChannelRef.current = null;
            setIsConnecting(false);
          }
          return;
        }
        if (deafenLockedRef.current || isDeafenedRef.current) {
          service.setDeafened(true);
        }
        if (!response.muted && !settingsSynced && joiningUserId) {
          void import("../store/voiceSettingsStore").then(
            ({ voiceSettingsReadyFor }) =>
              voiceSettingsReadyFor(joiningUserId).then(() => {
                if (!fallbackMutedRef.current) return;
                if (activeChannelRef.current !== joinAttempt) return;
                if (moderatorLockedRef.current || deafenLockedRef.current) {
                  fallbackReleasePendingRef.current = true;
                  return;
                }
                fallbackMutedRef.current = false;
                fallbackReleasePendingRef.current = false;
                serviceRef.current?.setMicrophoneMuted(false);
                setIsMuted(false);
              }),
          );
        }

        setIsConnecting(false);
      } catch (err) {
        console.error("[useLiveKit] Failed to join channel:", err);
        if (joinAttempt === null || activeChannelRef.current === joinAttempt) {
          activeChannelRef.current = null;
          setLock(false);
          selfMutedBeforeLockRef.current = false;
          setDeafenLock(false);
          selfDeafenedBeforeLockRef.current = false;
          setError(
            err instanceof Error ? err.message : "Failed to join channel",
          );
          setIsConnecting(false);
        }
        throw err;
      }
    },
    [],
  );

  const leaveChannel = useCallback(async () => {
    const service = serviceRef.current;
    if (!service) return;
    fallbackMutedRef.current = false;
    fallbackReleasePendingRef.current = false;
    try {
      const active = activeChannelRef.current;
      const ownsRoom = !active || service.getActiveRoomName() === active.channelId;
      if (ownsRoom) {
        console.log("[useLiveKit] Leaving channel...");
        await service.disconnect();
      } else {
        console.log("[useLiveKit] Room superseded, skipping disconnect");
      }
      if (activeChannelRef.current === active) {
        activeChannelRef.current = null;
      }
      setParticipants([]);
      setVideoTiles([]);
      setIsCameraEnabled(false);
      setIsScreenShareEnabled(false);
      setIsConnected(false);
      setIsMuted(false);
      applySelfDeafened(false);
      setDeafenLock(false);
      selfDeafenedBeforeLockRef.current = false;
      selfMutedBeforeDeafenLockRef.current = false;
      micMutedBeforeDeafenRef.current = false;
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
  const [isModeratorMuted, setIsModeratorMuted] = useState(false);
  const [moderatorMutedBy, setModeratorMutedBy] = useState<string | null>(null);
  const setLock = (locked: boolean, by?: string | null) => {
    moderatorLockedRef.current = locked;
    setIsModeratorMuted(locked);
    setModeratorMutedBy(locked ? (by ?? null) : null);
  };

  const restoreSelfMute = () => {
    const active = activeChannelRef.current;
    const auth = useAuthStore.getState();
    const userId =
      auth.user?.id || decodeAccessToken(auth.tokens?.accessToken ?? "")?.sub;
    if (!active || !userId) return;
    channelAPI.mute(active.communityId, active.channelId, userId).catch((err) => {
      console.warn("[useLiveKit] failed to restore self-mute:", err);
    });
  };

  const selfMutedNow = () =>
    deafenLockedRef.current
      ? selfMutedBeforeDeafenLockRef.current
      : isMutedRef.current && !fallbackMutedRef.current;
  const actionsRef = useRef<{
    liftModeratorLock: () => void;
    engageDeafenLock: (by: string | null) => void;
    liftDeafenLock: () => void;
    releaseSelfDeafen: () => void;
    setSelfMuted: (next: boolean) => void;
  }>({
    liftModeratorLock: () => {},
    engageDeafenLock: () => {},
    liftDeafenLock: () => {},
    releaseSelfDeafen: () => {},
    setSelfMuted: () => {},
  });

  const liftModeratorLock = () => {
    setLock(false);
    if (selfMutedBeforeLockRef.current) {
      selfMutedBeforeLockRef.current = false;
      restoreSelfMute();
      return;
    }
    if (deafenLockedRef.current) return;
    if (fallbackMutedRef.current && !fallbackReleasePendingRef.current) {
      return;
    }
    fallbackReleasePendingRef.current = false;
    fallbackMutedRef.current = false;
    fallbackReleasePendingRef.current = false;
    serviceRef.current?.setMicrophoneMuted(false);
    setIsMuted(false);
  };
  const engageDeafenLock = (by: string | null) => {
    if (!deafenLockedRef.current) {
      selfDeafenedBeforeLockRef.current = isDeafenedRef.current;
      selfMutedBeforeDeafenLockRef.current = moderatorLockedRef.current
        ? selfMutedBeforeLockRef.current
        : isMutedRef.current && !fallbackMutedRef.current;
    }
    setDeafenLock(true, by);
    serviceRef.current?.setDeafened(true);
    serviceRef.current?.setMicrophoneMuted(true);
    isMutedRef.current = true;
    setIsMuted(true);
  };
  const liftDeafenLock = () => {
    setDeafenLock(false);
    const wasSelfDeafened = selfDeafenedBeforeLockRef.current;
    const wasSelfMuted = selfMutedBeforeDeafenLockRef.current;
    selfDeafenedBeforeLockRef.current = false;
    selfMutedBeforeDeafenLockRef.current = false;
    if (!wasSelfDeafened && !isDeafenedRef.current) {
      serviceRef.current?.setDeafened(false);
    }
    if (wasSelfDeafened || wasSelfMuted || moderatorLockedRef.current) return;
    if (fallbackMutedRef.current && !fallbackReleasePendingRef.current) return;
    fallbackMutedRef.current = false;
    fallbackReleasePendingRef.current = false;
    serviceRef.current?.setMicrophoneMuted(false);
    isMutedRef.current = false;
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
        selfMutedBeforeLockRef.current = selfMutedNow();
        setLock(true, me.mutedBy ?? null);
        serviceRef.current?.setMicrophoneMuted(true);
        setIsMuted(true);
      } else if (!moderatorMuted && moderatorLockedRef.current) {
        if (me.isMuted) {
          setLock(false);
          selfMutedBeforeLockRef.current = false;
        } else {
          actionsRef.current.liftModeratorLock();
        }
      }
      const moderatorDeafened = !!me.deafenedBy && me.deafenedBy !== selfId;
      if (moderatorDeafened && !deafenLockedRef.current) {
        actionsRef.current.engageDeafenLock(me.deafenedBy ?? null);
      } else if (!moderatorDeafened && deafenLockedRef.current) {
        actionsRef.current.liftDeafenLock();
      }
    };
    const unsub = useVoicePresenceStore.subscribe(reconcile);
    return unsub;
  }, []);
  useEffect(() => {
    const onModeratorMute = (e: Event) => {
      const detail = (
        e as CustomEvent<{ channelId?: string; mutedBy?: string }>
      ).detail;
      const active = activeChannelRef.current;
      if (!detail?.channelId || !active || active.channelId !== detail.channelId) {
        return;
      }
      if (!moderatorLockedRef.current) {
        selfMutedBeforeLockRef.current = selfMutedNow();
      }
      serviceRef.current?.setMicrophoneMuted(true);
      setIsMuted(true);
      setLock(true, detail.mutedBy ?? null);
    };
    const onModeratorUnmute = (e: Event) => {
      const detail = (e as CustomEvent<{ channelId?: string }>).detail;
      const active = activeChannelRef.current;
      if (!detail?.channelId || !active || active.channelId !== detail.channelId) {
        return;
      }
      if (moderatorLockedRef.current) {
        actionsRef.current.liftModeratorLock();
      }
    };
    const onModeratorDeafen = (e: Event) => {
      const detail = (
        e as CustomEvent<{ channelId?: string; deafenedBy?: string }>
      ).detail;
      const active = activeChannelRef.current;
      if (!detail?.channelId || !active || active.channelId !== detail.channelId) {
        return;
      }
      actionsRef.current.engageDeafenLock(detail.deafenedBy ?? null);
    };
    const onModeratorUndeafen = (e: Event) => {
      const detail = (e as CustomEvent<{ channelId?: string }>).detail;
      const active = activeChannelRef.current;
      if (!detail?.channelId || !active || active.channelId !== detail.channelId) {
        return;
      }
      if (deafenLockedRef.current) actionsRef.current.liftDeafenLock();
    };
    window.addEventListener("rift:moderator-mute", onModeratorMute);
    window.addEventListener("rift:moderator-unmute", onModeratorUnmute);
    window.addEventListener("rift:moderator-deafen", onModeratorDeafen);
    window.addEventListener("rift:moderator-undeafen", onModeratorUndeafen);
    return () => {
      window.removeEventListener("rift:moderator-mute", onModeratorMute);
      window.removeEventListener("rift:moderator-unmute", onModeratorUnmute);
      window.removeEventListener("rift:moderator-deafen", onModeratorDeafen);
      window.removeEventListener("rift:moderator-undeafen", onModeratorUndeafen);
    };
  }, []);

  const setSelfMuted = (next: boolean) => {
    const service = serviceRef.current;
    if (!service) return;
    fallbackMutedRef.current = false;
    fallbackReleasePendingRef.current = false;
    if (!next && moderatorLockedRef.current) {
      const active = activeChannelRef.current;
      const auth = useAuthStore.getState();
      const userId =
        auth.user?.id || decodeAccessToken(auth.tokens?.accessToken ?? "")?.sub;
      if (!active || !userId) return;
      channelAPI
        .unmute(active.communityId, active.channelId, userId)
        .then(() => {
          setLock(false);
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
        ? channelAPI.mute(active.communityId, active.channelId, userId)
        : channelAPI.unmute(active.communityId, active.channelId, userId);
      call.catch((err) => {
        console.error("[useLiveKit] Failed to sync mute state with backend:", err);
      });
    } else {
      console.warn("[useLiveKit] Skipping mute API: missing active channel or user id");
    }
  };

  const toggleMute = useCallback(() => {
    if (deafenLockedRef.current) return;
    if (isDeafenedRef.current && isMutedRef.current) {
      micMutedBeforeDeafenRef.current = false;
      actionsRef.current.releaseSelfDeafen();
      return;
    }
    actionsRef.current.setSelfMuted(!isMutedRef.current);
  }, []);

  const clearSelfMuteFlag = () => {
    const active = activeChannelRef.current;
    const auth = useAuthStore.getState();
    const userId =
      auth.user?.id || decodeAccessToken(auth.tokens?.accessToken ?? "")?.sub;
    if (!active || !userId) return;
    channelAPI.unmute(active.communityId, active.channelId, userId).catch((err) => {
      console.warn("[useLiveKit] failed to clear self-mute under moderator mute:", err);
    });
  };

  const releaseSelfDeafen = () => {
    applySelfDeafened(false);
    if (!deafenLockedRef.current) serviceRef.current?.setDeafened(false);
    const deafenCreatedMute = !micMutedBeforeDeafenRef.current;
    micMutedBeforeDeafenRef.current = false;
    if (!deafenCreatedMute || !isMutedRef.current) return;
    if (!moderatorLockedRef.current) {
      setSelfMuted(false);
      return;
    }
    selfMutedBeforeLockRef.current = false;
    clearSelfMuteFlag();
  };

  actionsRef.current = {
    liftModeratorLock,
    engageDeafenLock,
    liftDeafenLock,
    releaseSelfDeafen,
    setSelfMuted,
  };

  const toggleDeafen = useCallback(() => {
    const service = serviceRef.current;
    if (!service) return;
    if (deafenLockedRef.current) {
      const active = activeChannelRef.current;
      const auth = useAuthStore.getState();
      const userId =
        auth.user?.id || decodeAccessToken(auth.tokens?.accessToken ?? "")?.sub;
      if (!active || !userId) return;
      channelAPI
        .undeafen(active.communityId, active.channelId, userId)
        .then(() => {
          setDeafenLock(false);
          selfDeafenedBeforeLockRef.current = false;
          actionsRef.current.releaseSelfDeafen();
        })
        .catch((err) => {
          console.warn("[useLiveKit] moderator deafen is still active:", err);
        });
      return;
    }
    if (isDeafenedRef.current) {
      actionsRef.current.releaseSelfDeafen();
      return;
    }
    const fallbackMuted = fallbackMutedRef.current;
    micMutedBeforeDeafenRef.current = isMutedRef.current && !fallbackMuted;
    applySelfDeafened(true);
    service.setDeafened(true);
    if (!isMutedRef.current || fallbackMuted) actionsRef.current.setSelfMuted(true);
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
    isModeratorMuted,
    moderatorMutedBy,
    isDeafened,
    isModeratorDeafened,
    moderatorDeafenedBy,
    isConnecting,
    error,
    localQuality,
    joinChannel,
    leaveChannel,
    toggleMute,
    toggleDeafen,
    getLocalAudioMediaStreamTrack,
    setNoiseSuppression,
    setNoiseStrength,
    getNoiseSuppressionMode,
    getLocalSessionId,
  };
}
