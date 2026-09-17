export interface CropRect {
  x: number;
  y: number;
  width: number;
  height: number;
}

const crops = new WeakMap<File, CropRect>();

export const setCrop = (file: File, rect: CropRect) => {
  crops.set(file, rect);
};

export const cropOf = (file: File): CropRect | undefined => crops.get(file);

export const appendCrop = (form: FormData, file: File) => {
  const rect = crops.get(file);
  if (!rect) return;
  form.append("cropX", String(rect.x));
  form.append("cropY", String(rect.y));
  form.append("cropWidth", String(rect.width));
  form.append("cropHeight", String(rect.height));
};

export interface CropView {
  frameWidth: number;
  frameHeight: number;
  naturalWidth: number;
  naturalHeight: number;
  zoom: number;
  offsetX: number;
  offsetY: number;
}

export const coverScale = (view: Pick<CropView, "frameWidth" | "frameHeight" | "naturalWidth" | "naturalHeight">) =>
  Math.max(view.frameWidth / view.naturalWidth, view.frameHeight / view.naturalHeight);

export const clampOffset = (view: CropView): { offsetX: number; offsetY: number } => {
  const scale = coverScale(view) * view.zoom;
  const minX = view.frameWidth - view.naturalWidth * scale;
  const minY = view.frameHeight - view.naturalHeight * scale;
  return {
    offsetX: Math.min(0, Math.max(minX, view.offsetX)),
    offsetY: Math.min(0, Math.max(minY, view.offsetY)),
  };
};

export const cropFromView = (view: CropView): CropRect => {
  const scale = coverScale(view) * view.zoom;
  const { offsetX, offsetY } = clampOffset(view);
  const aspect = view.frameWidth / view.frameHeight;
  let width = Math.min(view.naturalWidth, Math.round(view.frameWidth / scale));
  let height = Math.round(width / aspect);
  if (height > view.naturalHeight) {
    height = view.naturalHeight;
    width = Math.round(height * aspect);
  }
  const x = Math.min(view.naturalWidth - width, Math.max(0, Math.round(-offsetX / scale)));
  const y = Math.min(view.naturalHeight - height, Math.max(0, Math.round(-offsetY / scale)));
  return { x, y, width, height };
};

export const renderCropPreview = (file: File, rect: CropRect, size: number): Promise<string | null> =>
  new Promise((resolve) => {
    const url = URL.createObjectURL(file);
    const img = new Image();
    img.onload = () => {
      try {
        const canvas = document.createElement("canvas");
        canvas.width = size;
        canvas.height = Math.round((size * rect.height) / rect.width);
        const ctx = canvas.getContext("2d");
        if (!ctx) {
          resolve(null);
          return;
        }
        ctx.drawImage(img, rect.x, rect.y, rect.width, rect.height, 0, 0, canvas.width, canvas.height);
        resolve(canvas.toDataURL("image/png"));
      } catch {
        resolve(null);
      } finally {
        URL.revokeObjectURL(url);
      }
    };
    img.onerror = () => {
      URL.revokeObjectURL(url);
      resolve(null);
    };
    img.src = url;
  });
