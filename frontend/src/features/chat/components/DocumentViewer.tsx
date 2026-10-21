import React from "react";
import { createPortal } from "react-dom";
import { useTranslation } from "react-i18next";
import type { PDFDocumentProxy, PDFPageProxy } from "pdfjs-dist/types/src/display/api";

import { attachmentAPI } from "../api/attachment";
import {
  CSV_PREVIEW_MAX_ROWS,
  TEXT_PREVIEW_MAX_BYTES,
  documentKind,
  parseCsv,
  prettyJson,
} from "../lib/documentPreview";
import { loadPdfJs, openPdf } from "../lib/pdf";
import { saveAttachment } from "../lib/saveAttachment";
import type { Attachment } from "../../../shared/types/channel";
import Icon from "../../../shared/ui/Icon";
import LoadingDots from "../../../shared/ui/LoadingDots";
import { useEscape } from "./MediaViewers";

const MIN_SCALE = 0.25;
const MAX_SCALE = 4;
const SCALE_STEP = 1.2;
const PAGE_GAP = 16;

type Loaded =
  | { kind: "pdf"; document: PDFDocumentProxy }
  | { kind: "csv"; rows: string[][]; truncated: boolean }
  | { kind: "text"; text: string };

type Phase = { state: "loading" } | { state: "ready"; content: Loaded } | { state: "failed" };

const clampScale = (scale: number) => Math.min(MAX_SCALE, Math.max(MIN_SCALE, scale));

const PdfPage: React.FC<{
  document: PDFDocumentProxy;
  pageNumber: number;
  scale: number;
  baseSize: { width: number; height: number };
  onVisible: (page: number) => void;
}> = ({ document: pdf, pageNumber, scale, baseSize, onVisible }) => {
  const holderRef = React.useRef<HTMLDivElement>(null);
  const canvasRef = React.useRef<HTMLCanvasElement>(null);
  const textRef = React.useRef<HTMLDivElement>(null);
  const [visible, setVisible] = React.useState(false);

  React.useEffect(() => {
    const node = holderRef.current;
    if (!node) return;
    if (typeof IntersectionObserver === "undefined") {
      setVisible(true);
      return;
    }
    const observer = new IntersectionObserver(
      (entries) => {
        for (const entry of entries) {
          if (entry.isIntersecting) {
            setVisible(true);
            if (entry.intersectionRatio >= 0.5) onVisible(pageNumber);
          }
        }
      },
      { rootMargin: "800px 0px", threshold: [0, 0.5] },
    );
    observer.observe(node);
    return () => observer.disconnect();
  }, [onVisible, pageNumber]);

  React.useEffect(() => {
    if (!visible) return;
    let cancelled = false;
    let page: PDFPageProxy | null = null;
    let renderTask: { cancel: () => void; promise: Promise<unknown> } | null = null;
    (async () => {
      page = await pdf.getPage(pageNumber);
      if (cancelled) return;
      const viewport = page.getViewport({ scale });
      const canvas = canvasRef.current;
      const textLayer = textRef.current;
      if (!canvas || !textLayer) return;
      const ratio = window.devicePixelRatio || 1;
      canvas.width = Math.floor(viewport.width * ratio);
      canvas.height = Math.floor(viewport.height * ratio);
      canvas.style.width = `${viewport.width}px`;
      canvas.style.height = `${viewport.height}px`;
      const context = canvas.getContext("2d");
      if (!context) return;
      renderTask = page.render({
        canvasContext: context,
        viewport,
        transform: ratio === 1 ? undefined : [ratio, 0, 0, ratio, 0, 0],
      });
      await renderTask.promise;
      if (cancelled) return;
      const pdfjs = await loadPdfJs();
      textLayer.replaceChildren();
      textLayer.style.setProperty("--scale-factor", String(viewport.scale));
      await pdfjs.renderTextLayer({
        textContentSource: page.streamTextContent(),
        container: textLayer,
        viewport,
      }).promise;
    })().catch(() => {});
    return () => {
      cancelled = true;
      renderTask?.cancel();
    };
  }, [pdf, pageNumber, scale, visible]);

  return (
    <div
      ref={holderRef}
      className="doc-viewer-page"
      data-page={pageNumber}
      style={{ width: baseSize.width * scale, height: baseSize.height * scale }}
    >
      <canvas ref={canvasRef} />
      <div ref={textRef} className="doc-viewer-text-layer" />
    </div>
  );
};

const PdfDocument: React.FC<{
  document: PDFDocumentProxy;
  scale: number | null;
  onFit: (scale: number) => void;
  onPage: (page: number) => void;
  scrollTo: number | null;
}> = ({ document: pdf, scale, onFit, onPage, scrollTo }) => {
  const scrollRef = React.useRef<HTMLDivElement>(null);
  const [baseSize, setBaseSize] = React.useState<{ width: number; height: number } | null>(null);

  React.useEffect(() => {
    let cancelled = false;
    void pdf.getPage(1).then((page) => {
      if (cancelled) return;
      const viewport = page.getViewport({ scale: 1 });
      setBaseSize({ width: viewport.width, height: viewport.height });
      const available = (scrollRef.current?.clientWidth ?? viewport.width) - PAGE_GAP * 2;
      onFit(clampScale(available / viewport.width));
    });
    return () => {
      cancelled = true;
    };
  }, [pdf, onFit]);

  React.useEffect(() => {
    if (scrollTo === null) return;
    scrollRef.current?.querySelector(`[data-page="${scrollTo}"]`)?.scrollIntoView({ block: "start" });
  }, [scrollTo]);

  return (
    <div ref={scrollRef} className="doc-viewer-scroll" tabIndex={-1}>
      {baseSize && scale !== null
        ? Array.from({ length: pdf.numPages }, (_, i) => (
            <PdfPage key={i + 1} document={pdf} pageNumber={i + 1} scale={scale} baseSize={baseSize} onVisible={onPage} />
          ))
        : <LoadingDots />}
    </div>
  );
};

const readText = async (attachment: Attachment, signal: AbortSignal): Promise<string> => {
  const blob = await attachmentAPI.download(attachment.id, attachment.sizeBytes, signal);
  return (blob.size > TEXT_PREVIEW_MAX_BYTES ? blob.slice(0, TEXT_PREVIEW_MAX_BYTES) : blob).text();
};

export const DocumentViewer: React.FC<{ attachment: Attachment; onClose: () => void }> = ({ attachment, onClose }) => {
  const { t } = useTranslation("chat");
  const kind = documentKind(attachment.contentType, attachment.fileName);
  const [phase, setPhase] = React.useState<Phase>({ state: "loading" });
  const [scale, setScale] = React.useState<number | null>(null);
  const [fitScale, setFitScale] = React.useState<number | null>(null);
  const [page, setPage] = React.useState(1);
  const [scrollTo, setScrollTo] = React.useState<number | null>(null);
  useEscape(onClose);

  React.useEffect(() => {
    const controller = new AbortController();
    let opened: PDFDocumentProxy | null = null;
    setPhase({ state: "loading" });
    (async (): Promise<Loaded> => {
      if (kind === "pdf" || kind === "office") {
        const blob = await attachmentAPI.preview(attachment.id, attachment.sizeBytes, controller.signal);
        opened = await openPdf(await blob.arrayBuffer());
        return { kind: "pdf", document: opened };
      }
      const text = await readText(attachment, controller.signal);
      if (kind === "csv") {
        const { rows, truncated } = parseCsv(text, CSV_PREVIEW_MAX_ROWS);
        return { kind: "csv", rows, truncated };
      }
      return { kind: "text", text: kind === "json" ? prettyJson(text) : text };
    })()
      .then((content) => {
        if (!controller.signal.aborted) setPhase({ state: "ready", content });
      })
      .catch(() => {
        if (!controller.signal.aborted) setPhase({ state: "failed" });
      });
    return () => {
      controller.abort();
      void opened?.destroy();
    };
  }, [attachment, kind]);

  const handleFit = React.useCallback((value: number) => {
    setFitScale(value);
    setScale((current) => current ?? value);
  }, []);

  const pdf = phase.state === "ready" && phase.content.kind === "pdf" ? phase.content.document : null;
  const total = pdf?.numPages ?? 0;

  const goTo = React.useCallback(
    (target: number) => {
      if (!total) return;
      const next = Math.min(total, Math.max(1, target));
      setPage(next);
      setScrollTo(next);
    },
    [total],
  );

  React.useEffect(() => {
    if (!pdf) return;
    const onKey = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && (e.key === "=" || e.key === "+")) {
        e.preventDefault();
        setScale((s) => clampScale((s ?? 1) * SCALE_STEP));
      } else if ((e.ctrlKey || e.metaKey) && e.key === "-") {
        e.preventDefault();
        setScale((s) => clampScale((s ?? 1) / SCALE_STEP));
      } else if (e.key === "ArrowRight" || e.key === "PageDown") {
        goTo(page + 1);
      } else if (e.key === "ArrowLeft" || e.key === "PageUp") {
        goTo(page - 1);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [pdf, page, goTo]);

  let body: React.ReactNode;
  if (phase.state === "loading") {
    body = (
      <div className="doc-viewer-status">
        <LoadingDots />
        <span>{kind === "office" ? t("documentPreparing") : t("documentLoading")}</span>
      </div>
    );
  } else if (phase.state === "failed") {
    body = (
      <div className="doc-viewer-status">
        <span>{t("documentUnavailable")}</span>
        <button type="button" className="doc-viewer-action" onClick={() => void saveAttachment(attachment)}>
          {t("download")}
        </button>
      </div>
    );
  } else if (phase.content.kind === "pdf") {
    body = <PdfDocument document={phase.content.document} scale={scale} onFit={handleFit} onPage={setPage} scrollTo={scrollTo} />;
  } else if (phase.content.kind === "csv") {
    body = (
      <div className="doc-viewer-scroll doc-viewer-sheet">
        <table>
          <tbody>
            {phase.content.rows.map((row, r) => (
              <tr key={r}>
                {row.map((cell, c) => (r === 0 ? <th key={c}>{cell}</th> : <td key={c}>{cell}</td>))}
              </tr>
            ))}
          </tbody>
        </table>
        {phase.content.truncated && <p className="doc-viewer-note">{t("documentRowsTruncated", { rows: CSV_PREVIEW_MAX_ROWS })}</p>}
      </div>
    );
  } else {
    body = (
      <div className="doc-viewer-scroll doc-viewer-text">
        <pre>{phase.content.text}</pre>
        {attachment.sizeBytes > TEXT_PREVIEW_MAX_BYTES && <p className="doc-viewer-note">{t("documentTextTruncated")}</p>}
      </div>
    );
  }

  return createPortal(
    <div className="doc-viewer-overlay" onClick={onClose}>
      <div className="doc-viewer" role="dialog" aria-label={attachment.fileName} onClick={(e) => e.stopPropagation()}>
        <div className="doc-viewer-bar">
          <span className="doc-viewer-name" title={attachment.fileName}>{attachment.fileName}</span>
          {pdf && (
            <div className="doc-viewer-controls">
              <button type="button" aria-label={t("documentPreviousPage")} onClick={() => goTo(page - 1)} disabled={page <= 1}>
                <Icon name="chev-left" size={16} />
              </button>
              <span className="doc-viewer-page-count">{t("documentPage", { page, total })}</span>
              <button type="button" aria-label={t("documentNextPage")} onClick={() => goTo(page + 1)} disabled={page >= total}>
                <Icon name="chev" size={16} />
              </button>
              <button type="button" aria-label={t("documentZoomOut")} onClick={() => setScale((s) => clampScale((s ?? 1) / SCALE_STEP))}>
                −
              </button>
              <span className="doc-viewer-zoom">{Math.round((scale ?? 1) * 100)}%</span>
              <button type="button" aria-label={t("documentZoomIn")} onClick={() => setScale((s) => clampScale((s ?? 1) * SCALE_STEP))}>
                +
              </button>
              <button type="button" onClick={() => fitScale !== null && setScale(fitScale)}>
                {t("documentFitWidth")}
              </button>
            </div>
          )}
          <button type="button" className="doc-viewer-action" onClick={() => void saveAttachment(attachment)}>
            <Icon name="download" size={16} />
            <span>{t("download")}</span>
          </button>
          <button type="button" className="doc-viewer-close" aria-label={t("documentClose")} onClick={onClose}>
            <Icon name="x" size={18} />
          </button>
        </div>
        {body}
      </div>
    </div>,
    document.body,
  );
};

export default DocumentViewer;
