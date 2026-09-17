import React from "react";

import ImageCropModal from "../ui/ImageCropModal";
import type { ImageCropOptions } from "../ui/ImageCropModal";
import { setCrop } from "../lib/imageCrop";

interface Pending extends ImageCropOptions {
  file: File;
  resolve: (file: File | null) => void;
}

export function useImageCrop() {
  const [pending, setPending] = React.useState<Pending | null>(null);
  const pendingRef = React.useRef<Pending | null>(null);
  pendingRef.current = pending;

  React.useEffect(
    () => () => {
      pendingRef.current?.resolve(null);
    },
    [],
  );

  const requestCrop = React.useCallback(
    (file: File, options: ImageCropOptions) =>
      new Promise<File | null>((resolve) => {
        pendingRef.current?.resolve(null);
        setPending({ file, resolve, ...options });
      }),
    [],
  );

  const cropModal = pending ? (
    <ImageCropModal
      file={pending.file}
      aspect={pending.aspect}
      shape={pending.shape}
      title={pending.title}
      onCancel={() => {
        pending.resolve(null);
        setPending(null);
      }}
      onConfirm={(rect) => {
        setCrop(pending.file, rect);
        pending.resolve(pending.file);
        setPending(null);
      }}
    />
  ) : null;

  return { requestCrop, cropModal };
}
