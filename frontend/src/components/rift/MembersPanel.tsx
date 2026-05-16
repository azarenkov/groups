import React, { useEffect, useState } from "react";
import { channelAPI } from "../../api/channel";
import type { ChannelParticipant } from "../../types/channel";

interface MembersPanelProps {
  serverId: string;
  channelId: string;
}

const PALETTE = [
  "#0071e3",
  "#ff6b35",
  "#30a46c",
  "#bf5af2",
  "#ff9f0a",
  "#5e6ad2",
];

function colorFor(id: string): string {
  let h = 0;
  for (let i = 0; i < id.length; i++) h = (h * 31 + id.charCodeAt(i)) >>> 0;
  return PALETTE[h % PALETTE.length];
}

function initialsFor(name: string): string {
  return name ? name.charAt(0).toUpperCase() : "?";
}

const MembersPanel: React.FC<MembersPanelProps> = ({ serverId, channelId }) => {
  const [members, setMembers] = useState<ChannelParticipant[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    setError(null);
    channelAPI
      .listParticipants(serverId, channelId)
      .then((res) => {
        if (!cancelled) setMembers(res.participants);
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
      {members.map((m) => (
        <div key={m.id} className="member-row">
          <div className="ava" style={{ background: colorFor(m.id) }}>
            {initialsFor(m.name)}
          </div>
          <div className="info">
            <div className="name">{m.name}</div>
          </div>
        </div>
      ))}
    </aside>
  );
};

export default MembersPanel;
