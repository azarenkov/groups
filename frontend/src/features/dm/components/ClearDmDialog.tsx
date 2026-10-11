import React from "react";
import { create } from "zustand";
import { useTranslation } from "react-i18next";
import ConfirmModal from "../../../shared/ui/ConfirmModal";
import { ws } from "../../../shared/ws/websocket";
import { useAuthStore } from "../../auth/store/authStore";
import { dmAPI } from "../api/dm";

interface ClearDmTarget {
  id: string;
  name: string;
}

interface ClearDmDialogState {
  target: ClearDmTarget | null;
  open: (target: ClearDmTarget) => void;
  close: () => void;
}

export const useClearDmDialog = create<ClearDmDialogState>((set) => ({
  target: null,
  open: (target) => set({ target }),
  close: () => set({ target: null }),
}));

export async function clearDmConversation(peerId: string, forEveryone: boolean): Promise<void> {
  const selfId = useAuthStore.getState().user?.id;
  const { clearedAt } = await dmAPI.clearConversation(peerId, forEveryone);
  if (!selfId || useAuthStore.getState().user?.id !== selfId) return;
  ws.deliver({
    type: "dm_conversation.cleared",
    data: {
      actorId: selfId,
      participants: [selfId, peerId],
      scope: forEveryone ? "everyone" : "me",
      clearedAt,
    },
  });
}

const ClearDmDialog: React.FC = () => {
  const { t } = useTranslation("sidebar");
  const target = useClearDmDialog((s) => s.target);
  const close = useClearDmDialog((s) => s.close);
  if (!target) return null;
  const run = (forEveryone: boolean) => {
    close();
    void clearDmConversation(target.id, forEveryone).catch((e) =>
      console.warn("[ClearDmDialog] clear failed:", e),
    );
  };
  return (
    <ConfirmModal
      title={t("clearChatTitle", { name: target.name })}
      subtitle={t("clearChatSub")}
      confirmLabel={t("clearChatEveryone", { name: target.name })}
      secondaryLabel={t("clearChatMe")}
      danger
      onCancel={close}
      onSecondary={() => run(false)}
      onConfirm={() => run(true)}
    />
  );
};

export default ClearDmDialog;
