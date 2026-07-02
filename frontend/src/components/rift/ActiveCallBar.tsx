import React, { useEffect, useMemo, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { useTranslation } from "react-i18next";
import Icon from "./Icon";
import { useCallStore } from "../../store/callStore";
import { useFriendshipStore } from "../../store/friendshipStore";
import { getLiveKitService } from "../../services/livekit";
import { colorForId, initialFor } from "../../utils/avatar";

const ActiveCallBar: React.FC = () => {
  const activeCallId = useCallStore((s) => s.activeCallId);
  const leave = useCallStore((s) => s.leave);
  const addParticipants = useCallStore((s) => s.addParticipants);
  const participants = useCallStore((s) => s.participants);
  const friends = useFriendshipStore((s) => s.friends);
  const [muted, setMuted] = useState(false);
  const [addOpen, setAddOpen] = useState(false);
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [mountPoint, setMountPoint] = useState<HTMLElement | null>(null);
  const rootRef = useRef<HTMLDivElement>(null);
  const { t } = useTranslation(["call", "voice"]);

  useEffect(() => {
    setMountPoint(document.getElementById("voice-dock-slot"));
  }, []);

  const inCallIds = useMemo(
    () => new Set(Object.keys(participants)),
    [participants],
  );
  const invitable = useMemo(
    () =>
      friends.filter(
        (f) => f.status === "accepted" && !inCallIds.has(f.friendId),
      ),
    [friends, inCallIds],
  );

  useEffect(() => {
    if (!addOpen) return;
    const onDoc = (e: MouseEvent) => {
      if (rootRef.current && !rootRef.current.contains(e.target as Node)) {
        setAddOpen(false);
      }
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") setAddOpen(false);
    };
    document.addEventListener("mousedown", onDoc);
    document.addEventListener("keydown", onKey);
    return () => {
      document.removeEventListener("mousedown", onDoc);
      document.removeEventListener("keydown", onKey);
    };
  }, [addOpen]);

  if (!activeCallId || !mountPoint) return null;

  const toggleMute = () => {
    const next = !muted;
    getLiveKitService().setMicrophoneMuted(next);
    setMuted(next);
  };

  const toggleSelected = (id: string) =>
    setSelected((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });

  const confirmAdd = async () => {
    const ids = Array.from(selected);
    setAddOpen(false);
    setSelected(new Set());
    if (ids.length > 0) {
      await addParticipants(ids).catch((e) =>
        console.warn("[call] add participants failed:", e),
      );
    }
  };

  const count = Math.max(inCallIds.size, 1);

  return createPortal(
    <div className="voice-pill in-dock connected" ref={rootRef}>
      <div className="vp-meta-wrap">
        <div className="vp-title">{t("inCall")}</div>
        <div className="vp-meta">
          {count} {t("voice:connected").toLowerCase()}
        </div>
      </div>
      <div className="vp-actions">
        <button
          type="button"
          className={"vp-btn" + (addOpen ? " on" : "")}
          title={t("addParticipants")}
          onClick={() => setAddOpen((v) => !v)}
        >
          <Icon name="user-plus" size={15} />
        </button>
        <button
          type="button"
          className={"vp-btn" + (muted ? " muted" : "")}
          title={muted ? t("unmute") : t("mute")}
          onClick={toggleMute}
        >
          <Icon name={muted ? "mic-off" : "mic"} size={15} />
        </button>
      </div>
      <button
        type="button"
        className="vp-btn leave"
        title={t("leave")}
        onClick={() => leave()}
      >
        <Icon name="logout" size={15} />
      </button>

      {addOpen && (
        <div className="acb-add" onMouseDown={(e) => e.stopPropagation()}>
          <div className="acb-add-title">{t("addParticipants")}</div>
          <div className="acb-add-list">
            {invitable.length === 0 && (
              <div className="acb-add-empty">{t("noFriendsToAdd")}</div>
            )}
            {invitable.map((f) => (
              <button
                key={f.friendId}
                type="button"
                className={
                  "acb-add-item" + (selected.has(f.friendId) ? " on" : "")
                }
                onClick={() => toggleSelected(f.friendId)}
              >
                <span
                  className="acb-add-ava"
                  style={{ background: colorForId(f.friendId) }}
                >
                  {initialFor(f.friendName)}
                </span>
                <span className="acb-add-name">{f.friendName}</span>
                {selected.has(f.friendId) && <Icon name="check" size={14} />}
              </button>
            ))}
          </div>
          <div className="acb-add-foot">
            <button
              type="button"
              className="btn"
              onClick={() => setAddOpen(false)}
            >
              {t("cancel")}
            </button>
            <button
              type="button"
              className="btn primary"
              disabled={selected.size === 0}
              onClick={confirmAdd}
            >
              {t("add")}
            </button>
          </div>
        </div>
      )}
    </div>,
    mountPoint,
  );
};

export default ActiveCallBar;
