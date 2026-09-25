import { useEffect, useRef, useState } from "react";
import { communityAPI } from "../api/community";
import { ws } from "../../../shared/ws/websocket";
import { useChannelStore } from "../store/channelStore";
import { usePresenceStore } from "../../social/store/presenceStore";
import { useVoicePresenceStore } from "../../voice/store/voicePresenceStore";
import { useStreamViewersStore } from "../../voice/store/streamViewersStore";
import { useVoiceSessionStore } from "../../voice/store/voiceSessionStore";
import type { Community } from "../../../shared/types/community";

export function useVoicePresenceSync(params: {
  communities: Community[];
  hasSession: boolean;
}) {
  const { communities, hasSession } = params;
  const presenceSyncedRef = useRef<Set<string>>(new Set());
  const [presenceSyncTick, setPresenceSyncTick] = useState(0);

  useEffect(() => {
    return ws.onOpen(() => {
      presenceSyncedRef.current.clear();
      useVoicePresenceStore.getState().resetRosters();
      useVoiceSessionStore.getState().clearAll();
      usePresenceStore.getState().clearAll();
      setPresenceSyncTick((n) => n + 1);
    });
  }, []);

  useEffect(() => {
    if (!hasSession) return;
    const synced = presenceSyncedRef.current;
    communities.forEach((s) => {
      if (synced.has(s.id)) return;
      synced.add(s.id);
      communityAPI
        .listVoicePresence(s.id)
        .then(({ items, streamViewers, sessions }) => {
          const groups: Record<
            string,
            {
              id: string;
              name: string;
              isMuted: boolean;
              mutedBy?: string;
              deafenedBy?: string;
              cameraOn: boolean;
              screenOn: boolean;
            }[]
          > = {};
          for (const item of items) {
            (groups[item.channelId] ||= []).push({
              id: item.userId,
              name: item.userName,
              isMuted: item.isMuted,
              mutedBy: item.mutedBy ?? undefined,
              deafenedBy: item.deafenedBy ?? undefined,
              cameraOn: item.cameraOn,
              screenOn: item.screenOn,
            });
          }
          const snapshotChannels = new Set(Object.keys(groups));
          const knownChannels =
            useChannelStore.getState().channelsByCommunity[s.id] || [];
          for (const c of knownChannels) {
            if (c.kind !== "voice") continue;
            if (snapshotChannels.has(c.id)) continue;
            useVoicePresenceStore.getState().clearChannel(c.id);
          }
          for (const [channelId, members] of Object.entries(groups)) {
            useVoicePresenceStore.getState().setMembers(channelId, members);
          }
          useStreamViewersStore.getState().resetForCommunity(s.id, streamViewers);
          useVoiceSessionStore.getState().replaceForChannels(
            [
              ...knownChannels.filter((c) => c.kind === "voice").map((c) => c.id),
              ...snapshotChannels,
            ],
            sessions,
          );
        })
        .catch((err) => {
          synced.delete(s.id);
          console.warn(
            "[voice-presence] initial sync failed for community",
            s.id,
            err,
          );
        });
    });
  }, [communities, hasSession, presenceSyncTick]);
}
