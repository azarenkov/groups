import React, { useEffect } from "react";
import { useTranslation } from "react-i18next";
import { colorForId, initialFor } from "../../../shared/lib/avatar";
import { displayUserName } from "../../../shared/lib/displayName";
import { useBlockStore } from "../../social/store/blockStore";
import { useFriendshipStore } from "../../social/store/friendshipStore";
import { usePresenceStore } from "../../social/store/presenceStore";
import { useUserNamesStore } from "../../social/store/userNamesStore";
import { useDmPreviewStore } from "../../dm/store/dmPreviewStore";
import type { DmPreview } from "../../dm/store/dmPreviewStore";
import type { DirectConversation } from "../../dm/types";
import { sortDirects } from "../../dm/lib/directOrder";

export function useDirectConversations(params: {
  currentUserId?: string;
  selectedDm: DirectConversation | null;
  setSelectedDm: React.Dispatch<React.SetStateAction<DirectConversation | null>>;
}) {
  const { currentUserId, selectedDm, setSelectedDm } = params;
  const { t: tSidebar } = useTranslation("sidebar");
  const friends = useFriendshipStore((s) => s.friends);
  const friendStatuses = useFriendshipStore((s) => s.statusById);
  const onlineUsers = usePresenceStore((s) => s.online);
  const nameOverrides = useUserNamesStore((s) => s.overrides);
  const dmPreviews = useDmPreviewStore((s) => s.byFriend);
  const blockedPeers = useBlockStore((s) => s.blocked);
  const blockedByPeers = useBlockStore((s) => s.blockedBy);

  const previewText = React.useCallback(
    (p: DmPreview | undefined) => {
      if (!p) return undefined;
      const body = p.callStatus
        ? tSidebar("previewCall")
        : p.inviteCommunityId
          ? tSidebar("previewInvite")
          : p.content ||
            (p.stickerEmoji
              ? tSidebar("previewSticker", { emoji: p.stickerEmoji })
              : typeof p.videoNoteDurationMs === "number"
              ? tSidebar("previewVideoNote")
              : typeof p.voiceDurationMs === "number"
              ? tSidebar("previewVoice")
              : p.hasAttachments
                ? tSidebar("previewAttachment")
                : "");
      return p.senderId === currentUserId
        ? `${tSidebar("previewYou")} ${body}`
        : body;
    },
    [tSidebar, currentUserId],
  );

  const acceptedFriends: DirectConversation[] = React.useMemo(() => {
    const items: DirectConversation[] = friends
      .filter((f) => f.status === "accepted")
      .map((f) => ({
        id: f.friendId,
        name: displayUserName(f.friendName),
        initials: initialFor(displayUserName(f.friendName)),
        color: colorForId(f.friendId),
        online: !!onlineUsers[f.friendId],
        unread: 0,
        lastMessage: previewText(dmPreviews[f.friendId]),
        lastAt: dmPreviews[f.friendId]?.createdAt,
        isFriend: true,
        friendship: "accepted" as const,
      }));
    const known = new Set(items.map((d) => d.id));
    const pendingById = new Map(
      friends.filter((f) => f.status !== "accepted").map((f) => [f.friendId, f] as const),
    );
    for (const [peerId, p] of Object.entries(dmPreviews)) {
      if (known.has(peerId)) continue;
      if (
        (blockedPeers[peerId] || blockedByPeers[peerId]) &&
        selectedDm?.id !== peerId
      )
        continue;
      const pending = pendingById.get(peerId);
      const rawName =
        nameOverrides[peerId] ??
        pending?.friendName ??
        p.peerName ??
        (selectedDm?.id === peerId ? selectedDm.name : undefined);
      if (!rawName) continue;
      const name = displayUserName(rawName);
      const status = friendStatuses[peerId];
      const accepted = status === "accepted";
      items.push({
        id: peerId,
        name,
        initials: initialFor(name),
        color: colorForId(peerId),
        online: accepted && !!onlineUsers[peerId],
        unread: 0,
        lastMessage: previewText(p),
        lastAt: p.createdAt,
        isFriend: accepted,
        friendship: accepted
          ? "accepted"
          : status === "pending_outgoing" || status === "pending_incoming"
            ? status
            : "none",
      });
    }
    return sortDirects(items);
  }, [
    friends,
    friendStatuses,
    onlineUsers,
    dmPreviews,
    previewText,
    nameOverrides,
    selectedDm,
    blockedPeers,
    blockedByPeers,
  ]);

  useEffect(() => {
    if (!selectedDm) return;
    const fresh = acceptedFriends.find((d) => d.id === selectedDm.id);
    if (!fresh) {
      const known = friendStatuses[selectedDm.id];
      if (known === "accepted") return;
      if (selectedDm.friendship !== "none" && selectedDm.friendship !== "pending_outgoing" && selectedDm.friendship !== "pending_incoming") {
        setSelectedDm(null);
        return;
      }
      const status =
        known === "pending_outgoing" || known === "pending_incoming" ? known : "none";
      if (status !== selectedDm.friendship) {
        setSelectedDm((prev) => (prev && prev.id === selectedDm.id ? { ...prev, friendship: status } : prev));
      }
      return;
    }
    if (
      fresh.name === selectedDm.name &&
      fresh.isFriend === selectedDm.isFriend &&
      fresh.friendship === selectedDm.friendship
    )
      return;
    setSelectedDm((prev) =>
      prev && prev.id === fresh.id
        ? { ...prev, name: fresh.name, initials: fresh.initials, isFriend: fresh.isFriend, friendship: fresh.friendship }
        : prev,
    );
  }, [acceptedFriends, selectedDm, friendStatuses, setSelectedDm]);

  const pendingCount = React.useMemo(
    () => Object.values(friendStatuses).filter((s) => s === "pending_incoming").length,
    [friendStatuses],
  );

  return {
    friends,
    acceptedFriends,
    pendingCount,
    onlineUsers,
    nameOverrides,
    dmPreviews,
  };
}
