import React from "react";

import {
  maxAttachmentsPerMessage,
  maxAttachmentBytes,
  attachmentAPI,
} from "../api/attachment";
import type { PendingFile } from "../types";
import { useEntitlementsStore } from "../../subscription/store/entitlementsStore";

type SetFiles = React.Dispatch<React.SetStateAction<PendingFile[]>>;

const UPGRADE_RETRY_DELAYS_MS = [2000, 5000, 10000];

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

  const startUpload = React.useCallback((p: PendingFile, setFiles: SetFiles, attempt = -1) => {
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
        const delay = attempt >= 0 ? UPGRADE_RETRY_DELAYS_MS[attempt] : undefined;
        if (delay !== undefined && !cancelledUploadsRef.current.has(p.localId)) {
          setFiles((cur) =>
            cur.map((x) => (x.localId === p.localId ? { ...x, progress: 0 } : x)),
          );
          setTimeout(() => {
            if (cancelledUploadsRef.current.delete(p.localId)) return;
            startUploadRef.current(p, setFiles, attempt + 1);
          }, delay);
          return;
        }
        cancelledUploadsRef.current.delete(p.localId);
        setFiles((cur) =>
          cur.map((x) =>
            x.localId === p.localId ? { ...x, error: true } : x,
          ),
        );
      });
  }, []);
  const startUploadRef = React.useRef(startUpload);
  startUploadRef.current = startUpload;

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

  const tier = useEntitlementsStore((s) => s.tier);
  React.useEffect(() => {
    const limit = maxAttachmentBytes();
    const retry = (files: PendingFile[], setFiles: SetFiles) => {
      const eligible = files.filter((p) => p.oversize && p.error && p.file.size <= limit);
      if (eligible.length === 0) return;
      const ids = new Set(eligible.map((p) => p.localId));
      setFiles((cur) =>
        cur.map((x) =>
          ids.has(x.localId) ? { ...x, error: false, progress: 0 } : x,
        ),
      );
      for (const p of eligible) startUpload(p, setFiles, 0);
    };
    retry(pendingFilesRef.current, setPendingFiles);
    retry(editFilesRef.current, setEditFiles);
  }, [tier, startUpload]);

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
