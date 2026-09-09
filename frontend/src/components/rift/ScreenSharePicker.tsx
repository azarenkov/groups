import React, { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import type { ScreenShareSource } from "../../types/global";

interface PendingPick {
  requestId: number;
  sources: ScreenShareSource[];
}

const ScreenSharePicker: React.FC = () => {
  const { t } = useTranslation("voice");
  const [pending, setPending] = useState<PendingPick | null>(null);

  useEffect(() => {
    const api = window.electron;
    if (!api?.onScreenShareSources) return;
    return api.onScreenShareSources((data) => setPending(data));
  }, []);

  useEffect(() => {
    if (!pending) return;
    const requestId = pending.requestId;
    const onKey = (e: KeyboardEvent) => {
      if (e.key !== "Escape") return;
      if (document.querySelector(".incoming-call")) return;
      e.preventDefault();
      e.stopImmediatePropagation();
      window.electron?.pickScreenShareSource?.(requestId, null);
      setPending((current) => (current?.requestId === requestId ? null : current));
    };
    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
  }, [pending]);

  if (!pending) return null;

  const resolve = (sourceId: string | null) => {
    window.electron?.pickScreenShareSource?.(pending.requestId, sourceId);
    setPending(null);
  };

  const screens = pending.sources.filter((s) => s.kind === "screen");
  const windows = pending.sources.filter((s) => s.kind === "window");

  const renderGroup = (title: string, items: ScreenShareSource[]) =>
    items.length > 0 && (
      <div className="ssp-group">
        <div className="ssp-group-title">{title}</div>
        <div className="ssp-grid">
          {items.map((s) => (
            <button
              key={s.id}
              type="button"
              className="ssp-item"
              onClick={() => resolve(s.id)}
            >
              <img src={s.thumbnail} alt="" className="ssp-thumb" />
              <span className="ssp-name" title={s.name}>
                {s.name}
              </span>
            </button>
          ))}
        </div>
      </div>
    );

  return (
    <div className="ssp-overlay" onMouseDown={() => resolve(null)}>
      <div className="ssp-modal" onMouseDown={(e) => e.stopPropagation()}>
        <div className="ssp-header">
          <h2>{t("pickScreenTitle")}</h2>
          <p>{t("pickScreenSubtitle")}</p>
        </div>
        <div className="ssp-body">
          {renderGroup(t("entireScreen"), screens)}
          {renderGroup(t("applicationWindow"), windows)}
        </div>
        <div className="ssp-footer">
          <button
            type="button"
            className="btn"
            onClick={() => resolve(null)}
          >
            {t("cancel")}
          </button>
        </div>
      </div>
    </div>
  );
};

export default ScreenSharePicker;
