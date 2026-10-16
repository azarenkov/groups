import { useAuthStore } from "../../auth/store/authStore";
import { getLiveKitService } from "../lib/livekit";
import { installMediaStateFlushOnUnload, syncMediaState } from "../lib/mediaStateSync";
import { isRecoverableDisconnect } from "../lib/reconnect";
import { RECONNECT_WINDOW_MS, forgetVoiceSession, installVoiceSessionTouch, rememberVoiceSession } from "../lib/sessionHint";
import { useCallStore } from "./callStore";

const RETRY_EVERY_MS = 3000;

let installed = false;

async function recoverCall(callId: string): Promise<void> {
  const deadline = Date.now() + RECONNECT_WINDOW_MS;
  useCallStore.setState({ reconnecting: true });
  try {
    while (useCallStore.getState().activeCallId === callId) {
      if ((await useCallStore.getState().rehydrate()) !== "unreachable") return;
      if (Date.now() >= deadline) {
        await useCallStore.getState().leave();
        return;
      }
      await new Promise((resolve) => setTimeout(resolve, RETRY_EVERY_MS));
    }
  } finally {
    useCallStore.setState({ reconnecting: false });
  }
}

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
    const { activeCallId } = useCallStore.getState();
    if (!activeCallId || roomName !== activeCallId || !isRecoverableDisconnect(reason)) return;
    void recoverCall(activeCallId);
  });
}
