import React from "react";

import Icon from "./Icon";

interface ErrorToastProps {
  message: string | null;
  onDismiss: () => void;
}

const ErrorToast: React.FC<ErrorToastProps> = ({ message, onDismiss }) =>
  message ? (
        <div
          style={{
            position: "fixed",
            bottom: 24,
            left: "50%",
            transform: "translateX(-50%)",
            background: "var(--danger)",
            color: "#fff",
            padding: "10px 16px",
            borderRadius: 12,
            display: "flex",
            alignItems: "center",
            gap: 12,
            boxShadow: "var(--shadow-md)",
            zIndex: 1200,
            fontSize: 13,
          }}
        >
          <span className="error-toast-text">{message}</span>
          <button
            type="button"
            onClick={onDismiss}
            style={{
              color: "#fff",
              opacity: 0.9,
              width: 22,
              height: 22,
              display: "inline-flex",
              alignItems: "center",
              justifyContent: "center",
              borderRadius: 6,
              background: "transparent",
              border: "none",
              cursor: "pointer",
            }}
            aria-label="Dismiss"
          >
            <Icon name="x" size={12} />
          </button>
        </div>
  ) : null;

export default ErrorToast;
