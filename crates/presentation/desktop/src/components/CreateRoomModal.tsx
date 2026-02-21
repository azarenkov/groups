import React, { useState } from "react";
import { RoomType } from "../types/room";
import "../styles/CreateRoomModal.css";

interface CreateRoomModalProps {
  isOpen: boolean;
  onClose: () => void;
  onCreate: (
    name: string,
    roomType: RoomType,
    password?: string,
  ) => Promise<void>;
}

const MAX_ROOM_NAME_LENGTH = 100;

const CreateRoomModal: React.FC<CreateRoomModalProps> = ({
  isOpen,
  onClose,
  onCreate,
}) => {
  const [name, setName] = useState("");
  const [roomType, setRoomType] = useState<RoomType>(RoomType.Public);
  const [password, setPassword] = useState("");
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();

    if (!name.trim()) {
      setError("Room name is required");
      return;
    }

    if (roomType === RoomType.Private && !password.trim()) {
      setError("Password is required for private rooms");
      return;
    }

    setIsLoading(true);
    setError(null);

    try {
      await onCreate(
        name.trim(),
        roomType,
        roomType === RoomType.Private ? password.trim() : undefined,
      );
      resetForm();
      onClose();
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to create room");
    } finally {
      setIsLoading(false);
    }
  };

  const resetForm = () => {
    setName("");
    setRoomType(RoomType.Public);
    setPassword("");
    setError(null);
  };

  const handleClose = () => {
    if (!isLoading) {
      resetForm();
      onClose();
    }
  };

  if (!isOpen) return null;

  return (
    <div className="modal-overlay" onClick={handleClose}>
      <div className="modal-content" onClick={(e) => e.stopPropagation()}>
        <div className="modal-header">
          <h2>Create Room</h2>
          <p className="modal-subtitle">
            Create a voice room where you can talk with friends.
          </p>
          <button
            className="modal-close-btn"
            onClick={handleClose}
            disabled={isLoading}
          >
            ×
          </button>
        </div>

        <form onSubmit={handleSubmit} className="modal-form">
          <div className="form-group">
            <label htmlFor="room-name">
              Room Name
              {name.length > 0 && (
                <span className="char-count">
                  {name.length} / {MAX_ROOM_NAME_LENGTH}
                </span>
              )}
            </label>
            <input
              id="room-name"
              type="text"
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder="Enter room name"
              disabled={isLoading}
              maxLength={MAX_ROOM_NAME_LENGTH}
              autoFocus
            />
          </div>

          <div className="form-group">
            <label htmlFor="room-type">Room Type</label>
            <div className="room-type-selector">
              <button
                type="button"
                className={`room-type-option ${roomType === RoomType.Public ? "active" : ""}`}
                onClick={() => setRoomType(RoomType.Public)}
                disabled={isLoading}
              >
                <div className="room-type-icon">
                  <svg width="24" height="24" viewBox="0 0 24 24">
                    <path
                      fill="currentColor"
                      d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm0 18c-4.41 0-8-3.59-8-8s3.59-8 8-8 8 3.59 8 8-3.59 8-8 8z"
                    />
                  </svg>
                </div>
                <div className="room-type-text">
                  <div className="room-type-title">Public</div>
                  <div className="room-type-description">Anyone can join</div>
                </div>
              </button>
              <button
                type="button"
                className={`room-type-option ${roomType === RoomType.Private ? "active" : ""}`}
                onClick={() => setRoomType(RoomType.Private)}
                disabled={isLoading}
              >
                <div className="room-type-icon">
                  <svg width="24" height="24" viewBox="0 0 24 24">
                    <path
                      fill="currentColor"
                      d="M18 8h-1V6c0-2.76-2.24-5-5-5S7 3.24 7 6v2H6c-1.1 0-2 .9-2 2v10c0 1.1.9 2 2 2h12c1.1 0 2-.9 2-2V10c0-1.1-.9-2-2-2zM9 6c0-1.66 1.34-3 3-3s3 1.34 3 3v2H9V6z"
                    />
                  </svg>
                </div>
                <div className="room-type-text">
                  <div className="room-type-title">Private</div>
                  <div className="room-type-description">Password required</div>
                </div>
              </button>
            </div>
          </div>

          {roomType === RoomType.Private && (
            <div className="form-group">
              <label htmlFor="room-password">Password</label>
              <input
                id="room-password"
                type="password"
                value={password}
                onChange={(e) => setPassword(e.target.value)}
                placeholder="Enter room password"
                disabled={isLoading}
                maxLength={50}
              />
            </div>
          )}

          {error && <div className="error-message">{error}</div>}

          <div className="modal-actions">
            <button
              type="button"
              className="btn-secondary"
              onClick={handleClose}
              disabled={isLoading}
            >
              Cancel
            </button>
            <button
              type="submit"
              className="btn-primary"
              disabled={isLoading || !name.trim()}
            >
              {isLoading ? "Creating..." : "Create Room"}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
};

export default CreateRoomModal;
