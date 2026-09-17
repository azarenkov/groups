import React from "react";
import { useTranslation } from "react-i18next";

import Icon from "../../../shared/ui/Icon";
import { ApiError } from "../../../shared/lib/apiError";
import { reportAPI } from "../api/report";
import {
  MAX_REPORT_DETAILS,
  SERVER_REPORT_REASONS,
  USER_REPORT_REASONS,
  countReportDetails,
  detailsLengthAfterEdit,
} from "../lib/reasons";
import { useReportModalStore } from "../store/reportModalStore";

const ReportModal: React.FC = () => {
  const { t } = useTranslation(["report", "common"]);
  const target = useReportModalStore((s) => s.target);
  const opening = useReportModalStore((s) => s.opening);
  const close = useReportModalStore((s) => s.close);
  const [reason, setReason] = React.useState<string | null>(null);
  const [details, setDetails] = React.useState("");
  const [sending, setSending] = React.useState(false);
  const [error, setError] = React.useState<string | null>(null);
  const [sent, setSent] = React.useState(false);
  const [evidenceId, setEvidenceId] = React.useState<string | undefined>(undefined);
  const activeOpeningRef = React.useRef(0);
  const closeTimerRef = React.useRef<ReturnType<typeof setTimeout> | null>(null);

  React.useEffect(() => {
    activeOpeningRef.current = opening;
    if (closeTimerRef.current) {
      clearTimeout(closeTimerRef.current);
      closeTimerRef.current = null;
    }
    setReason(null);
    setDetails("");
    setError(null);
    setSent(false);
    setSending(false);
    setEvidenceId(target?.messageId);
  }, [opening, target?.messageId]);

  React.useEffect(
    () => () => {
      if (closeTimerRef.current) clearTimeout(closeTimerRef.current);
    },
    [],
  );

  React.useEffect(() => {
    if (!target) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") close();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [target, close]);

  if (!target) return null;

  const reasons =
    target.type === "user" ? USER_REPORT_REASONS : SERVER_REPORT_REASONS;
  const trimmed = details.trim();
  const written = countReportDetails(trimmed);
  const needsDetails = reason === "other" && written === 0;
  const tooLong = written > MAX_REPORT_DETAILS;
  const canSend = !!reason && !needsDetails && !tooLong && !sending;

  const submit = async () => {
    if (!reason || !canSend) return;
    const sentFor = opening;
    const stillHere = () => activeOpeningRef.current === sentFor;
    setSending(true);
    setError(null);
    const payload = {
      reason,
      details: written > 0 ? trimmed : undefined,
      messageId: evidenceId,
    };
    try {
      if (target.type === "user") {
        await reportAPI.reportUser(target.id, payload);
      } else {
        await reportAPI.reportServer(target.id, payload);
      }
      if (!stillHere()) return;
      setSent(true);
      closeTimerRef.current = setTimeout(() => {
        closeTimerRef.current = null;
        if (stillHere()) useReportModalStore.getState().close();
      }, 1200);
    } catch (err) {
      if (!stillHere()) return;
      const apiErr = err instanceof ApiError ? err : null;
      const code = apiErr?.code;
      if (code === "REPORT_ALREADY_SENT") setError(t("errorAlreadySent"));
      else if (code === "REPORT_CONTEXT_MISMATCH") {
        setEvidenceId(undefined);
        setError(t("errorContext"));
      } else if (code === "REPORT_LIMIT_REACHED") setError(t("errorLimit"));
      else if (code === "REPORT_TARGET_NOT_FOUND") setError(t("errorTargetGone"));
      else setError(apiErr?.message || t("errorGeneric"));
    } finally {
      if (stillHere()) setSending(false);
    }
  };

  return (
    <div className="modal-overlay" onMouseDown={close}>
      <div
        className="modal report-modal"
        onMouseDown={(e) => e.stopPropagation()}
        role="dialog"
        aria-modal="true"
      >
        <div className="m-head">
          <h3>
            {target.type === "user" ? t("titleUser") : t("titleServer")}
          </h3>
          <button
            type="button"
            tabIndex={-1}
            className="close"
            onClick={close}
            title={t("close", { ns: "common" })}
          >
            <Icon name="x" size={14} />
          </button>
        </div>
        <div className="m-body">
          {sent ? (
            <div className="report-sent">
              <Icon name="check" size={18} />
              <span>{t("sent")}</span>
            </div>
          ) : (
            <>
              <p className="report-lead">
                {t("lead", { name: target.name })}
              </p>
              <div className="report-reasons">
                {reasons.map((value) => (
                  <label key={value} className="report-reason">
                    <input
                      type="radio"
                      name="report-reason"
                      value={value}
                      checked={reason === value}
                      onChange={() => {
                        setReason(value);
                        setError(null);
                      }}
                    />
                    <span>{t(`reasons.${value}`)}</span>
                  </label>
                ))}
              </div>
              <label className="report-details-label" htmlFor="report-details">
                {reason === "other" ? t("detailsRequired") : t("detailsOptional")}
              </label>
              <textarea
                id="report-details"
                className="report-details"
                rows={4}
                onBeforeInput={(e) => {
                  const inserted = (e.nativeEvent as InputEvent).data ?? "";
                  if (!inserted) return;
                  const field = e.currentTarget;
                  const next = detailsLengthAfterEdit(
                    details,
                    field.selectionStart ?? details.length,
                    field.selectionEnd ?? details.length,
                    inserted,
                  );
                  if (next > MAX_REPORT_DETAILS) e.preventDefault();
                }}
                placeholder={t("detailsPlaceholder")}
                value={details}
                onChange={(e) => {
                  setDetails(e.target.value);
                  setError(null);
                }}
              />
              <div className={"report-counter" + (tooLong ? " over" : "")}>
                {written} / {MAX_REPORT_DETAILS}
              </div>
              {error && <div className="report-error">{error}</div>}
            </>
          )}
        </div>
        {!sent && (
          <div className="m-foot">
            <button type="button" onClick={close}>
              {t("cancel", { ns: "common" })}
            </button>
            <button
              type="button"
              className="primary"
              disabled={!canSend}
              onClick={() => void submit()}
            >
              {sending ? t("sending") : t("submit")}
            </button>
          </div>
        )}
      </div>
    </div>
  );
};

export default ReportModal;
