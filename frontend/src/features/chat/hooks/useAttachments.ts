import React from "react";

import {
  maxAttachmentsPerMessage,
  maxAttachmentBytes,
  attachmentAPI,
} from "../api/attachment";
import type { PendingFile } from "../types";

type SetFiles = React.Dispatch<React.SetStateAction<PendingFile[]>>;

const pendingFor = (file: File): PendingFile => {
  const oversize = file.size > maxAttachmentBytes();
  return {
    localId: `${Date.now()}-${Math.random().toString(36).slice(2)}`,
    file,
    progress: 0,
    error: oversize || file.size === 0,
    oversize,
  };
};

export function useAttachments() {
  const [pendingFiles, setPendingFiles] = React.useState<PendingFile[]>([]);
  const [dragOver, setDragOver] = React.useState(false);
  const fileInputRef = React.useRef<HTMLInputElement>(null);
  const cancelledUploadsRef = React.useRef<Set<string>>(new Set());
  const pendingFilesRef = React.useRef<PendingFile[]>([]);
  pendingFilesRef.current = pendingFiles;
  const inFlightSendRef = React.useRef<Set<string>>(new Set());

  const startUpload = React.useCallback((p: PendingFile, setFiles: SetFiles) => {
    void attachmentAPI
      .upload(p.file, (fraction) => {
        setFiles((cur) =>
          cur.map((x) =>
            x.localId === p.localId ? { ...x, progress: fraction } : x,
          ),
        );
      })
      .then((attachment) => {
        if (cancelledUploadsRef.current.delete(p.localId)) {
          void attachmentAPI.delete(attachment.id).catch(() => {});
          return;
        }
        setFiles((cur) =>
          cur.map((x) =>
            x.localId === p.localId
              ? { ...x, attachment, progress: 1, oversize: false }
              : x,
          ),
        );
      })
      .catch(() => {
        cancelledUploadsRef.current.delete(p.localId);
        setFiles((cur) =>
          cur.map((x) =>
            x.localId === p.localId ? { ...x, error: true } : x,
          ),
        );
      });
  }, []);

  const addFiles = React.useCallback((incoming: File[] | FileList) => {
    const files = Array.from(incoming);
    if (files.length === 0) return;
    const room =
      maxAttachmentsPerMessage() - pendingFilesRef.current.length;
    const additions = files.slice(0, Math.max(0, room)).map(pendingFor);
    if (additions.length === 0) return;
    setPendingFiles((prev) => [...prev, ...additions]);
    for (const p of additions) {
      if (!p.error) startUpload(p, setPendingFiles);
    }
  }, [startUpload]);

  const discardPendingFile = React.useCallback((target: PendingFile) => {
    if (target.attachment) {
      void attachmentAPI.delete(target.attachment.id).catch(() => {});
    } else if (!target.error) {
      cancelledUploadsRef.current.add(target.localId);
    }
  }, []);

  React.useEffect(() => {
    const inFlight = inFlightSendRef.current;
    return () => {
      for (const p of pendingFilesRef.current) {
        if (!inFlight.has(p.localId)) discardPendingFile(p);
      }
    };
  }, [discardPendingFile]);

  const removePendingFile = (localId: string) =>
    setPendingFiles((prev) => {
      const target = prev.find((p) => p.localId === localId);
      if (target) discardPendingFile(target);
      return prev.filter((p) => p.localId !== localId);
    });

  const [editFiles, setEditFiles] = React.useState<PendingFile[]>([]);
  const [editRemovedIds, setEditRemovedIds] = React.useState<string[]>([]);
  const editFilesRef = React.useRef<PendingFile[]>([]);
  editFilesRef.current = editFiles;

  const addEditFiles = React.useCallback(
    (incoming: File[] | FileList, room: number) => {
      const files = Array.from(incoming);
      if (files.length === 0) return;
      const additions = files.slice(0, Math.max(0, room)).map(pendingFor);
      if (additions.length === 0) return;
      setEditFiles((prev) => [...prev, ...additions]);
      for (const p of additions) {
        if (!p.error) startUpload(p, setEditFiles);
      }
    },
    [startUpload],
  );

  const removeEditFile = React.useCallback(
    (localId: string) =>
      setEditFiles((prev) => {
        const target = prev.find((p) => p.localId === localId);
        if (target) discardPendingFile(target);
        return prev.filter((p) => p.localId !== localId);
      }),
    [discardPendingFile],
  );

  const discardEditFiles = React.useCallback(() => {
    setEditFiles((prev) => {
      for (const p of prev) {
        if (!inFlightSendRef.current.has(p.localId)) discardPendingFile(p);
      }
      return [];
    });
    setEditRemovedIds([]);
  }, [discardPendingFile]);

  const discardPendingFiles = React.useCallback(() => {
    setPendingFiles((prev) => {
      for (const p of prev) {
        if (!inFlightSendRef.current.has(p.localId)) discardPendingFile(p);
      }
      return [];
    });
  }, [discardPendingFile]);

  const toggleRemoveAttachment = React.useCallback((id: string) => {
    setEditRemovedIds((prev) =>
      prev.includes(id) ? prev.filter((x) => x !== id) : [...prev, id],
    );
  }, []);

  React.useEffect(() => {
    const inFlight = inFlightSendRef.current;
    return () => {
      for (const p of editFilesRef.current) {
        if (!inFlight.has(p.localId)) discardPendingFile(p);
      }
    };
  }, [discardPendingFile]);


  return {
    pendingFiles,
    discardPendingFile,
    pendingFilesRef,
    editFilesRef,
    setPendingFiles,
    dragOver,
    setDragOver,
    fileInputRef,
    cancelledUploadsRef,
    inFlightSendRef,
    addFiles,
    removePendingFile,
    editFiles,
    setEditFiles,
    editRemovedIds,
    setEditRemovedIds,
    addEditFiles,
    removeEditFile,
    discardEditFiles,
    discardPendingFiles,
    toggleRemoveAttachment,
  };
}
