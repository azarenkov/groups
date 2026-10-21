import type { PDFDocumentProxy } from "pdfjs-dist/types/src/display/api";

import { fetchAssetText } from "../../../shared/lib/assets";

type PdfJs = typeof import("pdfjs-dist");

const WORKER_ASSET = "vendor/pdf.worker.min.js";

let loading: Promise<PdfJs> | null = null;

export const loadPdfJs = (): Promise<PdfJs> => {
  if (!loading) {
    loading = (async () => {
      const pdfjs = (await import("pdfjs-dist/legacy/build/pdf")) as unknown as PdfJs;
      const source = await fetchAssetText(WORKER_ASSET);
      const workerUrl = URL.createObjectURL(new Blob([source], { type: "application/javascript" }));
      pdfjs.GlobalWorkerOptions.workerPort = new Worker(workerUrl);
      return pdfjs;
    })().catch((error) => {
      loading = null;
      throw error;
    });
  }
  return loading;
};

export const openPdf = async (data: ArrayBuffer): Promise<PDFDocumentProxy> => {
  const pdfjs = await loadPdfJs();
  return pdfjs.getDocument({ data, isEvalSupported: false, enableXfa: false }).promise;
};
