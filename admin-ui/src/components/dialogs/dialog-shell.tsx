import { useEffect, type ReactNode } from "react";
import { X } from "lucide-react";
import { Button } from "@/components/ui/button";

export function DialogShell({
  children,
  className,
  closeLabel,
  headerContent,
  open,
  subtitle,
  title,
  titleId,
  onClose
}: {
  children: ReactNode;
  className: string;
  closeLabel: string;
  headerContent?: ReactNode;
  open: boolean;
  subtitle: string;
  title: string;
  titleId: string;
  onClose: () => void;
}) {
  useEffect(() => {
    if (!open) return;

    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") onClose();
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [onClose, open]);

  if (!open) return null;

  return (
    <div
      className={`dialog-backdrop ${className}-backdrop`}
      role="presentation"
      onMouseDown={(event) => {
        if (event.target === event.currentTarget) onClose();
      }}
    >
      <section className={`dialog-panel ${className}`} role="dialog" aria-modal="true" aria-labelledby={titleId}>
        <div className={`dialog-header${headerContent ? " has-custom-content" : ""}`}>
          {headerContent ?? (
            <div>
              <h2 className="dialog-title" id={titleId}>{title}</h2>
              <p className="dialog-subtitle">{subtitle}</p>
            </div>
          )}
          <Button className="dialog-close" type="button" variant="ghost" size="icon" aria-label={closeLabel} onClick={onClose}>
            <X size={16} />
          </Button>
        </div>
        {children}
      </section>
    </div>
  );
}
