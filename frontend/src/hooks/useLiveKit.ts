import { useState, useEffect, useCallback } from "react";
import { getLiveKitService, ParticipantInfo } from "../services/livekit";
import { roomAPI } from "../api/room";

interface UseLiveKitResult {
  participants: ParticipantInfo[];
  isConnected: boolean;
  isMuted: boolean;
  isConnecting: boolean;
  error: string | null;
  joinRoom: (roomId: string, serverId: string) => Promise<void>;
  leaveRoom: () => Promise<void>;
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

  const joinRoom = useCallback(async (roomId: string, serverId: string) => {
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

      console.log(`[useLiveKit] Joining room ${roomId} on server ${serverId}`);

      // Get token from backend
      const response = await roomAPI.joinRoom({ roomId, serverId });

      console.log(`[useLiveKit] Connecting to LiveKit at ${response.url}`);

      // Connect to LiveKit
      await livekitService.connect(response.token, response.url);

      setIsConnecting(false);
    } catch (err) {
      console.error("[useLiveKit] Failed to join room:", err);
      setError(err instanceof Error ? err.message : "Failed to join room");
      setIsConnecting(false);
      throw err;
    }
  }, []);

  const leaveRoom = useCallback(async () => {
    try {
      console.log("[useLiveKit] Leaving room...");
      await livekitService.disconnect();
      setParticipants([]);
      setIsConnected(false);
      setIsMuted(false);
      setError(null);
    } catch (err) {
      console.error("[useLiveKit] Failed to leave room:", err);
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
    joinRoom,
    leaveRoom,
    toggleMute,
  };
}
