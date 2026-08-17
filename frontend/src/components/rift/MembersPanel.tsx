import React, { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { channelAPI } from "../../api/channel";
import type { ChannelParticipant } from "../../types/channel";
import { UserAvatar } from "./Avatar";
import { usePresenceStore } from "../../store/presenceStore";
import { useUserNamesStore } from "../../store/userNamesStore";

interface MembersPanelProps {
  serverId: string;
  channelId: string;
}

const MembersPanel: React.FC<MembersPanelProps> = ({ serverId, channelId }) => {
  const { t } = useTranslation(["members", "common"]);
  const [members, setMembers] = useState<ChannelParticipant[]>([]);
  const onlineUsers = usePresenceStore((s) => s.online);
  const nameOverrides = useUserNamesStore((s) => s.overrides);
  const selfUserId = usePresenceStore((s) => s.selfUserId);
  const selfOnline = usePresenceStore((s) => s.selfOnline);
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

  const orderedMembers = selfUserId
    ? [...members].sort((a, b) => {
        if (a.id === selfUserId) return -1;
        if (b.id === selfUserId) return 1;
        return 0;
      })
    : members;

  return (
    <aside className="members-panel">
      <div className="mp-section">
        {t("title")}
        <span className="count">— {members.length}</span>
      </div>
      {loading && members.length === 0 && (
        <div className="mp-empty">{t("common:loading")}</div>
      )}
      {error && <div className="mp-empty">{error}</div>}
      {!loading && !error && members.length === 0 && (
        <div className="mp-empty">{t("noMembers")}</div>
      )}
      {orderedMembers.map((m) => {
        const isSelf = m.id === selfUserId;
        const isOnline = isSelf ? selfOnline : !!onlineUsers[m.id];
        const displayName = nameOverrides[m.id] ?? m.name;
        const statusLabel = isOnline
          ? t("common:online")
          : t("common:offline");
        return (
          <div key={m.id} className="member-row">
            <UserAvatar userId={m.id} name={displayName} className="ava">
              <span
                className={"presence " + (isOnline ? "online" : "offline")}
                title={statusLabel}
                aria-label={statusLabel}
              />
            </UserAvatar>
            <div className="info">
              <div className="name">
                {displayName}
                {isSelf && <span className="member-you">{t("you")}</span>}
              </div>
              <div className="role">{statusLabel}</div>
            </div>
          </div>
        );
      })}
    </aside>
  );
};

export default MembersPanel;
