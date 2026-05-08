import React from "react";
import Icon from "./Icon";

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
}) => (
  <div className="modal-overlay" onMouseDown={onCancel}>
    <div className="modal" onMouseDown={(e) => e.stopPropagation()}>
      <div className="m-head">
        <h3>{title}</h3>
        <button type="button" className="close" onClick={onCancel} title="Close">
          <Icon name="x" size={14} />
        </button>
      </div>
      <div className="m-body">
        {subtitle && (
          <p style={{ fontSize: 13, color: "var(--fg-3)", margin: "0 0 8px" }}>{subtitle}</p>
        )}
        {targetName && (
          <p style={{ fontSize: 13, color: "var(--fg-2)", margin: 0 }}>
            You're about to delete{" "}
            <strong style={{ color: "var(--fg-1)", fontWeight: 600 }}>{targetName}</strong>.
          </p>
        )}
      </div>
      <div className="m-foot">
        <button type="button" onClick={onCancel}>
          Cancel
        </button>
        <button
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

export default ConfirmModal;
