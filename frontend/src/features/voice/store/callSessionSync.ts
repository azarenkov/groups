import { useAuthStore } from "../../auth/store/authStore";
import { getLiveKitService } from "../lib/livekit";
import { installMediaStateFlushOnUnload, syncMediaState } from "../lib/mediaStateSync";
import { isRecoverableDisconnect } from "../lib/reconnect";
import { forgetVoiceSession, installVoiceSessionTouch, rememberVoiceSession } from "../lib/sessionHint";
import { useCallStore } from "./callStore";

let installed = false;

export function installCallSessionSync(): void {
  if (installed) return;
  installed = true;
  installVoiceSessionTouch();
  installMediaStateFlushOnUnload();
  useCallStore.subscribe((state, prev) => {
    if (state.activeCallId !== prev.activeCallId) {
      if (prev.activeCallId) forgetVoiceSession({ kind: "call", callId: prev.activeCallId });
      const userId = useAuthStore.getState().user?.id;
      if (state.activeCallId && userId) rememberVoiceSession(userId, { kind: "call", callId: state.activeCallId });
    }
    const mediaChanged =
      state.micMuted !== prev.micMuted ||
      state.deafened !== prev.deafened ||
      state.micMutedBeforeDeafen !== prev.micMutedBeforeDeafen;
    if (state.activeCallId && (mediaChanged || state.activeCallId !== prev.activeCallId)) {
      syncMediaState({ micMuted: state.deafened ? state.micMutedBeforeDeafen : state.micMuted, deafened: state.deafened });
    }
  });
  const service = getLiveKitService();
  service.subscribeReconnecting((reconnecting) => {
    const { activeCallId } = useCallStore.getState();
    const ours = !!activeCallId && service.getActiveRoomName() === activeCallId;
    if (!reconnecting || ours) useCallStore.setState({ reconnecting });
  });
  service.subscribeConnectionLost((roomName, reason) => {
    const call = useCallStore.getState();
    if (!call.activeCallId || roomName !== call.activeCallId || !isRecoverableDisconnect(reason)) return;
    void call.rehydrate();
  });
}
