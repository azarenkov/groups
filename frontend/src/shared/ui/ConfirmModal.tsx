import React, { useEffect, useRef } from "react";
import { Trans, useTranslation } from "react-i18next";
import Icon from "./Icon";
import { focusReturnTarget } from "./contextMenu/contextMenuStore";

interface ConfirmModalProps {
  title: string;
  subtitle?: string;
  targetName?: string;
  confirmLabel: string;
  danger?: boolean;
  onCancel: () => void;
  onConfirm: () => void;
}

const ConfirmModal: React.FC<ConfirmModalProps> = ({
  title,
  subtitle,
  targetName,
  confirmLabel,
  danger,
  onCancel,
  onConfirm,
}) => {
  const { t } = useTranslation("common");
  const cancelRef = useRef<HTMLButtonElement>(null);
  const confirmRef = useRef<HTMLButtonElement>(null);

  useEffect(() => {
    const previous = focusReturnTarget();
    cancelRef.current?.focus();
    return () => {
      if (previous && previous.isConnected) previous.focus({ preventScroll: true });
    };
  }, []);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        onCancel();
        return;
      }
      if (e.key !== "Tab") return;
      const active = document.activeElement;
      if (e.shiftKey && active === cancelRef.current) {
        e.preventDefault();
        confirmRef.current?.focus();
      } else if (!e.shiftKey && active === confirmRef.current) {
        e.preventDefault();
        cancelRef.current?.focus();
      } else if (active !== cancelRef.current && active !== confirmRef.current) {
        e.preventDefault();
        cancelRef.current?.focus();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onCancel]);

  return (
    <div className="modal-overlay" onMouseDown={onCancel}>
      <div className="modal" onMouseDown={(e) => e.stopPropagation()}>
        <div className="m-head">
          <h3>{title}</h3>
          <button
            type="button"
            tabIndex={-1}
            className="close"
            onClick={onCancel}
            title={t("close")}
          >
            <Icon name="x" size={14} />
          </button>
        </div>
        <div className="m-body">
          {subtitle && (
            <p style={{ fontSize: 13, color: "var(--fg-3)", margin: "0 0 8px" }}>{subtitle}</p>
          )}
          {targetName && (
            <p style={{ fontSize: 13, color: "var(--fg-2)", margin: 0 }}>
              <Trans
                i18nKey="confirm.aboutToDelete"
                ns="modals"
                values={{ name: targetName }}
                components={[
                  <strong key="n" style={{ color: "var(--fg-1)", fontWeight: 600 }} />,
                ]}
              />
            </p>
          )}
        </div>
        <div className="m-foot">
          <button ref={cancelRef} type="button" onClick={onCancel}>
            {t("cancel")}
          </button>
          <button
            ref={confirmRef}
            type="button"
            className={danger ? "danger" : "primary"}
            onClick={onConfirm}
            style={
              danger
                ? { background: "var(--danger)", color: "#fff", borderColor: "transparent" }
                : undefined
            }
          >
            {confirmLabel}
          </button>
        </div>
      </div>
    </div>
  );
};

export default ConfirmModal;
