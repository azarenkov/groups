import { callAPI } from "../api/call";
import { getLiveKitService } from "./livekit";

export async function connectToRoom(
  token: string,
  url: string,
  roomName: string,
  startMuted?: boolean,
): Promise<void> {
  const service = getLiveKitService();
  if (service.isConnected() || service.isConnectingNow()) {
    await service.disconnect().catch(() => {});
  }
  await service.connect(token, url, roomName, startMuted);
}

export const getAuthedUserId = async (): Promise<string | null> => {
  const { useAuthStore } = await import("../../auth/store/authStore");
  return useAuthStore.getState().user?.id ?? null;
};

export const getAuthedIdentity = async (): Promise<{
  userId: string | null;
  accessToken: string | null;
}> => {
  const { useAuthStore } = await import("../../auth/store/authStore");
  const state = useAuthStore.getState();
  return {
    userId: state.user?.id ?? null,
    accessToken: state.tokens?.accessToken ?? null,
  };
};

export const leaveAsInitiator = async (
  callId: string,
  accessToken: string | null,
): Promise<void> => {
  if (accessToken) {
    await callAPI.leaveWithToken(callId, accessToken).catch(() => {});
  } else {
    await callAPI.leave(callId).catch(() => {});
  }
};

export const awaitVoiceSettingsSyncedFor = async (
  userId: string | null,
): Promise<boolean> => {
  const { awaitVoiceSettingsSyncedFor: waitFor } = await import(
    "../store/voiceSettingsStore"
  );
  return waitFor(userId);
};
