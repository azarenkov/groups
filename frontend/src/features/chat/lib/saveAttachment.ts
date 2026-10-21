import { attachmentAPI } from "../api/attachment";

export const saveBlob = (blob: Blob, fileName: string) => {
  const objectUrl = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = objectUrl;
  a.download = fileName;
  document.body.appendChild(a);
  a.click();
  document.body.removeChild(a);
  setTimeout(() => URL.revokeObjectURL(objectUrl), 10_000);
};

export const saveAttachment = (attachment: { id: string; fileName: string; sizeBytes: number }) =>
  attachmentAPI
    .download(attachment.id, attachment.sizeBytes)
    .then((blob) => saveBlob(blob, attachment.fileName))
    .catch(() => {});
