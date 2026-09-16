import type { Server } from "../../../../shared/types/server";
import { channelAPI } from "../../api/channel";
import { moderatorDeafened } from "./permissions";

export const moderateMute = (
  server: Server,
  channelId: string,
  m: { id: string; isMuted?: boolean; mutedBy?: string },
) => {
  const moderatorMuted = !!m.isMuted && !!m.mutedBy && m.mutedBy !== m.id;
  const call = moderatorMuted
    ? channelAPI.unmute(server.id, channelId, m.id)
    : channelAPI.mute(server.id, channelId, m.id);
  call.catch((err) => console.warn("[sidebar] mute toggle failed:", err));
};

export const moderateDeafen = (
  server: Server,
  channelId: string,
  m: { id: string; deafenedBy?: string },
) => {
  const call = moderatorDeafened(m)
    ? channelAPI.undeafen(server.id, channelId, m.id)
    : channelAPI.deafen(server.id, channelId, m.id);
  call.catch((err) => console.warn("[sidebar] deafen toggle failed:", err));
};
