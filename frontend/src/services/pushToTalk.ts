import { getLiveKitService } from "./livekit";
import { useVoiceSettingsStore } from "../store/voiceSettingsStore";

let initialized = false;
let holding = false;
let holdingCode: string | null = null;

const isTextEntryTarget = (target: EventTarget | null): boolean => {
  const el = target as HTMLElement | null;
  if (!el || typeof el.closest !== "function") return false;
  return (
    el.closest('input, textarea, select, [contenteditable="true"]') !== null ||
    el.isContentEditable
  );
};

const stopTransmitting = () => {
  holding = false;
  holdingCode = null;
  getLiveKitService().pttSetTransmitting(false);
};

const applyEnforcement = (enabled: boolean) => {
  holding = false;
  holdingCode = null;
  getLiveKitService().setPushToTalkEnforced(enabled);
};

export const initPushToTalk = (): void => {
  if (initialized) return;
  initialized = true;
  applyEnforcement(useVoiceSettingsStore.getState().pushToTalk);
  useVoiceSettingsStore.subscribe((state, prev) => {
    if (state.pushToTalk !== prev.pushToTalk) {
      applyEnforcement(state.pushToTalk);
      return;
    }
    if (state.pttKey !== prev.pttKey && holding) {
      stopTransmitting();
    }
  });
  window.addEventListener("keydown", (e) => {
    const { pushToTalk, pttKey } = useVoiceSettingsStore.getState();
    if (e.code !== pttKey) return;
    if (!pushToTalk) return;
    if (isTextEntryTarget(e.target)) return;
    e.preventDefault();
    if (holding) return;
    holding = true;
    holdingCode = e.code;
    getLiveKitService().pttSetTransmitting(true);
  });
  window.addEventListener("keyup", (e) => {
    if (!holding || e.code !== holdingCode) return;
    stopTransmitting();
  });
  window.addEventListener("blur", () => {
    if (!holding) return;
    stopTransmitting();
  });
};
