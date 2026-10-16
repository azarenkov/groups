import { useAuthStore } from "../../auth/store/authStore";
import { getLiveKitService } from "../lib/livekit";
import { installMediaStateFlushOnUnload, syncMediaState } from "../lib/mediaStateSync";
import { isRecoverableDisconnect } from "../lib/reconnect";
import { RECONNECT_WINDOW_MS, forgetVoiceSession, installVoiceSessionTouch, rememberVoiceSession } from "../lib/sessionHint";
import { useCallStore } from "./callStore";

const RETRY_EVERY_MS = 3000;

let installed = false;
let resuming: Promise<void> | null = null;

export function resumeCall(): Promise<void> {
  if (resuming) return resuming;
  resuming = (async () => {
    const deadline = Date.now() + RECONNECT_WINDOW_MS;
    try {
      while ((await useCallStore.getState().rehydrate()) === "unreachable") {
        if (Date.now() >= deadline) {
          if (useCallStore.getState().activeCallId) await useCallStore.getState().leave();
          return;
        }
        await new Promise((resolve) => setTimeout(resolve, RETRY_EVERY_MS));
      }
    } finally {
      useCallStore.setState({ reconnecting: false });
      resuming = null;
    }
  })();
  return resuming;
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
    useCallStore.setState({ reconnecting: true });
    void resumeCall();
  });
}
