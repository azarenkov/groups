import React from "react";
import { useTranslation } from "react-i18next";
import Icon, { type IconName } from "./Icon";
import { serverAPI } from "../../api/server";
import { dmAPI } from "../../api/dm";
import { useServerStore } from "../../store/serverStore";
import { usePresenceStore } from "../../store/presenceStore";
import { ApiError } from "../../utils/apiError";
import { colorForId, initialFor } from "../../utils/avatar";
import type { Friendship } from "../../types/friendship";
import type { Server } from "../../types/server";

type SectionId = "overview" | "invites" | "members" | "danger";

const SECTIONS: { id: SectionId; icon: IconName }[] = [
  { id: "overview", icon: "grid" },
  { id: "invites", icon: "user-plus" },
  { id: "members", icon: "users" },
];

interface ServerSettingsViewProps {
  server: Server;
  friends: Friendship[];
  currentUserId?: string;
  onClose: () => void;
  onDeleteServer: (server: Server) => void;
}

const ServerSettingsView: React.FC<ServerSettingsViewProps> = ({
  server,
  friends,
  currentUserId,
  onClose,
  onDeleteServer,
}) => {
  const { t } = useTranslation(["serverSettings", "common"]);
  const [section, setSection] = React.useState<SectionId>("overview");
  const isOwner = currentUserId === server.ownerId;

  const [name, setName] = React.useState(server.name);
  const [description, setDescription] = React.useState(server.description ?? "");
  const [saving, setSaving] = React.useState(false);
  const [saveError, setSaveError] = React.useState<string | null>(null);
  const [saved, setSaved] = React.useState(false);
  const setServers = useServerStore((s) => s.setServers);
  const servers = useServerStore((s) => s.servers);

  const [members, setMembers] = React.useState<{ id: string; name: string }[]>([]);
  const [invitedIds, setInvitedIds] = React.useState<Set<string>>(new Set());
  const [inviteQuery, setInviteQuery] = React.useState("");
  const [invitePending, setInvitePending] = React.useState<string | null>(null);
  const [inviteError, setInviteError] = React.useState<string | null>(null);
  const [memberQuery, setMemberQuery] = React.useState("");

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
  }, [server.id]);

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
      setServers(
        servers.map((s) =>
          s.id === server.id
            ? { ...s, name: trimmed, description: description.trim() || null }
            : s,
        ),
      );
      setSaved(true);
      setTimeout(() => setSaved(false), 1600);
    } catch (err) {
      const apiErr = err instanceof ApiError ? err : null;
      setSaveError(apiErr?.message || t("common:error"));
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
  const glyph = server.name.slice(0, 2).toUpperCase();

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

  const memberQ = memberQuery.trim().toLowerCase();
  const visibleMembers = memberQ
    ? members.filter((m) => m.name.toLowerCase().includes(memberQ))
    : members;

  return (
    <div className="settings-overlay">
      <nav className="settings-rail">
        <div className="ssv-identity">
          <div className="ssv-identity-ava" style={{ background: accent }}>
            {glyph}
          </div>
          <div className="ssv-identity-info">
            <div className="ssv-identity-name">{server.name}</div>
            <div className="ssv-identity-sub">
              {t("memberCount", { count: members.length })} ·{" "}
              {t("onlineCount", { count: onlineCount })}
            </div>
          </div>
        </div>
        <div className="rail-section">{t("navTitle")}</div>
        {SECTIONS.map((s) => (
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
                <div className="ssv-hero-ava" style={{ background: accent }}>
                  {glyph}
                </div>
              </div>
              <label className="ssv-label">{t("serverName")}</label>
              <input
                className="ssv-input"
                value={name}
                maxLength={20}
                disabled={!isOwner}
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
                disabled={!isOwner}
                onChange={(e) => setDescription(e.target.value)}
              />
              <div className="ssv-hint">{t("descriptionHint")}</div>
              {saveError && <div className="ssv-error">{saveError}</div>}
              {isOwner && (
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

          {section === "invites" && (
            <>
              <h1>{t("nav.invites")}</h1>
              <p className="lead">{t("invitesLead", { server: server.name })}</p>
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
                        <div
                          className="ssv-row-ava"
                          style={{ background: colorForId(f.friendId) }}
                        >
                          {initialFor(f.friendName)}
                        </div>
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
                          <div
                            className="ssv-row-ava"
                            style={{ background: colorForId(f.friendId) }}
                          >
                            {initialFor(f.friendName)}
                          </div>
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
                          <div
                            className="ssv-row-ava"
                            style={{ background: colorForId(m.id) }}
                          >
                            {initialFor(m.name)}
                          </div>
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
                        <span
                          className={
                            "ssv-tag" + (m.id === server.ownerId ? " owner" : "")
                          }
                        >
                          {m.id === server.ownerId ? t("roleOwner") : t("roleMember")}
                        </span>
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
