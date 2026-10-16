import type { Channel } from "../../../shared/types/channel";
import type { ChatLocation } from "../../navigation/lib/appLocation";

export function channelServesLink(channel: Pick<Channel, "id" | "kind">, link: Extract<ChatLocation, { kind: "channel" }>): boolean {
  return channel.id === link.channelId && (!link.messageId || channel.kind === "text");
}
