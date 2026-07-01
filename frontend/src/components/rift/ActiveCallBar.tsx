import React, { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import Icon from "./Icon";
import { useCallStore } from "../../store/callStore";
import { useFriendshipStore } from "../../store/friendshipStore";
import { useParticipants } from "../../hooks/useParticipants";
import { getLiveKitService } from "../../services/livekit";
import { colorForId, initialFor } from "../../utils/avatar";

const ActiveCallBar: React.FC = () => {
  const activeCallId = useCallStore((s) => s.activeCallId);
  const leave = useCallStore((s) => s.leave);
  const addParticipants = useCallStore((s) => s.addParticipants);
  const friends = useFriendshipStore((s) => s.friends);
  const participants = useParticipants();
  const [muted, setMuted] = useState(false);
  const [addOpen, setAddOpen] = useState(false);
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const { t } = useTranslation("call");

  const inCallIds = useMemo(
    () => new Set(participants.map((p) => p.identity)),
    [participants],
  );
  const invitable = useMemo(
    () =>
      friends.filter(
        (f) => f.status === "accepted" && !inCallIds.has(f.friendId),
      ),
    [friends, inCallIds],
  );

  if (!activeCallId) return null;

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

  return (
    <div className="active-call-bar">
      <div className="acb-info">
        <span className="acb-dot" />
        <span className="acb-label">{t("inCall")}</span>
        <span className="acb-count">{participants.length + 1}</span>
      </div>
      <div className="acb-actions">
        <button
          type="button"
          className={"acb-btn" + (addOpen ? " on" : "")}
          title={t("addParticipants")}
          onClick={() => setAddOpen((v) => !v)}
        >
          <Icon name="user-plus" size={16} />
        </button>
        <button
          type="button"
          className={"acb-btn" + (muted ? " muted" : "")}
          title={muted ? t("unmute") : t("mute")}
          onClick={toggleMute}
        >
          <Icon name={muted ? "mic-off" : "mic"} size={16} />
        </button>
        <button
          type="button"
          className="acb-btn leave"
          title={t("leave")}
          onClick={() => leave()}
        >
          <Icon name="phone" size={16} />
        </button>
      </div>

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
    </div>
  );
};

export default ActiveCallBar;
