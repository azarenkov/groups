import { create } from "zustand";

import { getLiveKitService } from "../lib/livekit";

interface SpeakingState {
  ids: Set<string>;
  apply: (ids: Set<string>) => void;
}

export function mergeSpeaking(
  measured: Set<string>,
  tracked: Set<string>,
  reported: Set<string>,
  selfId: string | null,
  localSpeaking: boolean,
): Set<string> {
  const ids = new Set(measured);
  for (const id of reported) if (!tracked.has(id)) ids.add(id);
  if (localSpeaking && selfId) ids.add(selfId);
  return ids;
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
  let reported = new Set(
    service
      .getParticipants()
      .filter((p) => p.isSpeaking)
      .map((p) => p.identity),
  );
  let measured = new Set<string>();
  let tracked = new Set<string>();

  let localSpeaking = service.getLocalSpeaking();

  const publish = () => {
    useSpeakingStore
      .getState()
      .apply(mergeSpeaking(measured, tracked, reported, selfId, localSpeaking));
  };

  publish();

  const offParticipants = service.subscribeParticipants((participants) => {
    reported = new Set(
      participants.filter((p) => p.isSpeaking).map((p) => p.identity),
    );
    publish();
  });
  const offMeasured = service.subscribeRemoteSpeaking((state) => {
    measured = state.speaking;
    tracked = state.tracked;
    publish();
  });
  const offLocal = service.subscribeLocalSpeaking((speaking) => {
    localSpeaking = speaking;
    publish();
  });

  return () => {
    started = false;
    offParticipants();
    offMeasured();
    offLocal();
    useSpeakingStore.getState().apply(new Set());
  };
}
