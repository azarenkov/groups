import React from "react";
import { userAPI } from "../../social/api/user";
import { useReadReceiptStore } from "../store/readReceiptStore";
import type { ChannelMessage } from "../../../shared/types/channel";

export function useReadReceipts(params: {
  channelId?: string;
  dmFriendId?: string;
}) {
  const { channelId, dmFriendId } = params;
  const dmPeerReadAt = useReadReceiptStore((s) =>
    dmFriendId ? s.dmPeerReadAt[dmFriendId] : undefined,
  );
  const channelReads = useReadReceiptStore((s) =>
    channelId ? s.channelReads[channelId] : undefined,
  );

  React.useEffect(() => {
    if (!dmFriendId) return;
    userAPI
      .getDmReadState(dmFriendId)
      .then((r) =>
        useReadReceiptStore.getState().seedDmPeerReadAt(dmFriendId, r.readAt),
      )
      .catch(() => {});
  }, [dmFriendId]);

  React.useEffect(() => {
    if (!channelId) return;
    const seed = () =>
      userAPI
        .getChannelReadStates(channelId)
        .then((r) =>
          useReadReceiptStore.getState().seedChannelReads(channelId, r.members),
        )
        .catch(() => {});
    void seed();
    const interval = setInterval(seed, 60_000);
    return () => clearInterval(interval);
  }, [channelId]);

  const reseedChannelReads = React.useCallback(() => {
    if (!channelId) return;
    userAPI
      .getChannelReadStates(channelId)
      .then((r) =>
        useReadReceiptStore.getState().seedChannelReads(channelId, r.members),
      )
      .catch(() => {});
  }, [channelId]);

  const receiptFor = (
    m: ChannelMessage,
    mine: boolean,
  ): "sent" | "read" | null => {
    if (!mine || m.callStatus || m.inviteServerId) return null;
    const created = new Date(m.createdAt).getTime();
    if (dmFriendId) {
      return dmPeerReadAt && new Date(dmPeerReadAt).getTime() >= created
        ? "read"
        : "sent";
    }
    if (channelId) {
      if (!channelReads) return "sent";
      const members = Object.keys(channelReads);
      if (members.length === 0) return "sent";
      const all = members.every((uid) => {
        const r = channelReads[uid];
        return r != null && new Date(r).getTime() >= created;
      });
      return all ? "read" : "sent";
    }
    return null;
  };

  return { reseedChannelReads, receiptFor };
}
