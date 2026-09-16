import React from "react";
import { useTranslation } from "react-i18next";
import Icon, { type IconName } from "../../../shared/ui/Icon";
import { serverAPI } from "../api/server";
import { useFriendshipStore } from "../../social/store/friendshipStore";
import { usePresenceStore } from "../../social/store/presenceStore";
import { ApiError } from "../../../shared/lib/apiError";
import { colorForId } from "../../../shared/lib/avatar";
import { ServerLogo } from "../../../shared/ui/Avatar";
import { useUserNamesStore } from "../../social/store/userNamesStore";
import { PERM_MANAGE_CHANNELS, PERM_MANAGE_INVITES, PERM_KICK_MEMBERS, PERM_MANAGE_SERVER, topRoleFor, useRolesStore, userPermissions } from "../store/rolesStore";
import type { Friendship } from "../../../shared/types/friendship";
import DangerSection from "./serverSettings/DangerSection";
import RolesSection from "./serverSettings/RolesSection";
import InvitesSection from "./serverSettings/InvitesSection";
import MembersSection from "./serverSettings/MembersSection";
import ChannelsSection from "./serverSettings/ChannelsSection";
import OverviewSection from "./serverSettings/OverviewSection";
import type { Server } from "../../../shared/types/server";
import type { Channel, ChannelKind, ChannelVisibility } from "../../../shared/types/channel";

type SectionId = "overview" | "channels" | "roles" | "invites" | "members" | "danger";

const SECTIONS: { id: SectionId; icon: IconName }[] = [
  { id: "overview", icon: "grid" },
  { id: "channels", icon: "hash" },
  { id: "roles", icon: "shield" },
  { id: "invites", icon: "user-plus" },
  { id: "members", icon: "users" },
];

interface ServerSettingsViewProps {
  server: Server;
  friends: Friendship[];
  channels: Channel[];
  currentUserId?: string;
  onClose: () => void;
  onDeleteServer: (server: Server) => void;
  onCreateChannel: (
    name: string,
    kind: ChannelKind,
    visibility: ChannelVisibility,
    password?: string,
  ) => Promise<void>;
  onDeleteChannel: (channel: Channel) => void;
}

const ServerSettingsView: React.FC<ServerSettingsViewProps> = ({
  server,
  friends,
  channels,
  currentUserId,
  onClose,
  onDeleteServer,
  onCreateChannel,
  onDeleteChannel,
}) => {
  const { t } = useTranslation(["serverSettings", "common", "modals"]);
  const [section, setSection] = React.useState<SectionId>("overview");
  React.useEffect(() => {
    if (section === "invites") void useFriendshipStore.getState().loadAll();
  }, [section]);
  const isOwner = currentUserId === server.ownerId;

  const rolesData = useRolesStore((st) => st.byServer[server.id]);
  const fetchRoles = useRolesStore((st) => st.fetch);
  React.useEffect(() => {
    void fetchRoles(server.id).catch(() => {});
  }, [server.id, fetchRoles]);
  const myPerms = currentUserId
    ? userPermissions(rolesData, currentUserId)
    : 0;
  const myTopPosition = isOwner
    ? Number.POSITIVE_INFINITY
    : currentUserId
      ? (topRoleFor(rolesData, currentUserId)?.position ?? Number.NEGATIVE_INFINITY)
      : Number.NEGATIVE_INFINITY;
  const canManageServer = isOwner || (myPerms & PERM_MANAGE_SERVER) !== 0;
  const canManageChannels = isOwner || (myPerms & PERM_MANAGE_CHANNELS) !== 0;
  const canManageInvites = isOwner || (myPerms & PERM_MANAGE_INVITES) !== 0;
  const canKickMembers = isOwner || (myPerms & PERM_KICK_MEMBERS) !== 0;
  const outranksMember = (memberId: string) => {
    if (isOwner) return true;
    if (memberId === server.ownerId) return false;
    const targetTop = topRoleFor(rolesData, memberId)?.position;
    return (
      myTopPosition !== Number.NEGATIVE_INFINITY &&
      (targetTop === undefined || targetTop < myTopPosition)
    );
  };
  const canKick = (memberId: string) =>
    canKickMembers && memberId !== currentUserId && outranksMember(memberId);
  const [kickError, setKickError] = React.useState<string | null>(null);
  const kickMember = async (memberId: string) => {
    setKickError(null);
    try {
      await serverAPI.kickMember(server.id, memberId);
      setMembers((prev) => prev.filter((m) => m.id !== memberId));
      void fetchRoles(server.id).catch(() => {});
    } catch (e) {
      setKickError(e instanceof ApiError ? e.message : t("common:error"));
    }
  };
  const visibleSections = SECTIONS.filter(
    (sec) => sec.id !== "invites" || canManageInvites,
  );
  React.useEffect(() => {
    if (canManageInvites) return;
    setSection((cur) => (cur === "invites" ? "overview" : cur));
  }, [canManageInvites]);

  const [members, setMembers] = React.useState<{ id: string; name: string }[]>([]);
  const [membersTick, setMembersTick] = React.useState(0);
  React.useEffect(() => {
    const onChanged = (e: Event) => {
      const detail = (e as CustomEvent<{ serverId?: string }>).detail;
      if (detail?.serverId === server.id) setMembersTick((n) => n + 1);
    };
    window.addEventListener("rift:server-members-changed", onChanged);
    return () =>
      window.removeEventListener("rift:server-members-changed", onChanged);
  }, [server.id]);
  const nameSyncTick = useUserNamesStore((s) => s.syncTick);
  const [invitedIds, setInvitedIds] = React.useState<Set<string>>(new Set());

  const online = usePresenceStore((s) => s.online);
  const selfOnline = usePresenceStore((s) => s.selfOnline);

  React.useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onClose]);

  React.useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const [allMembers, invitesRes] = await Promise.all([
          serverAPI.listAllMembers(server.id),
          serverAPI.listSentInvites(server.id),
        ]);
        if (cancelled) return;
        setMembers(allMembers);
        setInvitedIds(new Set(invitesRes.recipientIds));
      } catch {
        return;
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [server.id, nameSyncTick, membersTick]);

  const memberIds = React.useMemo(
    () => new Set(members.map((m) => m.id)),
    [members],
  );


  const accent = server.accentColor ?? colorForId(server.id);

  const onlineCount = members.filter((m) =>
    m.id === currentUserId ? selfOnline : !!online[m.id],
  ).length;



  return (
    <div className="settings-overlay">
      <nav className="settings-rail">
        <div className="ssv-identity">
          <ServerLogo
            serverId={server.id}
            name={server.name}
            className="ssv-identity-ava"
            style={{ background: accent }}
          />
          <div className="ssv-identity-info">
            <div className="ssv-identity-name">{server.name}</div>
            <div className="ssv-identity-sub">
              {t("memberCount", { count: members.length })} ·{" "}
              {t("onlineCount", { count: onlineCount })}
            </div>
          </div>
        </div>
        <div className="rail-section">{t("navTitle")}</div>
        {visibleSections.map((s) => (
          <button
            key={s.id}
            type="button"
            className={"rail-item" + (section === s.id ? " active" : "")}
            onClick={() => setSection(s.id)}
          >
            <span className="ic">
              <Icon name={s.icon} size={15} />
            </span>
            {t(`nav.${s.id}`)}
          </button>
        ))}
        <div className="ssv-rail-divider" />
        <button
          type="button"
          className={"rail-item danger" + (section === "danger" ? " active" : "")}
          onClick={() => setSection("danger")}
        >
          <span className="ic">
            <Icon name={isOwner ? "trash" : "logout"} size={15} />
          </span>
          {t("nav.danger")}
        </button>
        <div className="ssv-rail-footer">
          <button type="button" className="ssv-back-btn" onClick={onClose}>
            <Icon name="chev-left" size={14} />
            {t("backToServer")}
          </button>
          <div id="server-settings-voice-dock" className="settings-voice-dock" />
        </div>
      </nav>

      <div className="settings-pane">
        <button className="settings-close" onClick={onClose} title="Close (Esc)">
          <Icon name="x" size={16} />
          <span className="esc">ESC</span>
        </button>
        <div className="settings-content">
          {section === "overview" && (
            <OverviewSection
              server={server}
              canManageServer={canManageServer}
            />
          )}

          {section === "channels" && (
            <ChannelsSection
              server={server}
              channels={channels}
              canManageChannels={canManageChannels}
              onCreateChannel={onCreateChannel}
              onDeleteChannel={onDeleteChannel}
            />
          )}

          {section === "roles" && (
            <RolesSection
              server={server}
              members={members}
              isOwner={isOwner}
              myPerms={myPerms}
              myTopPosition={myTopPosition}
            />
          )}

          {section === "invites" && canManageInvites && (
            <InvitesSection
              server={server}
              friends={friends}
              memberIds={memberIds}
              invitedIds={invitedIds}
              setInvitedIds={setInvitedIds}
              onAlreadyMember={(memberId, name) =>
                setMembers((prev) =>
                  prev.some((m) => m.id === memberId)
                    ? prev
                    : [...prev, { id: memberId, name }],
                )
              }
              canManageInvites={canManageInvites}
            />
          )}

          {section === "members" && (
            <MembersSection
              server={server}
              members={members}
              currentUserId={currentUserId}
              rolesData={rolesData}
              canKick={canKick}
              kickMember={kickMember}
              kickError={kickError}
            />
          )}

          {section === "danger" && (
            <DangerSection
              server={server}
              isOwner={isOwner}
              onClose={onClose}
              onDeleteServer={onDeleteServer}
            />
          )}
        </div>
      </div>
    </div>
  );
};

export default ServerSettingsView;
