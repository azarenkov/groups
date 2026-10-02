import React from "react";
import { ApiError } from "../../../shared/lib/apiError";
import type { ChannelMessage } from "../../../shared/types/channel";
import { MAX_CONTENT } from "../constants";
import type { useAttachments } from "./useAttachments";
import type { Messenger } from "./useMessenger";
import { isVoiceMessage } from "../lib/voice";

export function useMessageEditing(params: {
  messenger: Messenger | null;
  convId?: string;
  currentUserId?: string;
  attachments: ReturnType<typeof useAttachments>;
  messagesRef: React.MutableRefObject<ChannelMessage[]>;
  setMessages: React.Dispatch<React.SetStateAction<ChannelMessage[]>>;
}) {
  const {
    messenger,
    convId,
    currentUserId,
    attachments,
    messagesRef,
    setMessages,
  } = params;
  const { discardEditFiles } = attachments;
  const [editingId, setEditingId] = React.useState<string | null>(null);
  const [editText, setEditText] = React.useState("");
  const [savingEdit, setSavingEdit] = React.useState(false);
  const [deleteTarget, setDeleteTarget] = React.useState<ChannelMessage | null>(
    null,
  );

  React.useEffect(() => {
    setEditingId(null);
    setEditText("");
    setDeleteTarget(null);
    discardEditFiles();
  }, [convId, discardEditFiles]);

  const startEdit = React.useCallback(
    (msg: ChannelMessage) => {
      discardEditFiles();
      setEditingId(msg.id);
      setEditText(msg.content);
    },
    [discardEditFiles],
  );

  const editLastOwnMessage = React.useCallback((): boolean => {
    const last = [...messagesRef.current]
      .reverse()
      .find(
        (m) =>
          m.authorId === currentUserId &&
          !m.callStatus &&
          !m.inviteCommunityId &&
          !m.forwardedFrom &&
          !m.system &&
          !m.sticker &&
          !isVoiceMessage(m),
      );
    if (!last) return false;
    startEdit(last);
    return true;
  }, [currentUserId, startEdit, messagesRef]);

  const cancelEdit = () => {
    if (savingEdit) return;
    discardEditFiles();
    setEditingId(null);
    setEditText("");
  };

  const saveEdit = async (msg: ChannelMessage) => {
    if (!messenger || savingEdit) return;
    const {
      editFiles,
      editFilesRef,
      editRemovedIds,
      setEditFiles,
      setEditRemovedIds,
      discardPendingFile,
      inFlightSendRef,
    } = attachments;
    const v = editText.trim();
    const currentIds = (msg.attachments ?? []).map((a) => a.id);
    const removeIds = editRemovedIds.filter((id) => currentIds.includes(id));
    const readyAdds = editFiles.filter((p) => p.attachment);
    const addIds = readyAdds.map((p) => p.attachment!.id);
    if (editFiles.some((p) => !p.attachment && !p.error)) return;
    const finalCount = currentIds.length - removeIds.length + addIds.length;
    if (!v && finalCount === 0) return;
    if (v === msg.content && removeIds.length === 0 && addIds.length === 0) {
      cancelEdit();
      return;
    }
    if (v.length > MAX_CONTENT) return;
    setSavingEdit(true);
    const sentLocalIds = new Set(readyAdds.map((p) => p.localId));
    for (const id of sentLocalIds) inFlightSendRef.current.add(id);
    let linked = false;
    try {
      await messenger.editMessage(msg.id, v, addIds, removeIds);
      linked = true;
      const editedAt = new Date().toISOString();
      const edited = (msg.attachments ?? [])
        .filter((a) => !removeIds.includes(a.id))
        .concat(readyAdds.map((p) => p.attachment!));
      setMessages((prev) =>
        prev.map((m) =>
          m.id === msg.id
            ? { ...m, content: v, editedAt, attachments: edited }
            : m,
        ),
      );
      for (const p of editFilesRef.current) {
        if (!sentLocalIds.has(p.localId)) discardPendingFile(p);
      }
      setEditFiles([]);
      setEditRemovedIds([]);
      setEditingId(null);
      setEditText("");
    } catch (err) {
      const apiErr = err instanceof ApiError ? err : null;
      if (
        apiErr?.code === "MESSAGE_CONTENT_UNCHANGED" ||
        apiErr?.code === "DM_CONTENT_UNCHANGED"
      ) {
        setSavingEdit(false);
        cancelEdit();
        return;
      }
      console.warn("[ChatView] edit failed:", err);
    } finally {
      for (const id of sentLocalIds) inFlightSendRef.current.delete(id);
      if (!linked) {
        for (const p of readyAdds) {
          if (!editFilesRef.current.some((x) => x.localId === p.localId)) {
            discardPendingFile(p);
          }
        }
      }
      setSavingEdit(false);
    }
  };

  const deletingRef = React.useRef(new Set<string>());
  const confirmDelete = async (scope: "everyone" | "me") => {
    if (!deleteTarget || !messenger || deletingRef.current.has(deleteTarget.id)) return;
    const target = deleteTarget;
    deletingRef.current.add(target.id);
    setDeleteTarget(null);
    try {
      if (scope === "everyone") {
        await messenger.deleteMessage(target.id);
      } else {
        await messenger.hideMessage(target.id);
      }
      setMessages((prev) => prev.filter((m) => m.id !== target.id));
    } catch (err) {
      console.warn("[ChatView] delete failed:", err);
    } finally {
      deletingRef.current.delete(target.id);
    }
  };

  return {
    editingId,
    editText,
    setEditText,
    savingEdit,
    deleteTarget,
    setDeleteTarget,
    startEdit,
    editLastOwnMessage,
    cancelEdit,
    saveEdit,
    confirmDelete,
  };
}
