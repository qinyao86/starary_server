import type { LucideIcon } from "lucide-react";
import { ChevronDown } from "lucide-react";
import type { ReactNode } from "react";

export function Segmented({
  value,
  options,
  onChange
}: {
  value: string;
  options: Array<{ value: string; label: string; icon: LucideIcon }>;
  onChange: (value: string) => void;
}) {
  return (
    <div className="segmented">
      {options.map((option) => {
        const Icon = option.icon;
        return (
          <button
            className={option.value === value ? "is-active" : ""}
            key={option.value}
            type="button"
            onClick={() => onChange(option.value)}
          >
            <Icon size={15} />
            <span>{option.label}</span>
          </button>
        );
      })}
    </div>
  );
}

export function TextField({
  label,
  value,
  onChange,
  type = "text",
  placeholder,
  required = false,
  autoFocus = false
}: {
  label: string;
  value: string;
  onChange: (value: string) => void;
  type?: string;
  placeholder?: string;
  required?: boolean;
  autoFocus?: boolean;
}) {
  return (
    <label className="field">
      <span>{label}</span>
      <input
        autoFocus={autoFocus}
        required={required}
        type={type}
        value={value}
        placeholder={placeholder}
        onChange={(event) => onChange(event.target.value)}
      />
    </label>
  );
}

export function SelectField({
  children,
  className = "",
  label,
  required = false,
  value,
  onChange
}: {
  children: ReactNode;
  className?: string;
  label: string;
  required?: boolean;
  value: string;
  onChange: (value: string) => void;
}) {
  return (
    <label className={`field ${className}`.trim()}>
      <span>{label}</span>
      <span className="select-control">
        <select required={required} value={value} onChange={(event) => onChange(event.target.value)}>
          {children}
        </select>
        <ChevronDown aria-hidden="true" className="select-control-icon" size={15} />
      </span>
    </label>
  );
}

export function Badge({ children }: { children: ReactNode }) {
  return <span className="badge">{children}</span>;
}
