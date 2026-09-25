import React, { useEffect, useRef } from "react";
import { Trans, useTranslation } from "react-i18next";
import Icon from "./Icon";

interface ConfirmModalProps {
  title: string;
  subtitle?: string;
  targetName?: string;
  confirmLabel: string;
  danger?: boolean;
  secondaryLabel?: string;
  onCancel: () => void;
  onConfirm: () => void;
  onSecondary?: () => void;
}

const ConfirmModal: React.FC<ConfirmModalProps> = ({
  title,
  subtitle,
  targetName,
  confirmLabel,
  danger,
  secondaryLabel,
  onCancel,
  onConfirm,
  onSecondary,
}) => {
  const { t } = useTranslation("common");
  const cancelRef = useRef<HTMLButtonElement>(null);
  const secondaryRef = useRef<HTMLButtonElement>(null);
  const confirmRef = useRef<HTMLButtonElement>(null);

  useEffect(() => {
    cancelRef.current?.focus();
  }, []);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        onCancel();
        return;
      }
      if (e.key !== "Tab") return;
      const order = [cancelRef.current, secondaryRef.current, confirmRef.current].filter(
        (el): el is HTMLButtonElement => !!el,
      );
      const index = order.indexOf(document.activeElement as HTMLButtonElement);
      e.preventDefault();
      if (index === -1) {
        order[0]?.focus();
        return;
      }
      const next = (index + (e.shiftKey ? order.length - 1 : 1)) % order.length;
      order[next]?.focus();
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
          {secondaryLabel && onSecondary && (
            <button ref={secondaryRef} type="button" onClick={onSecondary}>
              {secondaryLabel}
            </button>
          )}
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
