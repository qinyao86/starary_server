import type { LucideIcon } from "lucide-react";
import type { ReactNode } from "react";

export function PageFrame({
  title,
  description,
  titleSlot,
  action,
  children
}: {
  title: string;
  description: string;
  titleSlot?: ReactNode;
  action?: ReactNode;
  children: ReactNode;
}) {
  return (
    <div className="page-frame">
      <div className="page-intro">
        <div className="page-intro-copy">
          {titleSlot ?? <h2>{title}</h2>}
          {description && <p>{description}</p>}
        </div>
        {action && <div className="page-intro-action">{action}</div>}
      </div>
      {children}
    </div>
  );
}

export function Panel({
  title,
  icon: Icon,
  className = "",
  action,
  children
}: {
  title: string;
  icon: LucideIcon;
  className?: string;
  action?: ReactNode;
  children: ReactNode;
}) {
  return (
    <section className={`panel ${className}`}>
      <div className="panel-header">
        <div className="panel-title">
          <Icon size={18} />
          <span>{title}</span>
        </div>
        {action}
      </div>
      <div className="panel-body">{children}</div>
    </section>
  );
}
