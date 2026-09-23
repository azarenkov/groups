import React from "react";
import { useTranslation } from "react-i18next";
import { blockGeneration, useBlockStore } from "../../social/store/blockStore";
import { friendshipAPI } from "../../social/api/friendship";
import Icon from "../../../shared/ui/Icon";
import CallJoinBanner from "../../voice/components/CallJoinBanner";
import ConfirmModal from "../../../shared/ui/ConfirmModal";
import { channelAPI } from "../../communities/api/channel";
import { dmAPI } from "../../dm/api/dm";
import { ChannelMessage } from "../../../shared/types/channel";
import { Community } from "../../../shared/types/community";
import { ApiError } from "../../../shared/lib/apiError";
import i18n from "../../../i18n";
import { useUserNamesStore } from "../../social/store/userNamesStore";
import { refetchUnread } from "../store/unreadStore";
import {
  PERM_MANAGE_MESSAGES,
  useRolesStore,
  userPermissions,
} from "../../communities/store/rolesStore";
import ChannelHeader from "./ChannelHeader";
import Composer from "./Composer";
import SearchResults from "./SearchResults";
import MessageList from "./MessageList";
import { useAttachments } from "../hooks/useAttachments";
import { useMentions } from "../hooks/useMentions";
import { useChatWsMessages } from "../hooks/useChatWsMessages";
import { useMessageComposer } from "../hooks/useMessageComposer";
import { useMessageEditing } from "../hooks/useMessageEditing";
import { useMessages } from "../hooks/useMessages";
import { useMessageSearch } from "../hooks/useMessageSearch";
import { useMessenger } from "../hooks/useMessenger";
import { useReadReceipts } from "../hooks/useReadReceipts";
import { PULL_THRESHOLD, usePullToRefresh } from "../hooks/usePullToRefresh";
import type { ChannelMeta, DmMeta } from "../types";

interface ChatViewProps {
  channel?: ChannelMeta;
  communityId?: string;
  currentUserId?: string;
  dm?: DmMeta;
  membersOpen?: boolean;
  communities?: Community[];
  onToggleMembers?: () => void;
  onRemoveFriend?: () => void;
  onBlockUser?: () => void;
  onGoToCommunity?: (communityId: string) => void | Promise<void>;
  topSlot?: React.ReactNode;
  callMode?: boolean;
}

const ChatView: React.FC<ChatViewProps> = ({
  channel,
  communityId,
  currentUserId,
  dm,
  membersOpen,
  communities,
  onToggleMembers,
  onRemoveFriend,
  onBlockUser,
  onGoToCommunity,
  topSlot,
  callMode,
}) => {
  const { t } = useTranslation("chat");
  const attachments = useAttachments();
  const {
    pendingFiles,
    dragOver,
    setDragOver,
    fileInputRef,
    addFiles,
    removePendingFile,
    editFiles,
    editRemovedIds,
    addEditFiles,
    removeEditFile,
    discardPendingFiles,
    toggleRemoveAttachment,
  } = attachments;

  const dmBlocked = useBlockStore((s) => (dm ? !!s.blocked[dm.friendId] : false));
  const dmBlockedBy = useBlockStore((s) => (dm ? !!s.blockedBy[dm.friendId] : false));
  const blockedIds = useBlockStore((s) => s.blocked);
  const blocksLoaded = useBlockStore((s) => s.loaded);
  const [revealedResults, setRevealedResults] = React.useState<Set<string>>(() => new Set());
  const blockPeerId = dm?.friendId;
  React.useEffect(() => {
    if (!blockPeerId) return;
    let cancelled = false;
    const gen = blockGeneration(blockPeerId);
    friendshipAPI
      .blockRelation(blockPeerId)
      .then((relation) => {
        if (!cancelled) useBlockStore.getState().applyRelation(blockPeerId, relation, gen);
      })
      .catch(() => {});
    return () => {
      cancelled = true;
    };
  }, [blockPeerId]);

  const nameOverrides = useUserNamesStore((s) => s.overrides);
  const nameSyncTick = useUserNamesStore((s) => s.syncTick);

  const dmFriendId = dm?.friendId;
  const channelId = channel?.id;
  const { reseedChannelReads, receiptFor } = useReadReceipts({
    channelId,
    dmFriendId,
  });

  const isText = !!channel && !!communityId;
  const isDm = !!dm && !!dm.friendId;
  const convId = channelId ?? dmFriendId;
  const messenger = useMessenger({ channel, communityId, dm, isDm });

  const {
    messages,
    setMessages,
    messagesRef,
    loadingOlder,
    accessError,
    setAccessError,
    highlightedMessageId,
    setHighlightedMessageId,
    showInitialLoader,
    bodyRef,
    currentChannelIdRef,
    handleScroll,
    refreshHistory,
    jumpToMessage,
  } = useMessages({
    messenger,
    convId,
    isText,
    isDm,
    nameSyncTick,
    reseedChannelReads,
  });

  useChatWsMessages({
    channelId: channelId ?? null,
    dmFriendId: dmFriendId ?? null,
    currentUserId,
    setMessages,
  });

  const search = useMessageSearch({
    messenger,
    convId,
    nameSyncTick,
    onClose: () => setHighlightedMessageId(null),
  });
  const searchActive = search.searchOpen && search.searchInput.trim().length > 0;

  const pull = usePullToRefresh({
    bodyRef,
    enabled: !!messenger,
    convId,
    onRefresh: async () => {
      await refreshHistory();
      void refetchUnread().catch(() => {});
    },
  });

  const composer = useMessageComposer({
    messenger,
    channel,
    communityId,
    dm,
    isText,
    convId,
    attachments,
    setMessages,
    currentChannelIdRef,
    refreshHistory,
  });
  const { text, setText, composerRef, composerError, setComposerError } = composer;

  const editing = useMessageEditing({
    messenger,
    convId,
    currentUserId,
    attachments,
    messagesRef,
    setMessages,
  });

  const rolesData = useRolesStore((s) =>
    communityId ? s.byCommunity[communityId] : undefined,
  );
  const fetchRoles = useRolesStore((s) => s.fetch);
  React.useEffect(() => {
    if (communityId) void fetchRoles(communityId).catch(() => {});
  }, [communityId, fetchRoles]);
  const communityOwnerId = React.useMemo(
    () => communities?.find((sv) => sv.id === communityId)?.ownerId,
    [communities, communityId],
  );
  const {
    mentionQuery,
    setMentionQuery,
    mentionIndex,
    setMentionIndex,
    mentionSuggestions,
    resetParticipants,
    updateMentionQuery,
    applyMention,
  } = useMentions({
    channel,
    communityId,
    currentUserId,
    nameOverrides,
    text,
    setText,
    composerRef,
  });

  React.useEffect(() => {
    discardPendingFiles();
    setMentionQuery(null);
    setMentionIndex(0);
    resetParticipants();
  }, [
    convId,
    discardPendingFiles,
    setMentionQuery,
    setMentionIndex,
    resetParticipants,
  ]);

  const canModerateMessages =
    !!channel &&
    !!currentUserId &&
    (communityOwnerId === currentUserId ||
      (userPermissions(rolesData, currentUserId) & PERM_MANAGE_MESSAGES) !== 0);

  const handleJumpToMessage = async (target: ChannelMessage) => {
    search.clearResults();
    await jumpToMessage(target);
  };

  const handleJoinFromAccessError = async () => {
    if (!channel || !communityId) return;
    try {
      await channelAPI.joinText(communityId, channel.id);
      setAccessError(null);
      await refreshHistory();
    } catch (err) {
      const apiErr = err instanceof ApiError ? err : null;
      const code = apiErr?.code;
      if (code === "CHANNEL_PASSWORD_REQUIRED" || code === "INVALID_CHANNEL_PASSWORD") {
        const pw = window.prompt(`Enter password for #${channel.name}`);
        if (pw === null) return;
        try {
          await channelAPI.joinText(communityId, channel.id, pw);
          setAccessError(null);
          await refreshHistory();
        } catch (retryErr) {
          const r = retryErr instanceof ApiError ? retryErr : null;
          setAccessError(r?.message || i18n.t("couldNotJoin", { ns: "chat" }));
        }
      } else {
        setAccessError(apiErr?.message || i18n.t("couldNotJoin", { ns: "chat" }));
      }
    }
  };

  const myCommunityIds = React.useMemo(
    () => new Set((communities ?? []).map((s) => s.id)),
    [communities],
  );

  const handleAcceptInvite = async (msg: ChannelMessage) => {
    if (!msg.inviteCommunityId) return;
    setComposerError(null);
    try {
      await dmAPI.acceptCommunityInvite(msg.id);
    } catch (err) {
      const apiErr = err instanceof ApiError ? err : null;
      setComposerError(apiErr?.message || i18n.t("couldNotJoinCommunity", { ns: "chat" }));
      return;
    }
    try {
      await onGoToCommunity?.(msg.inviteCommunityId);
    } catch (err) {
      console.warn("[ChatView] post-accept navigation failed:", err);
    }
  };

  const handleDeclineInvite = async (msg: ChannelMessage) => {
    if (!msg.inviteCommunityId) return;
    setComposerError(null);
    try {
      await dmAPI.declineCommunityInvite(msg.id);
      setMessages((prev) => prev.filter((m) => m.id !== msg.id));
    } catch (err) {
      const apiErr = err instanceof ApiError ? err : null;
      setComposerError(apiErr?.message || "Couldn't decline the invite.");
    }
  };

  if (!channel && !dm) return null;

  const placeholder = dm
    ? i18n.t("messagePlaceholderDm", { ns: "chat", user: dm.name })
    : i18n.t("messagePlaceholder", { ns: "chat", channel: channel?.name ?? "" });

  const headerName = channel ? channel.name : dm!.name;

  return (
    <>
      <ChannelHeader
        channel={channel}
        dm={dm}
        blocked={dmBlocked}
        blockedBy={dmBlockedBy}
        membersOpen={membersOpen}
        searchActive={search.searchOpen}
        onToggleSearch={search.toggleSearch}
        onToggleMembers={onToggleMembers}
        onRemoveFriend={onRemoveFriend}
        onBlockUser={onBlockUser}
        onUnblockUser={() => {
          if (dm) void useBlockStore.getState().unblock(dm.friendId).catch(() => {});
        }}
      />
      {topSlot}
      {dm && !callMode && <CallJoinBanner friendId={dm.friendId} />}
      {search.searchOpen && (channel || dm) && (
        <div className="search-bar">
          <Icon name="search" size={14} />
          <input
            autoFocus
            type="text"
            placeholder={
              channel
                ? i18n.t("searchInChannel", { ns: "chat", channel: channel.name })
                : i18n.t("searchWithUser", { ns: "chat", user: dm!.name })
            }
            value={search.searchInput}
            onChange={(e) => search.setSearchInput(e.target.value)}
          />
          <button
            type="button"
            className="search-clear"
            onClick={search.toggleSearch}
            title={i18n.t("closeSearch", { ns: "chat" })}
          >
            <Icon name="x" size={12} />
          </button>
        </div>
      )}
      <div
        className="chat-body-wrap"
        style={callMode ? { display: "none" } : undefined}
      >
      <div
        className={"pull-refresh" + (pull.refreshing ? " refreshing" : "")}
        style={{
          opacity: pull.refreshing
            ? 1
            : Math.min(pull.pullDistance / PULL_THRESHOLD, 1),
          transform: `translateX(-50%) translateY(${
            44 - (pull.refreshing ? PULL_THRESHOLD : pull.pullDistance)
          }px) scale(${
            pull.refreshing
              ? 1
              : 0.6 + 0.4 * Math.min(pull.pullDistance / PULL_THRESHOLD, 1)
          })`,
        }}
      >
        <span
          className="pull-refresh-icon"
          style={
            pull.refreshing
              ? undefined
              : { transform: `rotate(${pull.pullDistance * 2.5}deg)` }
          }
        >
          <Icon name="refresh" size={15} />
        </span>
      </div>
      <div
        className={"chat-body" + (dragOver ? " drag-over" : "")}
        ref={bodyRef}
        onScroll={searchActive ? undefined : handleScroll}
        onTouchStart={pull.onPullTouchStart}
        onTouchMove={pull.onPullTouchMove}
        onTouchEnd={pull.onPullTouchEnd}
        onWheel={pull.onPullWheel}
        onDragOver={(e) => {
          if (e.dataTransfer.types.includes("Files")) {
            e.preventDefault();
            setDragOver(true);
          }
        }}
        onDragLeave={() => setDragOver(false)}
        onDrop={(e) => {
          if (e.dataTransfer.files.length > 0) {
            e.preventDefault();
            setDragOver(false);
            addFiles(e.dataTransfer.files);
          }
        }}
        style={{
          transform: `translateY(${-pull.pullDistance}px)`,
          transition:
            pull.pullStartYRef.current !== null
              ? "none"
              : "transform 0.3s cubic-bezier(0.22, 1, 0.36, 1)",
        }}
      >
        {searchActive ? (
          <div className="search-results">
            {search.searchLoading ? (
              <div className="empty-state">
                <span className="loading-spinner" />
              </div>
            ) : search.searchError ? (
              <div className="empty-state">
                <p>{search.searchError}</p>
              </div>
            ) : !dm && !blocksLoaded ? (
              <div className="msg-loader">
                <span className="loading-spinner" />
              </div>
            ) : search.searchResults.length === 0 ? (
              <div className="empty-state">
                <div className="icon-wrap">
                  <Icon name="search" size={22} />
                </div>
                <h3>{i18n.t("noResults", { ns: "chat" })}</h3>
                <p>{i18n.t("noResultsBody", { ns: "chat", query: search.searchQuery })}</p>
              </div>
            ) : (
              <>
                <div className="search-meta">
                  {i18n.t(search.searchTotal === 1 ? "resultsFor" : "resultsForPlural", {
                    ns: "chat",
                    count: search.searchTotal,
                    query: search.searchQuery,
                  })}
                </div>
                <SearchResults
                  results={search.searchResults}
                  currentUserId={currentUserId}
                  isDm={!!dm}
                  blockedIds={blockedIds}
                  nameOverrides={nameOverrides}
                  revealedResults={revealedResults}
                  onReveal={(id) => setRevealedResults((prev) => new Set(prev).add(id))}
                  onJump={handleJumpToMessage}
                />
              </>
            )}
          </div>
        ) : showInitialLoader ? (
          <div className="empty-state">
            <span className="loading-spinner" />
          </div>
        ) : accessError ? (
          <div className="empty-state">
            <div className="icon-wrap">
              <Icon name={isDm ? "user" : "hash"} size={22} />
            </div>
            <h3>
              {isDm ? "Can't open this conversation" : `Join #${headerName}`}
            </h3>
            <p>{accessError}</p>
            {isText && (
              <button
                type="button"
                className="btn primary"
                onClick={handleJoinFromAccessError}
              >
                Join channel
              </button>
            )}
          </div>
        ) : !dm && !blocksLoaded ? (
          <div className="msg-loader">
            <span className="loading-spinner" />
          </div>
        ) : messages.length === 0 ? (
          <div className="empty-state">
            <div className="icon-wrap">
              <Icon name="hash" size={22} />
            </div>
            <h3>{t("welcomeTitle", { name: headerName })}</h3>
            <p>{t("welcomeBody")}</p>
          </div>
        ) : (
          <MessageList
            messages={messages}
            loadingOlder={loadingOlder}
            currentUserId={currentUserId}
            isDm={!!dm}
            blockedIds={blockedIds}
            nameOverrides={nameOverrides}
            editingId={editing.editingId}
            editText={editing.editText}
            editRemovedIds={editRemovedIds}
            editFiles={editFiles}
            savingEdit={editing.savingEdit}
            highlightedMessageId={highlightedMessageId}
            canModerateMessages={canModerateMessages}
            myCommunityIds={myCommunityIds}
            receiptFor={receiptFor}
            onEditTextChange={editing.setEditText}
            onStartEdit={editing.startEdit}
            onCancelEdit={editing.cancelEdit}
            onSaveEdit={editing.saveEdit}
            onToggleRemoveAttachment={toggleRemoveAttachment}
            onAddEditFiles={addEditFiles}
            onRemoveEditFile={removeEditFile}
            onDelete={editing.setDeleteTarget}
            onAcceptInvite={handleAcceptInvite}
            onDeclineInvite={handleDeclineInvite}
            onOpenInviteCommunity={onGoToCommunity}
          />
        )}
      </div>
      </div>
      {!callMode && dm && (dmBlocked || dmBlockedBy) ? (
        <div className="composer composer-blocked">
          <Icon name="info" size={15} />
          <span>{dmBlocked ? t("blockedComposer") : t("blockedByComposer")}</span>
          {dmBlocked && (
            <button
              type="button"
              className="btn"
              onClick={() => void useBlockStore.getState().unblock(dm.friendId).catch(() => {})}
            >
              {t("unblockUser")}
            </button>
          )}
        </div>
      ) : !callMode && (
      <Composer
          composerRef={composerRef}
          fileInputRef={fileInputRef}
          text={text}
          placeholder={placeholder}
          sending={composer.sending}
          composerError={composerError}
          pendingFiles={pendingFiles}
          mentionQuery={mentionQuery}
          mentionSuggestions={mentionSuggestions}
          mentionIndex={mentionIndex}
          onTextChange={(value, caret) => {
            setText(value);
            if (composerError) setComposerError(null);
            updateMentionQuery(value, caret);
          }}
          onCaretChange={updateMentionQuery}
          onSubmit={composer.handleSend}
          onAddFiles={addFiles}
          onRemovePendingFile={removePendingFile}
          onMentionIndexChange={setMentionIndex}
          onApplyMention={applyMention}
          onCloseMentions={() => setMentionQuery(null)}
          onEditLastMessage={editing.editLastOwnMessage}
        />
      )}
      {editing.deleteTarget && (
        <ConfirmModal
          title={i18n.t("deleteMessage", { ns: "chat" })}
          subtitle={i18n.t("deleteMessageConfirm", { ns: "chat" })}
          confirmLabel={i18n.t("delete", { ns: "common" })}
          danger
          onCancel={() => editing.setDeleteTarget(null)}
          onConfirm={editing.confirmDelete}
        />
      )}
    </>
  );
};

export default ChatView;
