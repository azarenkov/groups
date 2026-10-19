import React from "react";

export interface SwitchProps {
  disabled?: boolean; on: boolean; onClick: () => void; }

export function Switch({ on, onClick, disabled }: SwitchProps) {
  return <button className={'s-switch' + (on ? ' on' : '')} onClick={onClick} disabled={disabled} aria-pressed={on}/>;
}

export interface SettingsRowProps {
  label: string;
  hint?: string;
  hintClassName?: string;
  children: React.ReactNode;
}

export function SettingsRow({ label, hint, hintClassName, children }: SettingsRowProps) {
  return (
    <div className="s-row">
      <div className="s-row-text">
        <div className="lbl">{label}</div>
        {hint && <div className={"hint" + (hintClassName ? ` ${hintClassName}` : "")}>{hint}</div>}
      </div>
      <div className="s-row-control">{children}</div>
    </div>
  );
}
