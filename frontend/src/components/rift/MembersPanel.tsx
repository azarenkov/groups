import React from "react";
import type { MockMember } from "../../data/mockData";

interface MembersPanelProps {
  members: MockMember[];
}

const ORDER: MockMember["status"][] = ["online", "idle", "dnd", "offline"];
const LABELS: Record<MockMember["status"], string> = {
  online: "Online",
  idle: "Idle",
  dnd: "Do not disturb",
  offline: "Offline",
};

const MembersPanel: React.FC<MembersPanelProps> = ({ members }) => {
  const groups = React.useMemo(() => {
    const buckets: Partial<Record<MockMember["status"], MockMember[]>> = {};
    members.forEach((m) => {
      (buckets[m.status] ||= []).push(m);
    });
    return ORDER.filter((k) => buckets[k] && buckets[k]!.length).map((k) => ({
      key: k,
      label: LABELS[k],
      items: buckets[k]!,
    }));
  }, [members]);

  return (
    <aside className="members-panel">
      {groups.map((g) => (
        <div key={g.key}>
          <div className="mp-section">
            {g.label}
            <span className="count">— {g.items.length}</span>
          </div>
          {g.items.map((m) => (
            <div
              key={m.id}
              className={"member-row" + (g.key === "offline" ? " offline" : "")}
            >
              <div
                className="ava"
                style={{ background: m.color, opacity: g.key === "offline" ? 0.55 : 1 }}
              >
                {m.initials}
                <span className={"presence " + g.key} />
              </div>
              <div className="info">
                <div className="name">{m.name}</div>
                <div className="role">{m.role}</div>
              </div>
            </div>
          ))}
        </div>
      ))}
    </aside>
  );
};

export default MembersPanel;
