import { useState, useEffect, useCallback } from "react";
import { getLiveKitService, ParticipantInfo } from "../services/livekit";
import { channelAPI } from "../api/channel";

interface UseLiveKitResult {
  participants: ParticipantInfo[];
  isConnected: boolean;
  isMuted: boolean;
  isConnecting: boolean;
  error: string | null;
  joinChannel: (channelId: string, serverId: string) => Promise<void>;
  leaveChannel: () => Promise<void>;
  toggleMute: () => void;
}

export function useLiveKit(): UseLiveKitResult {
  const [participants, setParticipants] = useState<ParticipantInfo[]>([]);
  const [isConnected, setIsConnected] = useState(false);
  const [isMuted, setIsMuted] = useState(false);
  const [isConnecting, setIsConnecting] = useState(false);
  const [error, setError] = useState<string | null>(null);

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

  const joinChannel = useCallback(async (channelId: string, serverId: string) => {
    try {
      // Check if already connecting or connected
      if (livekitService.isConnectingNow()) {
        console.log("[useLiveKit] Already connecting, skipping join request");
        return;
      }

      if (livekitService.isConnected()) {
        console.log("[useLiveKit] Already connected, disconnecting first");
        await livekitService.disconnect();
        // Даем время на отключение
        await new Promise((resolve) => setTimeout(resolve, 500));
      }

      setIsConnecting(true);
      setError(null);

      console.log(`[useLiveKit] Joining channel ${channelId} on server ${serverId}`);

      // Get token from backend
      const response = await channelAPI.joinChannel({ channelId, serverId });

      console.log(`[useLiveKit] Connecting to LiveKit at ${response.url}`);

      // Connect to LiveKit
      await livekitService.connect(response.token, response.url);

      setIsConnecting(false);
    } catch (err) {
      console.error("[useLiveKit] Failed to join channel:", err);
      setError(err instanceof Error ? err.message : "Failed to join channel");
      setIsConnecting(false);
      throw err;
    }
  }, []);

  const leaveChannel = useCallback(async () => {
    try {
      console.log("[useLiveKit] Leaving channel...");
      await livekitService.disconnect();
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
  }, [isMuted, livekitService]);

  // Clean up on unmount
  useEffect(() => {
    return () => {
      if (isConnected) {
        livekitService.disconnect();
      }
    };
  }, [isConnected, livekitService]);

  return {
    participants,
    isConnected,
    isMuted,
    isConnecting,
    error,
    joinChannel,
    leaveChannel,
    toggleMute,
  };
}
