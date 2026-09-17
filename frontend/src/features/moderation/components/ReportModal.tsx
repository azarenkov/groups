import React from "react";
import { useTranslation } from "react-i18next";

import Icon from "../../../shared/ui/Icon";
import { ApiError } from "../../../shared/lib/apiError";
import { reportAPI } from "../api/report";
import {
  MAX_REPORT_DETAILS,
  SERVER_REPORT_REASONS,
  USER_REPORT_REASONS,
} from "../lib/reasons";
import { useReportModalStore } from "../store/reportModalStore";

const ReportModal: React.FC = () => {
  const { t } = useTranslation(["report", "common"]);
  const target = useReportModalStore((s) => s.target);
  const close = useReportModalStore((s) => s.close);
  const [reason, setReason] = React.useState<string | null>(null);
  const [details, setDetails] = React.useState("");
  const [sending, setSending] = React.useState(false);
  const [error, setError] = React.useState<string | null>(null);
  const [sent, setSent] = React.useState(false);

  const targetKey = target ? `${target.type}:${target.id}` : null;
  React.useEffect(() => {
    setReason(null);
    setDetails("");
    setError(null);
    setSent(false);
    setSending(false);
  }, [targetKey]);

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
  const needsDetails = reason === "other" && trimmed.length === 0;
  const tooLong = trimmed.length > MAX_REPORT_DETAILS;
  const canSend = !!reason && !needsDetails && !tooLong && !sending;

  const submit = async () => {
    if (!reason || !canSend) return;
    setSending(true);
    setError(null);
    const payload = {
      reason,
      details: trimmed.length > 0 ? trimmed : undefined,
      channelId: target.channelId,
      messageId: target.messageId,
    };
    try {
      if (target.type === "user") {
        await reportAPI.reportUser(target.id, payload);
      } else {
        await reportAPI.reportServer(target.id, payload);
      }
      setSent(true);
      window.setTimeout(() => useReportModalStore.getState().close(), 1200);
    } catch (err) {
      const apiErr = err instanceof ApiError ? err : null;
      const code = apiErr?.code;
      if (code === "REPORT_ALREADY_SENT") setError(t("errorAlreadySent"));
      else if (code === "REPORT_LIMIT_REACHED") setError(t("errorLimit"));
      else if (code === "REPORT_TARGET_NOT_FOUND") setError(t("errorTargetGone"));
      else setError(apiErr?.message || t("errorGeneric"));
    } finally {
      setSending(false);
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
                maxLength={MAX_REPORT_DETAILS + 200}
                placeholder={t("detailsPlaceholder")}
                value={details}
                onChange={(e) => {
                  setDetails(e.target.value);
                  setError(null);
                }}
              />
              <div className={"report-counter" + (tooLong ? " over" : "")}>
                {trimmed.length} / {MAX_REPORT_DETAILS}
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
