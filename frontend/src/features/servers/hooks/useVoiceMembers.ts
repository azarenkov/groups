import React from "react";
import { colorForId, initialFor } from "../../../shared/lib/avatar";
import { useVoicePresenceStore } from "../../voice/store/voicePresenceStore";
import { useLocalDeafened } from "../../voice/hooks/useLocalDeafened";
import { useParticipants } from "../../voice/hooks/useParticipants";
import type { SidebarVoiceMember } from "../components/sidebar/types";

export function useVoiceMembers(params: {
  joinedChannelId: string | null;
  currentUserId?: string;
  currentUserName: string;
  nameOverrides: Record<string, string>;
}) {
  const { joinedChannelId, currentUserId, currentUserName, nameOverrides } =
    params;
  const voiceParticipants = useParticipants();
  const selfDeafened = useLocalDeafened();
  const voicePresenceByChannel = useVoicePresenceStore((s) => s.byChannel);

  const voiceMembersByChannel: Record<string, SidebarVoiceMember[]> =
    React.useMemo(() => {
      const out: Record<string, SidebarVoiceMember[]> = {};
      const selfId = currentUserId ?? null;
      const liveDeafened = new Set(
        voiceParticipants.filter((p) => p.isDeafened).map((p) => p.identity),
      );
      for (const [channelId, members] of Object.entries(voicePresenceByChannel)) {
        const live = channelId === joinedChannelId;
        out[channelId] = members.map((m) => ({
          id: m.id,
          name: m.name,
          initials: initialFor(m.name),
          color: colorForId(m.id),
          isMuted: m.isMuted,
          mutedBy: m.mutedBy,
          isDeafened:
            !!m.deafenedBy ||
            (live && (m.id === selfId ? selfDeafened : liveDeafened.has(m.id))),
          deafenedBy: m.deafenedBy,
          isSharingScreen: m.screenOn,
          isCameraOn: m.cameraOn,
        }));
      }
      return out;
    }, [
      voicePresenceByChannel,
      voiceParticipants,
      joinedChannelId,
      currentUserId,
      selfDeafened,
    ]);

  const voiceMembers: SidebarVoiceMember[] = joinedChannelId
    ? [
        {
          id: currentUserId || "you",
          name: currentUserName || "You",
          initials: initialFor(currentUserName),
          color: colorForId(currentUserId || "you"),
          ...(selfDeafened ? { isDeafened: true } : {}),
        },
        ...voiceParticipants.map((p) => {
          const name = nameOverrides[p.identity] ?? (p.name || p.identity);
          return {
            id: p.identity,
            name,
            initials: initialFor(name),
            color: colorForId(p.identity),
            ...(p.isDeafened ? { isDeafened: true } : {}),
          };
        }),
      ]
    : [];

  return {
    voiceMembers,
    voiceMembersByChannel,
    voicePresenceByChannel,
    selfDeafened,
  };
}
