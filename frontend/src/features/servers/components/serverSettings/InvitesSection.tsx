import React from "react";
import { useTranslation } from "react-i18next";
import Icon from "../../../../shared/ui/Icon";
import { serverAPI } from "../../api/server";
import { dmAPI } from "../../../dm/api/dm";
import { ApiError } from "../../../../shared/lib/apiError";
import { UserAvatar } from "../../../../shared/ui/Avatar";
import type { Friendship } from "../../../../shared/types/friendship";
import type { InviteLink, Server } from "../../../../shared/types/server";


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

interface InvitesSectionProps {
  server: Server;
  friends: Friendship[];
  memberIds: Set<string>;
  invitedIds: Set<string>;
  setInvitedIds: React.Dispatch<React.SetStateAction<Set<string>>>;
  onAlreadyMember: (memberId: string, name: string) => void;
  canManageInvites: boolean;
}

export const InvitesSection: React.FC<InvitesSectionProps> = ({
  server,
  friends,
  memberIds,
  invitedIds,
  setInvitedIds,
  onAlreadyMember,
  canManageInvites,
}) => {
  const { t } = useTranslation(["serverSettings", "common", "modals"]);
  const [inviteQuery, setInviteQuery] = React.useState("");
  const [invitePending, setInvitePending] = React.useState<string | null>(null);
  const [inviteError, setInviteError] = React.useState<string | null>(null);
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
    if (!canManageInvites) {
      linksGen.current += 1;
      setLinks([]);
      setPrimaryLink(null);
      setLinksCursor(null);
      return;
    }
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

  const handleInvite = async (friendId: string) => {
    setInviteError(null);
    setInvitePending(friendId);
    try {
      await dmAPI.sendServerInvite(friendId, server.id);
      setInvitedIds((prev) => new Set(prev).add(friendId));
    } catch (err) {
      const apiErr = err instanceof ApiError ? err : null;
      if (apiErr?.code === "ALREADY_MEMBER") {
        onAlreadyMember(
          friendId,
          friends.find((f) => f.friendId === friendId)?.friendName ?? friendId,
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
  const inviteQ = inviteQuery.trim().toLowerCase();
  const inviteCandidates = friends.filter(
    (f) =>
      f.status === "accepted" &&
      (!inviteQ || f.friendName.toLowerCase().includes(inviteQ)),
  );
  const pendingInvites = friends.filter(
    (f) => invitedIds.has(f.friendId) && !memberIds.has(f.friendId),
  );

  return (
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
  );
};

export default InvitesSection;
