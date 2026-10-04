import React from "react";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router-dom";
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
import { useRolesStore, userPermissions } from "../../communities/store/rolesStore";
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
import { previewOf } from "../lib/replies";
import { messageLink } from "../lib/messageLink";
import type { MessageLinkTarget } from "../lib/messageLink";
import { copyTextToClipboard } from "../../../shared/lib/clipboard";
import { publicAppOrigin } from "../../../shared/config/runtime";
import { useMessageSearch } from "../hooks/useMessageSearch";
import { useMessenger } from "../hooks/useMessenger";
import { useReadReceipts } from "../hooks/useReadReceipts";
import { usePullToRefresh } from "../hooks/usePullToRefresh";
import { VOICE_LISTENED_EVENT, type SharedScope } from "../api/shared";
import { usePinnedMessages } from "../hooks/usePinnedMessages";
import PinnedBar from "./PinnedBar";
import type { ChannelMeta, DmMeta } from "../types";
import { openForward } from "../store/forwardModalStore";
import { canDeleteForEveryone } from "../lib/messageMenu";

interface ChatViewProps {
  channel?: ChannelMeta;
  communityId?: string;
  currentUserId?: string;
  dm?: DmMeta;
  membersOpen?: boolean;
  communities?: Community[];
  onToggleMembers?: () => void;
  sharedOpen?: boolean;
  onToggleShared?: () => void;
  onRemoveFriend?: () => void;
  onBlockUser?: () => void;
  onGoToCommunity?: (communityId: string) => void | Promise<void>;
  topSlot?: React.ReactNode;
  callMode?: boolean;
  openMessageId?: string;
  onOpenMessageHandled?: () => void;
}

const ChatView: React.FC<ChatViewProps> = ({
  channel,
  communityId,
  currentUserId,
  dm,
  membersOpen,
  communities,
  onToggleMembers,
  sharedOpen,
  onToggleShared,
  onRemoveFriend,
  onBlockUser,
  onGoToCommunity,
  topSlot,
  callMode,
  openMessageId,
  onOpenMessageHandled,
}) => {
  const { t } = useTranslation("chat");
  const navigate = useNavigate();
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
    jumpToMessageId,
  } = useMessages({
    messenger,
    convId,
    isText,
    isDm,
    nameSyncTick,
    reseedChannelReads,
    currentUserId,
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

  React.useEffect(() => {
    const onListened = (event: Event) => {
      const id = (event as CustomEvent<{ messageId?: string }>).detail?.messageId;
      if (!id) return;
      setMessages((prev) => (prev.some((m) => m.id === id) ? prev.map((m) => (m.id === id ? { ...m, voiceListened: true } : m)) : prev));
    };
    window.addEventListener(VOICE_LISTENED_EVENT, onListened);
    return () => window.removeEventListener(VOICE_LISTENED_EVENT, onListened);
  }, [setMessages]);

  const onOpenHandledRef = React.useRef(onOpenMessageHandled);
  onOpenHandledRef.current = onOpenMessageHandled;
  const { closeSearch } = search;
  React.useEffect(() => {
    if (!openMessageId || showInitialLoader) return;
    closeSearch();
    void jumpToMessageId(openMessageId);
    onOpenHandledRef.current?.();
  }, [openMessageId, showInitialLoader, jumpToMessageId, closeSearch]);

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
      userPermissions(rolesData, currentUserId).includes("manageMessages"));

  const pinScope = React.useMemo<SharedScope | null>(
    () =>
      dmFriendId
        ? { kind: "dm", friendId: dmFriendId }
        : channelId && communityId
          ? { kind: "channel", communityId, channelId }
          : null,
    [dmFriendId, channelId, communityId],
  );
  const pinned = usePinnedMessages(pinScope, currentUserId);
  const canPin = dm ? !dmBlocked && !dmBlockedBy : canModerateMessages;
  const [pinError, setPinError] = React.useState<string | null>(null);
  React.useEffect(() => {
    if (!pinError) return;
    const timer = setTimeout(() => setPinError(null), 4000);
    return () => clearTimeout(timer);
  }, [pinError]);
  React.useEffect(() => {
    setPinError(null);
  }, [convId]);
  const togglePin = (messageId: string, pin: boolean) => {
    void pinned.setPinned(messageId, pin).catch((e) => {
      const code = e instanceof ApiError ? e.code : undefined;
      setPinError(
        i18n.t(code === "PIN_LIMIT_REACHED" ? "pins.limit" : pin ? "pins.pinFailed" : "pins.unpinFailed", {
          ns: "chat",
        }),
      );
    });
  };

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
        sharedOpen={sharedOpen}
        onToggleShared={onToggleShared}
        onRemoveFriend={onRemoveFriend}
        onBlockUser={onBlockUser}
        onUnblockUser={() => {
          if (dm) void useBlockStore.getState().unblock(dm.friendId).catch(() => {});
        }}
      />
      {topSlot}
      {dm && !callMode && <CallJoinBanner friendId={dm.friendId} />}
      {!callMode && (
        <PinnedBar
          pins={pinned.pins}
          currentUserId={currentUserId}
          canUnpin={canPin}
          onJump={(id) => void jumpToMessageId(id)}
          onUnpin={(id) => togglePin(id, false)}
        />
      )}
      {pinError && (
        <div className="pin-error" role="alert">
          {pinError}
        </div>
      )}
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
      <div ref={pull.indicatorRef} className="pull-refresh">
        <span className="pull-refresh-icon">
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
        onTouchCancel={pull.onPullTouchCancel}
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
      >
        {searchActive ? (
          <div className="search-results">
            {search.searchLoading ? (
              <div className="empty-state">
                <span className="loading-spinner" />
              </div>
            ) : search.searchError ? (
              <div className="empty-state">
                <p className="error-text">{search.searchError}</p>
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
            <p className="error-text">{accessError}</p>
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
            onVoiceListened={async (message) => {
              const setListened = (value: boolean) =>
                setMessages((prev) =>
                  prev.map((m) => (m.id === message.id ? { ...m, voiceListened: value } : m)),
                );
              if (!messenger) throw new Error("no conversation");
              setListened(true);
              try {
                await messenger.markVoiceListened(message.id);
              } catch (e) {
                console.warn("[ChatView] mark voice listened failed:", e);
                setListened(false);
                throw e;
              }
            }}
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
            myCommunityIds={myCommunityIds}
            receiptFor={receiptFor}
            onEditTextChange={editing.setEditText}
            onStartEdit={editing.startEdit}
            onCancelEdit={editing.cancelEdit}
            onSaveEdit={editing.saveEdit}
            onToggleRemoveAttachment={toggleRemoveAttachment}
            onAddEditFiles={addEditFiles}
            onRemoveEditFile={removeEditFile}
            onReply={(message, authorName) => {
                composer.setReplyTarget(previewOf(message, authorName));
                composerRef.current?.focus();
              }}
              onJumpToMessage={(id) => void jumpToMessageId(id)}
              onCopyLink={(message) => {
                const target: MessageLinkTarget | null =
                  isDm && dmFriendId && currentUserId
                    ? { kind: "dm", participants: [currentUserId, dmFriendId], messageId: message.id }
                    : channelId && communityId
                      ? { kind: "channel", communityId, channelId, messageId: message.id }
                      : null;
                if (!target) return;
                void copyTextToClipboard(messageLink(publicAppOrigin(), target)).catch(
                  (e) => console.warn("[ChatView] copy link failed:", e),
                );
              }}
              onForward={(message) => {
                if (isDm && dmFriendId) {
                  openForward({ kind: "dm", friendId: dmFriendId, messageId: message.id });
                } else if (channelId && communityId) {
                  openForward({ kind: "channel", communityId, channelId, messageId: message.id });
                }
              }}
              onOpenForwardSource={(message) => {
                const origin = message.forwardedFrom;
                if (!origin || origin.source.kind !== "channel") return;
                const link = new URL(
                  messageLink(window.location.origin, {
                    kind: "channel",
                    communityId: origin.source.communityId,
                    channelId: origin.source.channelId,
                    messageId: origin.messageId,
                  }),
                );
                navigate(link.pathname + link.search);
              }}
              onDelete={editing.setDeleteTarget}
            onAcceptInvite={handleAcceptInvite}
            onDeclineInvite={handleDeclineInvite}
            onOpenInviteCommunity={onGoToCommunity}
            pinnedIds={pinned.pinnedIds}
            onTogglePin={canPin ? (message, pin) => togglePin(message.id, pin) : undefined}
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
          replyTarget={composer.replyTarget}
          onCancelReply={() => composer.setReplyTarget(null)}
          onSendVoice={composer.sendVoice}
          onSendSticker={(sticker) => void composer.sendSticker(sticker)}
          onVoiceError={composer.setComposerError}
          conversationKey={convId}
        />
      )}
      {editing.deleteTarget &&
        (canDeleteForEveryone(
          editing.deleteTarget,
          editing.deleteTarget.authorId === currentUserId,
          canModerateMessages,
        ) ? (
          <ConfirmModal
            title={i18n.t("deleteMessage", { ns: "chat" })}
            subtitle={i18n.t("deleteMessageChoose", { ns: "chat" })}
            secondaryLabel={i18n.t("deleteForMe", { ns: "chat" })}
            confirmLabel={i18n.t("deleteForEveryone", { ns: "chat" })}
            danger
            onCancel={() => editing.setDeleteTarget(null)}
            onSecondary={() => void editing.confirmDelete("me")}
            onConfirm={() => void editing.confirmDelete("everyone")}
          />
        ) : (
          <ConfirmModal
            title={i18n.t("deleteMessage", { ns: "chat" })}
            subtitle={i18n.t("deleteForMeConfirm", { ns: "chat" })}
            confirmLabel={i18n.t("deleteForMe", { ns: "chat" })}
            danger
            onCancel={() => editing.setDeleteTarget(null)}
            onConfirm={() => void editing.confirmDelete("me")}
          />
        ))}
    </>
  );
};

export default ChatView;
