import { create } from "zustand";

import { getLiveKitService } from "../lib/livekit";

interface SpeakingState {
  ids: Set<string>;
  apply: (ids: Set<string>) => void;
}

const sameSet = (a: Set<string>, b: Set<string>) => {
  if (a.size !== b.size) return false;
  for (const id of a) if (!b.has(id)) return false;
  return true;
};

export const useSpeakingStore = create<SpeakingState>()((set, get) => ({
  ids: new Set<string>(),
  apply: (ids) => {
    if (sameSet(get().ids, ids)) return;
    set({ ids });
  },
}));

export const useIsSpeaking = (userId: string | null | undefined): boolean =>
  useSpeakingStore((state) => (userId ? state.ids.has(userId) : false));

let started = false;

export function startSpeakingTracking(selfId: string | null): () => void {
  if (started) return () => {};
  started = true;
  const service = getLiveKitService();
  let remote = new Set<string>();
  let localSpeaking = false;

  const publish = () => {
    const ids = new Set(remote);
    if (localSpeaking && selfId) ids.add(selfId);
    useSpeakingStore.getState().apply(ids);
  };

  const offParticipants = service.subscribeParticipants((participants) => {
    remote = new Set(
      participants.filter((p) => p.isSpeaking).map((p) => p.identity),
    );
    publish();
  });
  const offLocal = service.subscribeLocalSpeaking((speaking) => {
    localSpeaking = speaking;
    publish();
  });

  return () => {
    started = false;
    offParticipants();
    offLocal();
    useSpeakingStore.getState().apply(new Set());
  };
}
