import React from "react";
import { useTranslation } from "react-i18next";

import Icon from "../../../shared/ui/Icon";
import { CommunityLogo, UserAvatar } from "../../../shared/ui/Avatar";
import { ApiError } from "../../../shared/lib/apiError";
import { useFriendshipStore } from "../../social/store/friendshipStore";
import { useCommunityStore } from "../../communities/store/communityStore";
import { useChannelStore } from "../../communities/store/channelStore";
import { forwardMessage } from "../api/forward";
import type { ForwardTarget } from "../api/forward";
import {
  MAX_FORWARD_TARGETS,
  buildForwardOptions,
  forwardTargetKey,
  toggleForwardTarget,
} from "../lib/forwardTargets";
import type { ForwardOption } from "../lib/forwardTargets";
import { useForwardModalStore } from "../store/forwardModalStore";
import { MAX_CONTENT } from "../constants";

const ForwardModal: React.FC = () => {
  const { t } = useTranslation(["chat", "common"]);
  const source = useForwardModalStore((s) => s.source);
  const opening = useForwardModalStore((s) => s.opening);
  const close = useForwardModalStore((s) => s.close);
  const friends = useFriendshipStore((s) => s.friends);
  const friendsLoaded = useFriendshipStore((s) => s.loaded);
  const communities = useCommunityStore((s) => s.communities);
  const channelsByCommunity = useChannelStore((s) => s.channelsByCommunity);
  const [query, setQuery] = React.useState("");
  const [selected, setSelected] = React.useState<ForwardTarget[]>([]);
  const [comment, setComment] = React.useState("");
  const [sending, setSending] = React.useState(false);
  const [error, setError] = React.useState<string | null>(null);
  const [failed, setFailed] = React.useState<Record<string, string>>({});
  const [done, setDone] = React.useState(false);
  const activeOpeningRef = React.useRef(0);
  const closeTimerRef = React.useRef<ReturnType<typeof setTimeout> | null>(null);

  React.useEffect(() => {
    activeOpeningRef.current = opening;
    if (closeTimerRef.current) {
      clearTimeout(closeTimerRef.current);
      closeTimerRef.current = null;
    }
    setQuery("");
    setSelected([]);
    setComment("");
    setSending(false);
    setError(null);
    setFailed({});
    setDone(false);
  }, [opening]);

  React.useEffect(
    () => () => {
      if (closeTimerRef.current) clearTimeout(closeTimerRef.current);
    },
    [],
  );

  React.useEffect(() => {
    if (!source) return;
    if (!friendsLoaded) void useFriendshipStore.getState().loadAll();
    const loaded = useChannelStore.getState().channelsByCommunity;
    for (const c of useCommunityStore.getState().communities) {
      if (!loaded[c.id]) void useChannelStore.getState().fetchChannels(c.id);
    }
  }, [source, friendsLoaded]);

  React.useEffect(() => {
    if (!source) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key !== "Escape") return;
      e.stopPropagation();
      close();
    };
    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
  }, [source, close]);

  const options = React.useMemo(
    () => buildForwardOptions(friends, communities, channelsByCommunity, query),
    [friends, communities, channelsByCommunity, query],
  );

  if (!source) return null;

  const selectedKeys = new Set(selected.map(forwardTargetKey));
  const labelFor = new Map<string, string>(
    [...options.dms, ...options.channels].map((o) => [
      o.key,
      o.sublabel ? `${o.label} · ${o.sublabel}` : o.label,
    ]),
  );

  const submit = async () => {
    if (sending || selected.length === 0) return;
    const opened = activeOpeningRef.current;
    setSending(true);
    setError(null);
    setFailed({});
    try {
      const results = await forwardMessage(source, selected, comment);
      if (activeOpeningRef.current !== opened) return;
      const errors: Record<string, string> = {};
      for (const r of results) {
        if (r.error) errors[forwardTargetKey(r.target)] = r.error.message;
      }
      if (Object.keys(errors).length === 0) {
        setDone(true);
        closeTimerRef.current = setTimeout(() => {
          if (activeOpeningRef.current === opened) close();
        }, 900);
        return;
      }
      setFailed(errors);
      setSelected((prev) => prev.filter((t) => errors[forwardTargetKey(t)]));
    } catch (e) {
      if (activeOpeningRef.current !== opened) return;
      setError(e instanceof ApiError ? e.message : t("forwardFailed"));
    } finally {
      if (activeOpeningRef.current === opened) setSending(false);
    }
  };

  const row = (o: ForwardOption) => {
    const checked = selectedKeys.has(o.key);
    const limitReached = !checked && selected.length >= MAX_FORWARD_TARGETS;
    return (
      <button
        key={o.key}
        type="button"
        className={"pf-row forward-row" + (checked ? " checked" : "")}
        disabled={sending || done || limitReached}
        aria-pressed={checked}
        onClick={() => setSelected((prev) => toggleForwardTarget(prev, o.target))}
      >
        {o.target.kind === "dm" ? (
          <UserAvatar userId={o.avatarId} name={o.label} className="pf-ava" />
        ) : (
          <CommunityLogo communityId={o.avatarId} name={o.sublabel ?? o.label} className="pf-ava forward-community" />
        )}
        <span className="forward-row-text">
          <span className="pf-name">{o.label}</span>
          {o.sublabel && <span className="forward-row-sub">{o.sublabel}</span>}
          {failed[o.key] && <span className="forward-row-error">{failed[o.key]}</span>}
        </span>
        <span className="forward-check" aria-hidden="true">
          {checked && <Icon name="check" size={12} />}
        </span>
      </button>
    );
  };

  const empty = options.dms.length === 0 && options.channels.length === 0;

  return (
    <div className="modal-overlay" onMouseDown={close}>
      <div className="modal forward-modal" onMouseDown={(e) => e.stopPropagation()}>
        <div className="m-head">
          <h3>{t("forwardTitle")}</h3>
          <button type="button" className="close" onClick={close} title={t("common:close")}>
            <Icon name="x" size={14} />
          </button>
        </div>
        <div className="m-body">
          <div className="m-field" style={{ marginBottom: 10 }}>
            <input
              type="text"
              autoFocus
              placeholder={t("forwardSearch")}
              value={query}
              onChange={(e) => setQuery(e.target.value)}
            />
          </div>
          <div className="forward-list">
            {empty ? (
              <div className="pf-empty">{t("forwardEmpty")}</div>
            ) : (
              <>
                {options.dms.length > 0 && (
                  <div className="pf-section">
                    <div className="pf-section-title">{t("forwardSectionDms")}</div>
                    {options.dms.map(row)}
                  </div>
                )}
                {options.channels.length > 0 && (
                  <div className="pf-section">
                    <div className="pf-section-title">{t("forwardSectionChannels")}</div>
                    {options.channels.map(row)}
                  </div>
                )}
              </>
            )}
          </div>
          <div className="m-field forward-comment">
            <textarea
              rows={2}
              maxLength={MAX_CONTENT}
              placeholder={t("forwardComment")}
              value={comment}
              readOnly={sending || done}
              onChange={(e) => setComment(e.target.value)}
            />
          </div>
          {Object.keys(failed).length > 0 && (
            <div className="report-error">
              {t("forwardPartial", {
                targets: Object.keys(failed)
                  .map((k) => labelFor.get(k) ?? "")
                  .filter((l) => l.length > 0)
                  .join(", "),
              })}
            </div>
          )}
          {error && <div className="report-error">{error}</div>}
        </div>
        <div className="m-foot">
          <button type="button" onClick={close}>
            {t("common:cancel")}
          </button>
          <button
            type="button"
            className="primary"
            disabled={sending || done || selected.length === 0}
            onClick={() => void submit()}
          >
            {done
              ? t("forwardDone")
              : sending
                ? t("forwardSending")
                : selected.length > 0
                  ? t("forwardSendCount", { count: selected.length })
                  : t("forwardSend")}
          </button>
        </div>
      </div>
    </div>
  );
};

export default ForwardModal;
