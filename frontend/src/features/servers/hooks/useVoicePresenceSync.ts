import { useEffect, useRef, useState } from "react";
import { serverAPI } from "../api/server";
import { ws } from "../../../shared/ws/websocket";
import { useChannelStore } from "../store/channelStore";
import { usePresenceStore } from "../../social/store/presenceStore";
import { useVoicePresenceStore } from "../../voice/store/voicePresenceStore";
import { useStreamViewersStore } from "../../voice/store/streamViewersStore";
import type { Server } from "../../../shared/types/server";

export function useVoicePresenceSync(params: {
  servers: Server[];
  hasSession: boolean;
}) {
  const { servers, hasSession } = params;
  const presenceSyncedRef = useRef<Set<string>>(new Set());
  const [presenceSyncTick, setPresenceSyncTick] = useState(0);

  useEffect(() => {
    return ws.onOpen(() => {
      presenceSyncedRef.current.clear();
      useVoicePresenceStore.getState().resetRosters();
      usePresenceStore.getState().clearAll();
      setPresenceSyncTick((n) => n + 1);
    });
  }, []);

  useEffect(() => {
    if (!hasSession) return;
    const synced = presenceSyncedRef.current;
    servers.forEach((s) => {
      if (synced.has(s.id)) return;
      synced.add(s.id);
      serverAPI
        .listVoicePresence(s.id)
        .then(({ items, streamViewers }) => {
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
            useChannelStore.getState().channelsByServer[s.id] || [];
          for (const c of knownChannels) {
            if (c.kind !== "voice") continue;
            if (snapshotChannels.has(c.id)) continue;
            useVoicePresenceStore.getState().clearChannel(c.id);
          }
          for (const [channelId, members] of Object.entries(groups)) {
            useVoicePresenceStore.getState().setMembers(channelId, members);
          }
          useStreamViewersStore.getState().resetForServer(s.id, streamViewers);
        })
        .catch((err) => {
          synced.delete(s.id);
          console.warn(
            "[voice-presence] initial sync failed for server",
            s.id,
            err,
          );
        });
    });
  }, [servers, hasSession, presenceSyncTick]);
}
