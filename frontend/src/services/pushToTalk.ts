import { getLiveKitService } from "./livekit";
import { useVoiceSettingsStore } from "../store/voiceSettingsStore";

const PTT_CODE = "Space";

let initialized = false;
let holding = false;

const isEditableTarget = (target: EventTarget | null): boolean => {
  const el = target as HTMLElement | null;
  if (!el || !el.tagName) return false;
  const tag = el.tagName;
  return (
    tag === "INPUT" ||
    tag === "TEXTAREA" ||
    tag === "SELECT" ||
    el.isContentEditable
  );
};

const applyEnforcement = (enabled: boolean) => {
  holding = false;
  getLiveKitService().setPushToTalkEnforced(enabled);
};

export const initPushToTalk = (): void => {
  if (initialized) return;
  initialized = true;
  applyEnforcement(useVoiceSettingsStore.getState().pushToTalk);
  useVoiceSettingsStore.subscribe((state, prev) => {
    if (state.pushToTalk !== prev.pushToTalk) {
      applyEnforcement(state.pushToTalk);
    }
  });
  window.addEventListener("keydown", (e) => {
    if (e.code !== PTT_CODE) return;
    if (!useVoiceSettingsStore.getState().pushToTalk) return;
    if (isEditableTarget(e.target)) return;
    e.preventDefault();
    if (holding) return;
    holding = true;
    getLiveKitService().pttSetTransmitting(true);
  });
  window.addEventListener("keyup", (e) => {
    if (e.code !== PTT_CODE) return;
    if (!holding) return;
    holding = false;
    if (!useVoiceSettingsStore.getState().pushToTalk) return;
    getLiveKitService().pttSetTransmitting(false);
  });
  window.addEventListener("blur", () => {
    if (!holding) return;
    holding = false;
    if (useVoiceSettingsStore.getState().pushToTalk) {
      getLiveKitService().pttSetTransmitting(false);
    }
  });
};
