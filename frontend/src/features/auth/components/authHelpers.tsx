import React from "react";

export const ArrowLeft: React.FC<{ size?: number }> = ({ size = 14 }) => (
  <svg
    width={size}
    height={size}
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    strokeWidth={1.8}
    strokeLinecap="round"
    strokeLinejoin="round"
  >
    <line x1="19" y1="12" x2="5" y2="12" />
    <polyline points="12 19 5 12 12 5" />
  </svg>
);

interface OtpInputProps {
  value: string;
  onChange: (v: string) => void;
  autoFocus?: boolean;
}

export const OtpInput: React.FC<OtpInputProps> = ({ value, onChange, autoFocus }) => {
  const refs = React.useRef<(HTMLInputElement | null)[]>([]);
  const digits = (value || "").padEnd(6, " ").slice(0, 6).split("");

  const setAt = (i: number, ch: string) => {
    const next = digits.slice();
    next[i] = ch;
    onChange(next.join("").replace(/\s+$/, ""));
  };

  return (
    <div className="auth-otp">
      {digits.map((d, i) => (
        <input
          key={i}
          ref={(el) => {
            refs.current[i] = el;
          }}
          autoFocus={autoFocus && i === 0}
          inputMode="numeric"
          maxLength={1}
          className={d.trim() ? "filled" : ""}
          value={d.trim()}
          onChange={(e) => {
            const v = e.target.value.replace(/\D/g, "").slice(-1);
            setAt(i, v || " ");
            if (v && i < 5) refs.current[i + 1]?.focus();
          }}
          onKeyDown={(e) => {
            if (e.key === "Backspace" && !digits[i].trim() && i > 0) {
              refs.current[i - 1]?.focus();
              setAt(i - 1, " ");
              e.preventDefault();
            } else if (e.key === "ArrowLeft" && i > 0) refs.current[i - 1]?.focus();
            else if (e.key === "ArrowRight" && i < 5) refs.current[i + 1]?.focus();
          }}
          onPaste={(e) => {
            const pasted = e.clipboardData.getData("text").replace(/\D/g, "").slice(0, 6);
            if (!pasted) return;
            e.preventDefault();
            onChange(pasted);
            refs.current[Math.min(pasted.length, 5)]?.focus();
          }}
        />
      ))}
    </div>
  );
};

export interface PasswordStrength {
  score: 0 | 1 | 2 | 3 | 4;
  label: string;
}

export const passwordStrength = (pw: string): PasswordStrength => {
  if (!pw) return { score: 0, label: "" };
  let s = 0;
  if (pw.length >= 8) s++;
  if (pw.length >= 12) s++;
  if (/[A-Z]/.test(pw) && /[a-z]/.test(pw)) s++;
  if (/[0-9]/.test(pw) && /[^A-Za-z0-9]/.test(pw)) s++;
  const labels = ["Too short", "Weak", "Okay", "Strong", "Excellent"];
  return { score: s as PasswordStrength["score"], label: labels[s] || "" };
};

export const GoogleGlyph: React.FC = () => (
  <svg width="16" height="16" viewBox="0 0 24 24" aria-hidden="true">
    <path
      fill="#EA4335"
      d="M12 10.2v3.84h5.44c-.24 1.4-.98 2.59-2.08 3.39v2.82h3.37C20.69 18.3 22 15.5 22 12.17c0-.76-.07-1.49-.2-2.17H12z"
    />
    <path
      fill="#34A853"
      d="M12 22c2.7 0 4.96-.9 6.61-2.43l-3.37-2.62c-.93.63-2.13 1-3.24 1-2.49 0-4.6-1.68-5.35-3.95H3.18v2.48A10 10 0 0 0 12 22z"
    />
    <path
      fill="#FBBC04"
      d="M6.65 13.76A5.99 5.99 0 0 1 6.33 12c0-.61.11-1.2.3-1.76V7.76H3.18A10 10 0 0 0 2 12c0 1.6.38 3.11 1.06 4.44l3.59-2.68z"
    />
    <path
      fill="#4285F4"
      d="M12 6.5c1.46 0 2.77.5 3.8 1.48l2.85-2.84A9.6 9.6 0 0 0 12 2 10 10 0 0 0 3.18 7.56l3.47 2.7C7.4 8.18 9.51 6.5 12 6.5z"
    />
  </svg>
);

export const AppleGlyph: React.FC = () => (
  <svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor" aria-hidden="true">
    <path d="M16.365 1.43c0 1.14-.43 2.22-1.14 3.01-.75.85-1.94 1.51-3.1 1.42-.13-1.1.42-2.26 1.1-3 .76-.83 2-1.46 3.14-1.43zM20.5 17.3c-.55 1.27-.82 1.84-1.53 2.96-.99 1.55-2.39 3.48-4.12 3.49-1.54.01-1.93-1-4.02-1-2.1 0-2.52 1-4.06.99-1.73-.01-3.05-1.76-4.04-3.31-2.77-4.33-3.06-9.4-1.35-12.1 1.21-1.92 3.12-3.04 4.9-3.04 1.82 0 2.96 1 4.47 1 1.46 0 2.35-1 4.45-1 1.59 0 3.27.87 4.47 2.36-3.93 2.15-3.29 7.76 0 8.65z" />
  </svg>
);

export const FormError: React.FC<{ children: React.ReactNode }> = ({ children }) => (
  <div
    className="error-text"
    role="alert"
    style={{
      fontSize: 12,
      color: "var(--danger)",
      background: "var(--danger-tint)",
      padding: "8px 12px",
      borderRadius: 8,
      marginTop: 10,
      textAlign: "center",
    }}
  >
    {children}
  </div>
);
