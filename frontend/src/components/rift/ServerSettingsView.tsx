import React from "react";
import { useTranslation } from "react-i18next";
import Icon, { type IconName } from "./Icon";
import { serverAPI } from "../../api/server";
import { dmAPI } from "../../api/dm";
import { useServerStore } from "../../store/serverStore";
import { useFriendshipStore } from "../../store/friendshipStore";
import { usePresenceStore } from "../../store/presenceStore";
import { ApiError } from "../../utils/apiError";
import { colorForId } from "../../utils/avatar";
import { ServerLogo, UserAvatar } from "./Avatar";
import { bustAvatar, useAvatarUrl } from "../../utils/avatarCache";
import { useUserNamesStore } from "../../store/userNamesStore";
import {
  PERM_MANAGE_CHANNELS,
  PERM_MANAGE_INVITES,
  PERM_MANAGE_MESSAGES,
  PERM_MANAGE_ROLES,
  PERM_KICK_MEMBERS,
  PERM_MANAGE_SERVER,
  PERM_DEAFEN_MEMBERS,
  PERM_MUTE_MEMBERS,
  topRoleFor,
  useRolesStore,
  userPermissions,
} from "../../store/rolesStore";
import type { Friendship } from "../../types/friendship";
import type { InviteLink, Server } from "../../types/server";
import type { Channel, ChannelKind, ChannelVisibility } from "../../types/channel";

type SectionId = "overview" | "channels" | "roles" | "invites" | "members" | "danger";

const EXPIRY_OPTIONS = [0, 1, 7, 30] as const;
const MAX_USES_OPTIONS = [0, 10, 25, 50, 100] as const;

const daysLeft = (expiresAt: string | null): number | null => {
  if (!expiresAt) return null;
  const ms = new Date(expiresAt).getTime() - Date.now();
  return Math.max(1, Math.ceil(ms / 86_400_000));
};

const copyText = async (text: string): Promise<boolean> => {
  try {
    if (navigator.clipboard && window.isSecureContext) {
      await navigator.clipboard.writeText(text);
      return true;
    }
  } catch {
    void 0;
  }
  try {
    const ta = document.createElement("textarea");
    ta.value = text;
    ta.style.position = "fixed";
    ta.style.opacity = "0";
    document.body.appendChild(ta);
    ta.focus();
    ta.select();
    const ok = document.execCommand("copy");
    document.body.removeChild(ta);
    return ok;
  } catch {
    return false;
  }
};

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
  const canManageRoles = isOwner || (myPerms & PERM_MANAGE_ROLES) !== 0;
  const myTopPosition = isOwner
    ? Number.POSITIVE_INFINITY
    : currentUserId
      ? (topRoleFor(rolesData, currentUserId)?.position ?? Number.NEGATIVE_INFINITY)
      : Number.NEGATIVE_INFINITY;
  const canTouchRole = (position: number) =>
    canManageRoles && position < myTopPosition;
  const grantableBits = isOwner ? ~0 : myPerms;
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
  const outranksAllHolders = (roleId: string) => {
    if (isOwner) return true;
    const holders = Object.entries(rolesData?.memberRoles ?? {})
      .filter(([, roleIds]) => roleIds.includes(roleId))
      .map(([id]) => id);
    return holders.every((id) => outranksMember(id));
  };
  const canKick = (memberId: string) =>
    canKickMembers && memberId !== currentUserId && outranksMember(memberId);
  const kickMember = async (memberId: string) => {
    try {
      await serverAPI.kickMember(server.id, memberId);
      setMembers((prev) => prev.filter((m) => m.id !== memberId));
      refreshRoles();
    } catch (e) {
      setRoleError(e instanceof ApiError ? e.message : t("common:error"));
    }
  };
  const visibleSections = SECTIONS.filter(
    (sec) => sec.id !== "invites" || canManageInvites,
  );
  React.useEffect(() => {
    if (canManageInvites) return;
    linksGen.current += 1;
    setLinks([]);
    setPrimaryLink(null);
    setLinksCursor(null);
    setSection((cur) => (cur === "invites" ? "overview" : cur));
  }, [canManageInvites]);

  const ROLE_COLORS = [
    "#5865f2", "#3ba55c", "#faa61a", "#ed4245", "#eb459e", "#9b59b6", "#00b0f4", "#8e8e93",
  ];
  const PERMS: { bit: number; key: string }[] = [
    { bit: PERM_MANAGE_CHANNELS, key: "permManageChannels" },
    { bit: PERM_MANAGE_INVITES, key: "permManageInvites" },
    { bit: PERM_MANAGE_MESSAGES, key: "permManageMessages" },
    { bit: PERM_MUTE_MEMBERS, key: "permMuteMembers" },
    { bit: PERM_DEAFEN_MEMBERS, key: "permDeafenMembers" },
    { bit: PERM_MANAGE_ROLES, key: "permManageRoles" },
    { bit: PERM_MANAGE_SERVER, key: "permManageServer" },
    { bit: PERM_KICK_MEMBERS, key: "permKickMembers" },
  ];
  const [newRoleName, setNewRoleName] = React.useState("");
  const [newRoleColor, setNewRoleColor] = React.useState(ROLE_COLORS[0]);
  const [newRolePerms, setNewRolePerms] = React.useState(0);
  const [roleBusy, setRoleBusy] = React.useState(false);
  const [roleError, setRoleError] = React.useState<string | null>(null);
  const [expandedRoleId, setExpandedRoleId] = React.useState<string | null>(null);
  const [rolePermsDraft, setRolePermsDraft] = React.useState<Record<string, number>>({});

  const refreshRoles = () => void fetchRoles(server.id).catch(() => {});

  const submitCreateRole = async () => {
    const name = newRoleName.trim();
    if (!name) {
      setRoleError(t("roleNameRequired"));
      return;
    }
    setRoleBusy(true);
    setRoleError(null);
    try {
      await serverAPI.createRole(server.id, name, newRoleColor, newRolePerms);
      setNewRoleName("");
      setNewRolePerms(0);
      refreshRoles();
    } catch (e) {
      setRoleError(e instanceof ApiError ? e.message : t("common:error"));
    } finally {
      setRoleBusy(false);
    }
  };


  type RoleDraft = { name?: string; color?: string; add: number; remove: number };
  const roleWriteChainsRef = React.useRef<Record<string, Promise<void>>>({});
  const roleLatestRef = React.useRef<Record<string, RoleDraft>>({});

  const enqueueRoleEdit = (
    role: { id: string; name: string; color: string; permissions: number },
    patch: { name?: string; color?: string; add?: number; remove?: number },
  ) => {
    const base = roleLatestRef.current[role.id] ?? { add: 0, remove: 0 };
    const next: RoleDraft = {
      name: patch.name ?? base.name,
      color: patch.color ?? base.color,
      add: (base.add | (patch.add ?? 0)) & ~(patch.remove ?? 0),
      remove: (base.remove | (patch.remove ?? 0)) & ~(patch.add ?? 0),
    };
    roleLatestRef.current[role.id] = next;
    setRolePermsDraft((prev) => ({
      ...prev,
      [role.id]: (role.permissions | next.add) & ~next.remove,
    }));
    const chain = roleWriteChainsRef.current[role.id] ?? Promise.resolve();
    roleWriteChainsRef.current[role.id] = chain.then(async () => {
      const target = roleLatestRef.current[role.id];
      if (!target) return;
      const empty =
        target.name === undefined &&
        target.color === undefined &&
        target.add === 0 &&
        target.remove === 0;
      if (empty) {
        delete roleLatestRef.current[role.id];
        return;
      }
      delete roleLatestRef.current[role.id];
      try {
        await serverAPI.updateRole(server.id, role.id, {
          name: target.name,
          color: target.color,
          permissionsAdd: target.add || undefined,
          permissionsRemove: target.remove || undefined,
        });
        if (!roleLatestRef.current[role.id]) {
          setRolePermsDraft((cur) => {
            const nextDraft = { ...cur };
            delete nextDraft[role.id];
            return nextDraft;
          });
        }
        refreshRoles();
      } catch (e) {
        setRoleError(e instanceof ApiError ? e.message : t("common:error"));
        if (!roleLatestRef.current[role.id]) {
          setRolePermsDraft((cur) => {
            const nextDraft = { ...cur };
            delete nextDraft[role.id];
            return nextDraft;
          });
        }
        refreshRoles();
      }
    });
  };

  const toggleRolePerm = (
    role: { id: string; name: string; color: string; permissions: number },
    bit: number,
    checked: boolean,
  ) => {
    enqueueRoleEdit(role, checked ? { add: bit } : { remove: bit });
  };

  const swapRoles = async (
    a: { id: string; position: number },
    b: { id: string; position: number },
  ) => {
    try {
      await serverAPI.swapRoles(server.id, a.id, b.id);
      refreshRoles();
    } catch (e) {
      setRoleError(e instanceof ApiError ? e.message : t("common:error"));
    }
  };

  const removeRole = async (roleId: string) => {
    try {
      await serverAPI.deleteRole(server.id, roleId);
      if (expandedRoleId === roleId) setExpandedRoleId(null);
      refreshRoles();
    } catch (e) {
      setRoleError(e instanceof ApiError ? e.message : t("common:error"));
    }
  };

  const toggleAssignment = async (roleId: string, memberId: string, has: boolean) => {
    try {
      if (has) await serverAPI.revokeRole(server.id, memberId, roleId);
      else await serverAPI.assignRole(server.id, memberId, roleId);
      refreshRoles();
    } catch (e) {
      setRoleError(e instanceof ApiError ? e.message : t("common:error"));
    }
  };

  const [newChName, setNewChName] = React.useState("");
  const [newChKind, setNewChKind] = React.useState<ChannelKind>("text");
  const [newChVisibility, setNewChVisibility] =
    React.useState<ChannelVisibility>("public");
  const [newChPassword, setNewChPassword] = React.useState("");
  const [newChBusy, setNewChBusy] = React.useState(false);
  const [newChError, setNewChError] = React.useState<string | null>(null);

  const submitNewChannel = async () => {
    const name = newChName.trim();
    if (!name) {
      setNewChError(t("channelNameRequired"));
      return;
    }
    if (newChVisibility === "private" && !newChPassword.trim()) {
      setNewChError(t("channelPasswordRequired"));
      return;
    }
    setNewChBusy(true);
    setNewChError(null);
    try {
      await onCreateChannel(
        name,
        newChKind,
        newChVisibility,
        newChVisibility === "private" ? newChPassword : undefined,
      );
      setNewChName("");
      setNewChPassword("");
      setNewChVisibility("public");
    } catch (e) {
      setNewChError(e instanceof ApiError ? e.message : t("common:error"));
    } finally {
      setNewChBusy(false);
    }
  };

  const [name, setName] = React.useState(server.name);
  const [description, setDescription] = React.useState(server.description ?? "");
  const [saving, setSaving] = React.useState(false);
  const [saveError, setSaveError] = React.useState<string | null>(null);
  const [saved, setSaved] = React.useState(false);
  const patchServer = useServerStore((s) => s.patchServer);

  const logoInputRef = React.useRef<HTMLInputElement>(null);
  const [logoBusy, setLogoBusy] = React.useState(false);
  const logoUrl = useAvatarUrl("server", server.id);

  const handleLogoFile = async (file: File) => {
    setLogoBusy(true);
    setSaveError(null);
    try {
      await serverAPI.uploadLogo(server.id, file);
      bustAvatar("server", server.id);
    } catch (err) {
      const apiErr = err instanceof ApiError ? err : null;
      if (apiErr?.code === "IMAGE_CONTENT_REJECTED") {
        setSaveError(t("logoRejected"));
      } else {
        setSaveError(apiErr?.message || t("logoError"));
      }
    } finally {
      setLogoBusy(false);
    }
  };

  const handleLogoRemove = async () => {
    setLogoBusy(true);
    setSaveError(null);
    try {
      await serverAPI.deleteLogo(server.id);
      bustAvatar("server", server.id);
    } catch (err) {
      const apiErr = err instanceof ApiError ? err : null;
      setSaveError(apiErr?.message || t("logoError"));
    } finally {
      setLogoBusy(false);
    }
  };

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
  const nameOverrides = useUserNamesStore((s) => s.overrides);
  const nameSyncTick = useUserNamesStore((s) => s.syncTick);
  const [invitedIds, setInvitedIds] = React.useState<Set<string>>(new Set());
  const [inviteQuery, setInviteQuery] = React.useState("");
  const [invitePending, setInvitePending] = React.useState<string | null>(null);
  const [inviteError, setInviteError] = React.useState<string | null>(null);
  const [memberQuery, setMemberQuery] = React.useState("");

  const online = usePresenceStore((s) => s.online);
  const selfOnline = usePresenceStore((s) => s.selfOnline);

  const [links, setLinks] = React.useState<InviteLink[]>([]);
  const [primaryLink, setPrimaryLink] = React.useState<InviteLink | null>(null);
  const [linksCursor, setLinksCursor] = React.useState<string | null>(null);
  const [linksLoadingMore, setLinksLoadingMore] = React.useState(false);
  const [linkName, setLinkName] = React.useState("");
  const [expiryDays, setExpiryDays] = React.useState<number>(0);
  const [maxUses, setMaxUses] = React.useState<number>(0);
  const [copied, setCopied] = React.useState<string | null>(null);
  const [linkBusy, setLinkBusy] = React.useState(false);
  const copyTimer = React.useRef<ReturnType<typeof setTimeout> | null>(null);
  const linksGen = React.useRef(0);
  const revokedTokens = React.useRef<Set<string>>(new Set());
  const extraLinks = links;

  const loadLinks = React.useCallback(async () => {
    const gen = ++linksGen.current;
    try {
      const res = await serverAPI.listInviteLinks(server.id);
      if (gen !== linksGen.current) return;
      setPrimaryLink(res.primary);
      setLinks(res.links.filter((l) => !revokedTokens.current.has(l.token)));
      setLinksCursor(res.nextCursor);
    } catch {
      return;
    }
  }, [server.id]);

  const loadMoreLinks = async () => {
    if (!linksCursor || linksLoadingMore) return;
    const gen = linksGen.current;
    setLinksLoadingMore(true);
    try {
      const res = await serverAPI.listInviteLinks(server.id, { cursor: linksCursor });
      if (gen !== linksGen.current) return;
      setLinks((prev) => {
        const known = new Set(prev.map((l) => l.token));
        return [
          ...prev,
          ...res.links.filter((l) => !known.has(l.token) && !revokedTokens.current.has(l.token)),
        ];
      });
      setLinksCursor(res.nextCursor);
    } catch {
      return;
    } finally {
      setLinksLoadingMore(false);
    }
  };

  React.useEffect(() => {
    if (!canManageInvites) return;
    void loadLinks();
  }, [canManageInvites, loadLinks]);

  const copyLink = (link: InviteLink) => {
    void copyText(link.url).then((ok) => {
      if (!ok) return;
      setCopied(link.token);
      if (copyTimer.current) clearTimeout(copyTimer.current);
      copyTimer.current = setTimeout(() => setCopied(null), 1600);
    });
  };

  const createLink = async () => {
    setLinkBusy(true);
    try {
      const created = await serverAPI.createInviteLink(server.id, {
        name: linkName.trim() || undefined,
        expiresDays: expiryDays || undefined,
        maxUses: maxUses || undefined,
      });
      setLinks((prev) => [created, ...prev]);
      setLinkName("");
    } catch {
      return;
    } finally {
      setLinkBusy(false);
    }
  };

  const revokeLink = async (link: InviteLink) => {
    setLinkBusy(true);
    try {
      await serverAPI.revokeInviteLink(server.id, link.token);
      revokedTokens.current.add(link.token);
      setLinks((prev) => prev.filter((l) => l.token !== link.token));
      if (link.isPrimary) await loadLinks();
    } catch {
      return;
    } finally {
      setLinkBusy(false);
    }
  };

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

  const handleSave = async () => {
    const trimmed = name.trim();
    if (trimmed.length < 3) {
      setSaveError(t("nameTooShort"));
      return;
    }
    setSaving(true);
    setSaveError(null);
    try {
      await serverAPI.updateServer(server.id, {
        name: trimmed,
        description: description.trim() || undefined,
      });
      patchServer(server.id, {
        name: trimmed,
        description: description.trim() || null,
      });
      setSaved(true);
      setTimeout(() => setSaved(false), 1600);
    } catch (err) {
      const apiErr = err instanceof ApiError ? err : null;
      if (apiErr?.code === "SERVER_NAME_NOT_ALLOWED") {
        setSaveError(t("nameNotAllowed"));
      } else if (apiErr?.code === "SERVER_DESCRIPTION_NOT_ALLOWED") {
        setSaveError(t("descriptionNotAllowed"));
      } else if (apiErr?.code === "MODERATION_UNAVAILABLE") {
        setSaveError(t("moderationUnavailable"));
      } else {
        setSaveError(apiErr?.message || t("common:error"));
      }
    } finally {
      setSaving(false);
    }
  };

  const handleInvite = async (friendId: string) => {
    setInviteError(null);
    setInvitePending(friendId);
    try {
      await dmAPI.sendServerInvite(friendId, server.id);
      setInvitedIds((prev) => new Set(prev).add(friendId));
    } catch (err) {
      const apiErr = err instanceof ApiError ? err : null;
      if (apiErr?.code === "ALREADY_MEMBER") {
        setMembers((prev) =>
          prev.some((m) => m.id === friendId)
            ? prev
            : [
                ...prev,
                {
                  id: friendId,
                  name:
                    friends.find((f) => f.friendId === friendId)?.friendName ??
                    friendId,
                },
              ],
        );
      } else if (apiErr?.code === "INVITE_ALREADY_PENDING") {
        setInvitedIds((prev) => new Set(prev).add(friendId));
      } else {
        setInviteError(apiErr?.message || t("common:error"));
      }
    } finally {
      setInvitePending(null);
    }
  };

  const accent = server.accentColor ?? colorForId(server.id);

  const onlineCount = members.filter((m) =>
    m.id === currentUserId ? selfOnline : !!online[m.id],
  ).length;

  const inviteQ = inviteQuery.trim().toLowerCase();
  const inviteCandidates = friends.filter(
    (f) =>
      f.status === "accepted" &&
      (!inviteQ || f.friendName.toLowerCase().includes(inviteQ)),
  );
  const pendingInvites = friends.filter(
    (f) => invitedIds.has(f.friendId) && !memberIds.has(f.friendId),
  );

  const effectiveMembers = React.useMemo(
    () =>
      members.map((m) =>
        nameOverrides[m.id] ? { ...m, name: nameOverrides[m.id] } : m,
      ),
    [members, nameOverrides],
  );
  const memberQ = memberQuery.trim().toLowerCase();
  const visibleMembers = memberQ
    ? effectiveMembers.filter((m) => m.name.toLowerCase().includes(memberQ))
    : effectiveMembers;

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
            <>
              <h1>{t("nav.overview")}</h1>
              <p className="lead">{t("overviewLead")}</p>
              <div className="ssv-hero">
                <input
                  ref={logoInputRef}
                  type="file"
                  accept="image/png,image/jpeg,image/webp"
                  hidden
                  onChange={(e) => {
                    const f = e.target.files?.[0];
                    if (f) void handleLogoFile(f);
                    e.target.value = "";
                  }}
                />
                <ServerLogo
                  serverId={server.id}
                  name={server.name}
                  className="ssv-hero-ava"
                  style={{
                    background: accent,
                    cursor: canManageServer ? "pointer" : undefined,
                  }}
                  title={canManageServer ? t("uploadLogo") : undefined}
                >
                  {canManageServer && (
                    <span
                      className="edit-badge ssv-logo-badge"
                      role="button"
                      tabIndex={0}
                      onClick={() => logoInputRef.current?.click()}
                      onKeyDown={(e) => {
                        if (e.key === "Enter" || e.key === " ") {
                          logoInputRef.current?.click();
                        }
                      }}
                    >
                      <Icon name="camera" size={12} />
                    </span>
                  )}
                </ServerLogo>
                {canManageServer && (
                  <div className="ssv-logo-actions">
                    <button
                      type="button"
                      className="ssv-btn"
                      disabled={logoBusy}
                      onClick={() => logoInputRef.current?.click()}
                    >
                      {t("uploadLogo")}
                    </button>
                    {logoUrl && (
                      <button
                        type="button"
                        className="ssv-btn danger"
                        disabled={logoBusy}
                        onClick={() => void handleLogoRemove()}
                      >
                        {t("removeLogo")}
                      </button>
                    )}
                  </div>
                )}
              </div>
              <label className="ssv-label">{t("serverName")}</label>
              <input
                className="ssv-input"
                value={name}
                maxLength={20}
                disabled={!canManageServer}
                onChange={(e) => {
                  setName(e.target.value);
                  setSaveError(null);
                }}
              />
              <label className="ssv-label">{t("description")}</label>
              <textarea
                className="ssv-input ssv-textarea"
                value={description}
                maxLength={300}
                disabled={!canManageServer}
                onChange={(e) => setDescription(e.target.value)}
              />
              <div className="ssv-hint">{t("descriptionHint")}</div>
              {saveError && <div className="ssv-error">{saveError}</div>}
              {canManageServer && (
                <div className="ssv-actions">
                  <button
                    type="button"
                    className={"ssv-btn primary" + (saved ? " saved" : "")}
                    disabled={saving}
                    onClick={handleSave}
                  >
                    {saved ? t("common:saved") : t("common:save")}
                  </button>
                  <button
                    type="button"
                    className="ssv-btn"
                    onClick={() => {
                      setName(server.name);
                      setDescription(server.description ?? "");
                      setSaveError(null);
                    }}
                  >
                    {t("common:cancel")}
                  </button>
                </div>
              )}
            </>
          )}

          {section === "channels" && (
            <>
              <h1>{t("nav.channels")}</h1>
              <p className="lead">
                {t("channelsLead", { count: channels.length, server: server.name })}
              </p>
              {canManageChannels && (
              <form
                className="ssv-invite-card"
                onSubmit={(e) => {
                  e.preventDefault();
                  void submitNewChannel();
                }}
              >
                <div className="ssv-invite-head">
                  <Icon name="hash" size={16} />
                  <span>{t("createChannel")}</span>
                </div>
                <div className="ssv-invite-row">
                  <input
                    className="ssv-input ssv-create-input"
                    placeholder={t("modals:createChannel.namePlaceholder")}
                    value={newChName}
                    maxLength={100}
                    disabled={newChBusy}
                    onChange={(e) => {
                      setNewChName(e.target.value);
                      setNewChError(null);
                    }}
                  />
                  <button
                    type="submit"
                    className="ssv-btn primary"
                    disabled={newChBusy}
                  >
                    <Icon name="plus" size={14} />
                    {t("createChannel")}
                  </button>
                </div>
                <div className="ssv-invite-opts">
                  <label className="ssv-invite-opt">
                    <span className="ssv-invite-opt-label">{t("channelKind")}</span>
                    <select
                      value={newChKind}
                      disabled={newChBusy}
                      onChange={(e) => setNewChKind(e.target.value as ChannelKind)}
                    >
                      <option value="text">{t("kindText")}</option>
                      <option value="voice">{t("kindVoice")}</option>
                    </select>
                  </label>
                  <label className="ssv-invite-opt">
                    <span className="ssv-invite-opt-label">{t("channelVisibility")}</span>
                    <select
                      value={newChVisibility}
                      disabled={newChBusy}
                      onChange={(e) =>
                        setNewChVisibility(e.target.value as ChannelVisibility)
                      }
                    >
                      <option value="public">{t("modals:createChannel.public")}</option>
                      <option value="private">{t("modals:createChannel.private")}</option>
                    </select>
                  </label>
                  {newChVisibility === "private" && (
                    <label className="ssv-invite-opt">
                      <span className="ssv-invite-opt-label">{t("channelPassword")}</span>
                      <input
                        className="ssv-create-password"
                        type="password"
                        value={newChPassword}
                        disabled={newChBusy}
                        onChange={(e) => {
                          setNewChPassword(e.target.value);
                          setNewChError(null);
                        }}
                      />
                    </label>
                  )}
                </div>
                {newChError && <div className="ssv-error">{newChError}</div>}
              </form>
              )}
              <div className="ssv-section-label">
                {t("channelList", { count: channels.length })}
              </div>
              <div className="ssv-card">
                {channels.length === 0 && (
                  <div className="ssv-empty">{t("noChannels")}</div>
                )}
                {channels.map((c, i) => (
                  <React.Fragment key={c.id}>
                    {i > 0 && <div className="ssv-divider" />}
                    <div className="ssv-row">
                      <span className="ssv-channel-ico">
                        <Icon name={c.kind === "voice" ? "volume" : "hash"} size={15} />
                      </span>
                      <div className="ssv-row-main">
                        <div className="ssv-row-name">
                          {c.name}
                          {c.isDefault && (
                            <span className="ssv-default-badge">{t("defaultChannelBadge")}</span>
                          )}
                        </div>
                        <div className="ssv-row-sub">
                          {c.kind === "voice" ? t("kindVoice") : t("kindText")}
                          {c.visibility === "private" && ` · ${t("privateChannel")}`}
                        </div>
                        {c.isDefault && (
                          <div className="ssv-row-note">{t("defaultChannelNote")}</div>
                        )}
                      </div>
                      {c.visibility === "private" && (
                        <span className="ssv-tag">
                          <Icon name="lock" size={11} />
                        </span>
                      )}
                      {canManageChannels && !c.isDefault && (
                        <button
                          type="button"
                          className="ssv-row-delete"
                          title={t("deleteChannel")}
                          onClick={() => onDeleteChannel(c)}
                        >
                          <Icon name="trash" size={14} />
                        </button>
                      )}
                    </div>
                  </React.Fragment>
                ))}
              </div>
            </>
          )}

          {section === "roles" && (
            <>
              <h1>{t("nav.roles")}</h1>
              <p className="lead">{t("rolesLead", { server: server.name })}</p>
              {canManageRoles && (
                <form
                  className="ssv-invite-card"
                  onSubmit={(e) => {
                    e.preventDefault();
                    void submitCreateRole();
                  }}
                >
                  <div className="ssv-invite-head">
                    <Icon name="shield" size={16} />
                    <span>{t("createRole")}</span>
                  </div>
                  <div className="ssv-invite-row">
                    <input
                      className="ssv-input ssv-create-input"
                      placeholder={t("roleName")}
                      value={newRoleName}
                      maxLength={40}
                      disabled={roleBusy}
                      onChange={(e) => {
                        setNewRoleName(e.target.value);
                        setRoleError(null);
                      }}
                    />
                    <button type="submit" className="ssv-btn primary" disabled={roleBusy}>
                      <Icon name="plus" size={14} />
                      {t("createRole")}
                    </button>
                  </div>
                  <div className="ssv-role-colors">
                    {ROLE_COLORS.map((c) => (
                      <button
                        key={c}
                        type="button"
                        className={
                          "ssv-role-color" + (newRoleColor === c ? " on" : "")
                        }
                        style={{ background: c }}
                        onClick={() => setNewRoleColor(c)}
                      />
                    ))}
                  </div>
                  <div className="ssv-role-perms">
                    {PERMS.map((perm) => {
                      const on = (newRolePerms & perm.bit) !== 0;
                      const locked =
                        roleBusy || (!on && (grantableBits & perm.bit) === 0);
                      return (
                      <div key={perm.bit} className="ssv-role-perm">
                        <span>{t(perm.key)}</span>
                        <button
                          type="button"
                          className={"s-switch" + (on ? " on" : "")}
                          disabled={locked}
                          onClick={() =>
                            setNewRolePerms((prev) =>
                              on ? prev & ~perm.bit : prev | perm.bit,
                            )
                          }
                        />
                      </div>
                      );
                    })}
                  </div>
                </form>
              )}
              {roleError && <div className="ssv-error">{roleError}</div>}
              <div className="ssv-section-label">
                {t("roleList", { count: rolesData?.roles.length ?? 0 })}
              </div>
              <div className="ssv-card">
                {(rolesData?.roles.length ?? 0) === 0 && (
                  <div className="ssv-empty">{t("noRoles")}</div>
                )}
                {(() => {
                  const sortedRoles = rolesData?.roles ?? [];
                  return sortedRoles.map((role, i) => {
                  const expanded = expandedRoleId === role.id;
                  const holders = effectiveMembers.filter((m) =>
                    (rolesData?.memberRoles[m.id] ?? []).includes(role.id),
                  );
                  return (
                    <React.Fragment key={role.id}>
                      {i > 0 && <div className="ssv-divider" />}
                      <div
                        className="ssv-row ssv-role-row"
                        role="button"
                        tabIndex={0}
                        onClick={() =>
                          setExpandedRoleId(expanded ? null : role.id)
                        }
                        onKeyDown={(e) => {
                          if (e.key === "Enter" || e.key === " ") {
                            e.preventDefault();
                            setExpandedRoleId(expanded ? null : role.id);
                          }
                        }}
                      >
                        <span
                          className="ssv-role-dot"
                          style={{ background: role.color }}
                        />
                        <div className="ssv-row-main">
                          <div className="ssv-row-name">{role.name}</div>
                          <div className="ssv-row-sub">
                            {PERMS.filter((p2) => (role.permissions & p2.bit) !== 0)
                              .map((p2) => t(p2.key))
                              .join(" · ") || "—"}
                          </div>
                        </div>
                        <span className="ssv-tag">{holders.length}</span>
                        {canManageRoles && (
                          <span
                            className="ssv-role-reorder"
                            onClick={(e) => e.stopPropagation()}
                          >
                            <button
                              type="button"
                              className="ssv-row-delete"
                              disabled={
                                i === 0 ||
                                !canTouchRole(role.position) ||
                                !canTouchRole(sortedRoles[i - 1].position)
                              }
                              title="↑"
                              onClick={() => void swapRoles(role, sortedRoles[i - 1])}
                            >
                              <Icon name="chev" size={13} className="flip-y" />
                            </button>
                            <button
                              type="button"
                              className="ssv-row-delete"
                              disabled={
                                i === sortedRoles.length - 1 ||
                                !canTouchRole(role.position) ||
                                !canTouchRole(sortedRoles[i + 1].position)
                              }
                              title="↓"
                              onClick={() => void swapRoles(role, sortedRoles[i + 1])}
                            >
                              <Icon name="chev" size={13} />
                            </button>
                          </span>
                        )}
                        {canTouchRole(role.position) && outranksAllHolders(role.id) && (
                          <button
                            type="button"
                            className="ssv-row-delete"
                            title={t("deleteRole")}
                            onClick={(e) => {
                              e.stopPropagation();
                              void removeRole(role.id);
                            }}
                          >
                            <Icon name="trash" size={14} />
                          </button>
                        )}
                      </div>
                      {expanded && canTouchRole(role.position) && (
                        <div className="ssv-role-detail">
                          <div className="ssv-role-colors">
                            {ROLE_COLORS.map((c) => (
                              <button
                                key={c}
                                type="button"
                                className={
                                  "ssv-role-color" + (role.color === c ? " on" : "")
                                }
                                style={{ background: c }}
                                onClick={() => enqueueRoleEdit(role, { color: c })}
                              />
                            ))}
                          </div>
                          <div className="ssv-role-perms">
                            {PERMS.map((perm) => {
                              const on =
                                ((rolePermsDraft[role.id] ?? role.permissions) &
                                  perm.bit) !==
                                0;
                              const locked = on
                                ? !outranksAllHolders(role.id)
                                : (grantableBits & perm.bit) === 0;
                              return (
                                <div key={perm.bit} className="ssv-role-perm">
                                  <span>{t(perm.key)}</span>
                                  <button
                                    type="button"
                                    className={"s-switch" + (on ? " on" : "")}
                                    disabled={locked}
                                    onClick={() =>
                                      toggleRolePerm(role, perm.bit, !on)
                                    }
                                  />
                                </div>
                              );
                            })}
                          </div>
                          <div className="ssv-section-label">{t("roleMembers")}</div>
                          <div className="ssv-role-members">
                            {effectiveMembers.map((m) => {
                              const has = (rolesData?.memberRoles[m.id] ?? []).includes(
                                role.id,
                              );
                              return (
                                <div key={m.id} className="ssv-role-member-row">
                                  <UserAvatar
                                    userId={m.id}
                                    name={m.name}
                                    className="ssv-role-member-ava"
                                  />
                                  <span className="ssv-role-member-name">
                                    {m.name}
                                  </span>
                                  <button
                                    type="button"
                                    className={"s-switch" + (has ? " on" : "")}
                                    disabled={
                                      !outranksMember(m.id) ||
                                      (!has &&
                                        (role.permissions & ~grantableBits) !== 0)
                                    }
                                    onClick={() =>
                                      void toggleAssignment(role.id, m.id, has)
                                    }
                                  />
                                </div>
                              );
                            })}
                          </div>
                        </div>
                      )}
                    </React.Fragment>
                  );
                  });
                })()}
              </div>
            </>
          )}

          {section === "invites" && canManageInvites && (
            <>
              <h1>{t("nav.invites")}</h1>
              <p className="lead">{t("invitesLead", { server: server.name })}</p>

              <div className="ssv-invite-card">
                <div className="ssv-invite-head">
                  <Icon name="link" size={16} />
                  <span>{t("inviteLink")}</span>
                </div>
                <p className="ssv-invite-note">{t("primaryLinkHint")}</p>
                <div className="ssv-invite-row">
                  <div className="ssv-invite-code" title={primaryLink?.url ?? undefined}>
                    <span className="ssv-invite-url">{primaryLink?.url ?? "—"}</span>
                  </div>
                  <button
                    type="button"
                    className={
                      "ssv-btn primary" +
                      (primaryLink && copied === primaryLink.token ? " saved" : "")
                    }
                    disabled={!primaryLink}
                    onClick={() => primaryLink && copyLink(primaryLink)}
                  >
                    <Icon
                      name={primaryLink && copied === primaryLink.token ? "check" : "link"}
                      size={14}
                    />
                    {primaryLink && copied === primaryLink.token ? t("copied") : t("copy")}
                  </button>
                  <button
                    type="button"
                    className="ssv-btn"
                    disabled={!primaryLink || linkBusy}
                    title={t("regenerateHint")}
                    onClick={() => primaryLink && revokeLink(primaryLink)}
                  >
                    <Icon name="refresh" size={14} />
                    {t("regenerate")}
                  </button>
                </div>
              </div>

              <div className="ssv-section-label">{t("newLink")}</div>
              <div className="ssv-invite-card">
                <div className="ssv-invite-opts">
                  <label className="ssv-invite-opt grow">
                    <span className="ssv-invite-opt-label">{t("linkName")}</span>
                    <input
                      value={linkName}
                      maxLength={60}
                      placeholder={t("linkNamePlaceholder")}
                      onChange={(e) => setLinkName(e.target.value)}
                    />
                  </label>
                  <label className="ssv-invite-opt">
                    <span className="ssv-invite-opt-label">{t("expires")}</span>
                    <select
                      value={expiryDays}
                      onChange={(e) => setExpiryDays(Number(e.target.value))}
                    >
                      {EXPIRY_OPTIONS.map((d) => (
                        <option key={d} value={d}>
                          {d === 0 ? t("expiryNever") : t("expiryDays", { count: d })}
                        </option>
                      ))}
                    </select>
                  </label>
                  <label className="ssv-invite-opt">
                    <span className="ssv-invite-opt-label">{t("maxUses")}</span>
                    <select
                      value={maxUses}
                      onChange={(e) => setMaxUses(Number(e.target.value))}
                    >
                      {MAX_USES_OPTIONS.map((n) => (
                        <option key={n} value={n}>
                          {n === 0 ? t("noLimit") : n}
                        </option>
                      ))}
                    </select>
                  </label>
                  <button
                    type="button"
                    className="ssv-invite-refresh"
                    disabled={linkBusy}
                    onClick={createLink}
                  >
                    <Icon name="plus" size={16} />
                    <span>{t("createLink")}</span>
                  </button>
                </div>
              </div>

              {extraLinks.length > 0 && (
                <>
                  <div className="ssv-section-label">
                    {t("activeInvites", { count: extraLinks.length })}
                  </div>
                  <div className="ssv-card">
                    {extraLinks.map((inv, i) => (
                      <React.Fragment key={inv.token}>
                        {i > 0 && <div className="ssv-divider" />}
                        <div className="ssv-row">
                          <UserAvatar
                            userId={inv.createdBy}
                            name={inv.creatorName ?? "?"}
                            className="ssv-row-ava"
                          />
                          <div className="ssv-row-main">
                            <div className="ssv-invite-code-line">
                              <span className="ssv-invite-mono ssv-invite-url" title={inv.url}>
                                {inv.name ?? inv.url.replace(/^https?:\/\//, "")}
                              </span>
                              {inv.creatorName && (
                                <span className="ssv-invite-by">
                                  · {inv.creatorName}
                                </span>
                              )}
                            </div>
                            <div className="ssv-row-sub">
                              {daysLeft(inv.expiresAt) != null
                                ? t("expiresIn", {
                                    count: daysLeft(inv.expiresAt) ?? 0,
                                  })
                                : t("expiryNever")}{" "}
                              · {t("useCount", { count: inv.uses })}
                              {inv.maxUses != null && ` / ${inv.maxUses}`}
                            </div>
                          </div>
                          <button
                            type="button"
                            className="ssv-icon-btn"
                            title={t("copy")}
                            onClick={() => copyLink(inv)}
                          >
                            <Icon
                              name={copied === inv.token ? "check" : "link"}
                              size={15}
                            />
                          </button>
                          <button
                            type="button"
                            className="ssv-icon-btn danger"
                            title={t("revoke")}
                            disabled={linkBusy}
                            onClick={() => revokeLink(inv)}
                          >
                            <Icon name="x" size={15} />
                          </button>
                        </div>
                      </React.Fragment>
                    ))}
                  </div>
                  {linksCursor && (
                    <button
                      type="button"
                      className="ssv-link-btn"
                      disabled={linksLoadingMore}
                      onClick={() => void loadMoreLinks()}
                    >
                      {t("loadMoreLinks")}
                    </button>
                  )}
                </>
              )}

              <div className="ssv-section-label">{t("inviteFriendsTitle")}</div>
              <div className="ssv-search">
                <Icon name="search" size={15} />
                <input
                  placeholder={t("searchFriends")}
                  value={inviteQuery}
                  onChange={(e) => setInviteQuery(e.target.value)}
                />
              </div>
              <div className="ssv-card">
                {inviteCandidates.length === 0 && (
                  <div className="ssv-empty">{t("noFriends")}</div>
                )}
                {inviteCandidates.map((f, i) => {
                  const isMember = memberIds.has(f.friendId);
                  const sent = invitedIds.has(f.friendId);
                  const busy = invitePending === f.friendId;
                  return (
                    <React.Fragment key={f.friendId}>
                      {i > 0 && <div className="ssv-divider" />}
                      <div className="ssv-row">
                        <UserAvatar
                          userId={f.friendId}
                          name={f.friendName}
                          className="ssv-row-ava"
                        />
                        <div className="ssv-row-main">
                          <div className="ssv-row-name">{f.friendName}</div>
                        </div>
                        {isMember ? (
                          <span className="ssv-tag">{t("alreadyMember")}</span>
                        ) : (
                          <button
                            type="button"
                            className={"ssv-btn small" + (sent ? "" : " primary")}
                            disabled={sent || busy}
                            onClick={() => handleInvite(f.friendId)}
                          >
                            {sent ? (
                              <>
                                <Icon name="check" size={12} /> {t("invited")}
                              </>
                            ) : (
                              t("invite")
                            )}
                          </button>
                        )}
                      </div>
                    </React.Fragment>
                  );
                })}
              </div>
              {inviteError && <div className="ssv-error">{inviteError}</div>}
              {pendingInvites.length > 0 && (
                <>
                  <div className="ssv-section-label">
                    {t("pendingInvites", { count: pendingInvites.length })}
                  </div>
                  <div className="ssv-card">
                    {pendingInvites.map((f, i) => (
                      <React.Fragment key={f.friendId}>
                        {i > 0 && <div className="ssv-divider" />}
                        <div className="ssv-row">
                          <UserAvatar
                            userId={f.friendId}
                            name={f.friendName}
                            className="ssv-row-ava"
                          />
                          <div className="ssv-row-main">
                            <div className="ssv-row-name">{f.friendName}</div>
                            <div className="ssv-row-sub">{t("invitePending")}</div>
                          </div>
                        </div>
                      </React.Fragment>
                    ))}
                  </div>
                </>
              )}
            </>
          )}

          {section === "members" && (
            <>
              <h1>{t("nav.members")}</h1>
              <p className="lead">
                {t("membersLead", { count: members.length, server: server.name })}
              </p>
              <div className="ssv-search">
                <Icon name="search" size={15} />
                <input
                  placeholder={t("searchMembers")}
                  value={memberQuery}
                  onChange={(e) => setMemberQuery(e.target.value)}
                />
              </div>
              <div className="ssv-card">
                {visibleMembers.length === 0 && (
                  <div className="ssv-empty">{t("noMembers")}</div>
                )}
                {visibleMembers.map((m, i) => {
                  const isOnline =
                    m.id === currentUserId ? selfOnline : !!online[m.id];
                  return (
                    <React.Fragment key={m.id}>
                      {i > 0 && <div className="ssv-divider" />}
                      <div className={"ssv-row" + (isOnline ? "" : " off")}>
                        <div className="ssv-row-ava-wrap">
                          <UserAvatar
                            userId={m.id}
                            name={m.name}
                            className="ssv-row-ava"
                          />
                          {isOnline && <span className="ssv-row-dot" />}
                        </div>
                        <div className="ssv-row-main">
                          <div className="ssv-row-name">{m.name}</div>
                          <div className="ssv-row-sub">
                            {isOnline
                              ? t("common:statusOnline", {
                                  defaultValue: t("online"),
                                })
                              : t("offline")}
                          </div>
                        </div>
                        {(() => {
                          const top = topRoleFor(rolesData, m.id);
                          return top ? (
                            <span
                              className="ssv-tag"
                              style={{
                                color: top.color,
                                background: `color-mix(in srgb, ${top.color} 16%, transparent)`,
                              }}
                            >
                              {top.name}
                            </span>
                          ) : null;
                        })()}
                        <span
                          className={
                            "ssv-tag" + (m.id === server.ownerId ? " owner" : "")
                          }
                        >
                          {m.id === server.ownerId ? t("roleOwner") : t("roleMember")}
                        </span>
                        {canKick(m.id) && (
                          <button
                            type="button"
                            className="ssv-row-delete"
                            title={t("kickMember")}
                            onClick={() => void kickMember(m.id)}
                          >
                            <Icon name="logout" size={14} />
                          </button>
                        )}
                      </div>
                    </React.Fragment>
                  );
                })}
              </div>
            </>
          )}

          {section === "danger" && (
            <>
              <h1>{t("nav.danger")}</h1>
              <p className="lead">{t("dangerLead")}</p>
              <div className="ssv-danger-card">
                <div className="ssv-danger-info">
                  <div className="ssv-danger-title">
                    {isOwner ? t("deleteServer") : t("leaveServer")}
                  </div>
                  <div className="ssv-danger-sub">
                    {isOwner ? t("deleteServerHint") : t("leaveServerHint")}
                  </div>
                </div>
                <button
                  type="button"
                  className="ssv-btn danger"
                  onClick={() => {
                    onClose();
                    onDeleteServer(server);
                  }}
                >
                  {isOwner ? t("common:delete") : t("leave")}
                </button>
              </div>
            </>
          )}
        </div>
      </div>
    </div>
  );
};

export default ServerSettingsView;
