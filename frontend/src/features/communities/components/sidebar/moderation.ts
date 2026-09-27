import type { Community } from "../../../../shared/types/community";
import { channelAPI } from "../../api/channel";
import { moderatorDeafened } from "./permissions";

export const moderateMute = (
  community: Community,
  channelId: string,
  m: { id: string; isMuted?: boolean; mutedBy?: string },
) => {
  const moderatorMuted = !!m.isMuted && !!m.mutedBy && m.mutedBy !== m.id;
  const call = moderatorMuted
    ? channelAPI.unmute(community.id, channelId, m.id)
    : channelAPI.mute(community.id, channelId, m.id);
  call.catch((err) => console.warn("[sidebar] mute toggle failed:", err));
};

export const moderateDeafen = (
  community: Community,
  channelId: string,
  m: { id: string; deafenedBy?: string },
) => {
  const call = moderatorDeafened(m)
    ? channelAPI.undeafen(community.id, channelId, m.id)
    : channelAPI.deafen(community.id, channelId, m.id);
  call.catch((err) => console.warn("[sidebar] deafen toggle failed:", err));
};

export const moderateDisconnect = (
  community: Community,
  channelId: string,
  m: { id: string },
) => {
  channelAPI
    .disconnect(community.id, channelId, m.id)
    .catch((err) => console.warn("[sidebar] disconnect failed:", err));
};
