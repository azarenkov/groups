import React, { useEffect, useRef } from "react";
import { useLiveKit } from "../hooks/useLiveKit";
import "../styles/VoiceRoom.css";

interface VoiceRoomProps {
  roomId: string;
  serverId: string;
  roomName: string;
  onClose: () => void;
}

const VoiceRoom: React.FC<VoiceRoomProps> = ({
  roomId,
  serverId,
  roomName,
  onClose,
}) => {
  const {
    participants,
    isConnected,
    isMuted,
    isConnecting,
    error,
    joinRoom,
    leaveRoom,
    toggleMute,
  } = useLiveKit();

  // Track if we've already joined to prevent duplicate joins
  const hasJoinedRef = useRef(false);
  const currentRoomRef = useRef<string | null>(null);
  const isJoiningRef = useRef(false);

  useEffect(() => {
    // Only join if we haven't joined this room yet and not currently joining
    const shouldJoin =
      (!hasJoinedRef.current || currentRoomRef.current !== roomId) &&
      !isJoiningRef.current;

    if (shouldJoin) {
      hasJoinedRef.current = true;
      currentRoomRef.current = roomId;
      isJoiningRef.current = true;

      console.log(`[VoiceRoom] Joining room ${roomId} on server ${serverId}`);

      joinRoom(roomId, serverId)
        .then(() => {
          console.log("[VoiceRoom] Successfully joined room");
          isJoiningRef.current = false;
        })
        .catch((err) => {
          console.error("[VoiceRoom] Failed to join room:", err);
          hasJoinedRef.current = false;
          isJoiningRef.current = false;
        });
    } else {
      console.log("[VoiceRoom] Skipping duplicate join attempt", {
        hasJoined: hasJoinedRef.current,
        currentRoom: currentRoomRef.current,
        requestedRoom: roomId,
        isJoining: isJoiningRef.current,
      });
    }

    // Leave room on unmount
    return () => {
      console.log("[VoiceRoom] Cleanup - leaving room");
      hasJoinedRef.current = false;
      currentRoomRef.current = null;
      isJoiningRef.current = false;
      leaveRoom();
    };
  }, [roomId, serverId]);

  const handleDisconnect = async () => {
    await leaveRoom();
    onClose();
  };

  return (
    <div className="voice-room">
      <div className="voice-room-header">
        <div className="voice-room-title">
          <svg width="20" height="20" viewBox="0 0 24 24">
            <path
              fill="currentColor"
              d="M12 3a9 9 0 0 0-9 9v7a2 2 0 0 0 2 2h2v-5H5v-4a7 7 0 1 1 14 0v4h-2v5h2a2 2 0 0 0 2-2v-7a9 9 0 0 0-9-9z"
            />
          </svg>
          <span>Voice - {roomName}</span>
        </div>
        <button
          className="voice-room-close"
          onClick={handleDisconnect}
          title="Disconnect"
        >
          <svg width="20" height="20" viewBox="0 0 24 24">
            <path
              fill="currentColor"
              d="M19 6.41L17.59 5 12 10.59 6.41 5 5 6.41 10.59 12 5 17.59 6.41 19 12 13.41 17.59 19 19 17.59 13.41 12z"
            />
          </svg>
        </button>
      </div>

      <div className="voice-room-content">
        {isConnecting && (
          <div className="voice-room-status">
            <div className="spinner"></div>
            <span>Connecting to voice...</span>
          </div>
        )}

        {error && (
          <div className="voice-room-error">
            <svg width="24" height="24" viewBox="0 0 24 24">
              <path
                fill="currentColor"
                d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm1 15h-2v-2h2v2zm0-4h-2V7h2v6z"
              />
            </svg>
            <span>{error}</span>
          </div>
        )}

        {isConnected && (
          <div className="voice-room-participants">
            <div className="participants-header">
              <span>Participants — {participants.length + 1}</span>
            </div>

            {/* Local participant (you) */}
            <div className="participant you">
              <div className="participant-avatar">
                <svg width="20" height="20" viewBox="0 0 24 24">
                  <path
                    fill="currentColor"
                    d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm0 3c1.66 0 3 1.34 3 3s-1.34 3-3 3-3-1.34-3-3 1.34-3 3-3zm0 14.2c-2.5 0-4.71-1.28-6-3.22.03-1.99 4-3.08 6-3.08 1.99 0 5.97 1.09 6 3.08-1.29 1.94-3.5 3.22-6 3.22z"
                  />
                </svg>
              </div>
              <div className="participant-info">
                <span className="participant-name">You</span>
                {isMuted && <span className="muted-badge">Muted</span>}
              </div>
              <div className={`speaking-indicator ${!isMuted ? "active" : ""}`}>
                <div className="wave"></div>
                <div className="wave"></div>
                <div className="wave"></div>
              </div>
            </div>

            {/* Remote participants */}
            {participants.map((participant) => (
              <div key={participant.identity} className="participant">
                <div className="participant-avatar">
                  <svg width="20" height="20" viewBox="0 0 24 24">
                    <path
                      fill="currentColor"
                      d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm0 3c1.66 0 3 1.34 3 3s-1.34 3-3 3-3-1.34-3-3 1.34-3 3-3zm0 14.2c-2.5 0-4.71-1.28-6-3.22.03-1.99 4-3.08 6-3.08 1.99 0 5.97 1.09 6 3.08-1.29 1.94-3.5 3.22-6 3.22z"
                    />
                  </svg>
                </div>
                <div className="participant-info">
                  <span className="participant-name">{participant.name}</span>
                </div>
                <div
                  className={`speaking-indicator ${participant.isSpeaking ? "active" : ""}`}
                >
                  <div className="wave"></div>
                  <div className="wave"></div>
                  <div className="wave"></div>
                </div>
              </div>
            ))}
          </div>
        )}
      </div>

      {isConnected && (
        <div className="voice-room-controls">
          <button
            className={`control-btn ${isMuted ? "muted" : ""}`}
            onClick={toggleMute}
            title={isMuted ? "Unmute" : "Mute"}
          >
            {isMuted ? (
              <svg width="20" height="20" viewBox="0 0 24 24">
                <path
                  fill="currentColor"
                  d="M19 11h-1.7c0 .74-.16 1.43-.43 2.05l1.23 1.23c.56-.98.9-2.09.9-3.28zm-4.02.17c0-.06.02-.11.02-.17V5c0-1.66-1.34-3-3-3S9 3.34 9 5v.18l5.98 5.99zM4.27 3L3 4.27l6.01 6.01V11c0 1.66 1.33 3 2.99 3 .22 0 .44-.03.65-.08l1.66 1.66c-.71.33-1.5.52-2.31.52-2.76 0-5.3-2.1-5.3-5.1H5c0 3.41 2.72 6.23 6 6.72V21h2v-3.28c.91-.13 1.77-.45 2.54-.9L19.73 21 21 19.73 4.27 3z"
                />
              </svg>
            ) : (
              <svg width="20" height="20" viewBox="0 0 24 24">
                <path
                  fill="currentColor"
                  d="M12 14c1.66 0 3-1.34 3-3V5c0-1.66-1.34-3-3-3S9 3.34 9 5v6c0 1.66 1.34 3 3 3z"
                />
                <path
                  fill="currentColor"
                  d="M17 11c0 2.76-2.24 5-5 5s-5-2.24-5-5H5c0 3.53 2.61 6.43 6 6.92V21h2v-3.08c3.39-.49 6-3.39 6-6.92h-2z"
                />
              </svg>
            )}
          </button>

          <button
            className="control-btn disconnect"
            onClick={handleDisconnect}
            title="Disconnect"
          >
            <svg width="20" height="20" viewBox="0 0 24 24">
              <path
                fill="currentColor"
                d="M12 9c-1.6 0-3.15.25-4.6.72v3.1c0 .39-.23.74-.56.9-.98.49-1.87 1.12-2.66 1.85-.18.18-.43.28-.7.28-.28 0-.53-.11-.71-.29L.29 13.08c-.18-.17-.29-.42-.29-.7 0-.28.11-.53.29-.71C3.34 8.78 7.46 7 12 7s8.66 1.78 11.71 4.67c.18.18.29.43.29.71 0 .28-.11.53-.29.71l-2.48 2.48c-.18.18-.43.29-.71.29-.27 0-.52-.11-.7-.28-.79-.74-1.68-1.36-2.66-1.85-.33-.16-.56-.5-.56-.9v-3.1C15.15 9.25 13.6 9 12 9z"
              />
            </svg>
          </button>
        </div>
      )}
    </div>
  );
};

export default VoiceRoom;
