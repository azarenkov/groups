import { CONNECTED_CHIME, DISCONNECTED_CHIME, PEER_JOINED_CHIME, PEER_LEFT_CHIME, type ChimeNote } from "../../../shared/lib/chime";
import { playVoiceCue } from "../../../shared/lib/notificationSound";
import { voiceCueAllowed } from "../../settings/store/notificationSettingsStore";
import { idleVoiceCues, nextVoiceCue, type VoiceCue, type VoiceCueEvent, type VoiceCueState } from "./voiceCues";

const CHIMES: Record<VoiceCue, ChimeNote[]> = {
  connected: CONNECTED_CHIME,
  disconnected: DISCONNECTED_CHIME,
  peerJoined: PEER_JOINED_CHIME,
  peerLeft: PEER_LEFT_CHIME,
};

let state: VoiceCueState = idleVoiceCues;

export function reportVoiceCue(event: VoiceCueEvent): void {
  const next = nextVoiceCue(state, event, Date.now());
  state = next.state;
  if (next.cue && voiceCueAllowed()) playVoiceCue(CHIMES[next.cue]);
}

export function resetVoiceCues(): void {
  state = idleVoiceCues;
}
