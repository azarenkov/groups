import React, { useEffect, useState } from "react";
import { channelAPI } from "../../api/channel";
import type { ChannelParticipant } from "../../types/channel";
import { colorForId, initialFor } from "../../utils/avatar";
import { usePresenceStore } from "../../store/presenceStore";

interface MembersPanelProps {
  serverId: string;
  channelId: string;
}

const MembersPanel: React.FC<MembersPanelProps> = ({ serverId, channelId }) => {
  const [members, setMembers] = useState<ChannelParticipant[]>([]);
  const onlineUsers = usePresenceStore((s) => s.online);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    setError(null);

    const PER_PAGE = 50;
    const fetchAll = async (): Promise<ChannelParticipant[]> => {
      const acc: ChannelParticipant[] = [];
      let page = 1;
      while (!cancelled) {
        const res = await channelAPI.listParticipants(
          serverId,
          channelId,
          page,
          PER_PAGE,
        );
        acc.push(...res.participants);
        const totalPages = res.pagination?.totalPages ?? 1;
        if (page >= totalPages || res.participants.length < PER_PAGE) break;
        page += 1;
      }
      return acc;
    };

    fetchAll()
      .then((all) => {
        if (!cancelled) setMembers(all);
      })
      .catch((err) => {
        if (!cancelled) {
          setError(err instanceof Error ? err.message : "Failed to load members");
          setMembers([]);
        }
      })
      .finally(() => {
        if (!cancelled) setLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, [serverId, channelId]);

  return (
    <aside className="members-panel">
      <div className="mp-section">
        Members
        <span className="count">— {members.length}</span>
      </div>
      {loading && members.length === 0 && (
        <div className="mp-empty">Loading…</div>
      )}
      {error && <div className="mp-empty">{error}</div>}
      {!loading && !error && members.length === 0 && (
        <div className="mp-empty">No one has joined yet.</div>
      )}
      {members.map((m) => {
        const isOnline = !!onlineUsers[m.id];
        return (
          <div key={m.id} className="member-row">
            <div
              className="ava"
              style={{ background: colorForId(m.id), position: "relative" }}
            >
              {initialFor(m.name)}
              <span
                className={"presence " + (isOnline ? "online" : "offline")}
                title={isOnline ? "Online" : "Offline"}
                aria-label={isOnline ? "Online" : "Offline"}
              />
            </div>
            <div className="info">
              <div className="name">{m.name}</div>
              <div className="role">{isOnline ? "Online" : "Offline"}</div>
            </div>
          </div>
        );
      })}
    </aside>
  );
};

export default MembersPanel;
