import { ImageUp, Library } from "lucide-react";
import { useEffect, useState } from "react";

import type { TeamLibrary } from "../../api";
import type { TranslatorContext } from "../../types";
import { resolveLibraryIconUrl } from "../../utils/library-icons";

export function LibraryIcon({
  library,
  token,
  t,
  editable = false,
  busy = false,
  size = "md",
  onChange,
}: TranslatorContext & {
  library: TeamLibrary;
  token: string | null;
  editable?: boolean;
  busy?: boolean;
  size?: "sm" | "md" | "lg";
  onChange?: () => void;
}) {
  const iconUrl = resolveLibraryIconUrl(library, token);
  const [imageFailed, setImageFailed] = useState(false);

  useEffect(() => {
    setImageFailed(false);
  }, [iconUrl]);

  const content = iconUrl && !imageFailed ? (
    <img alt="" src={iconUrl} onError={() => setImageFailed(true)} />
  ) : (
    <Library aria-hidden="true" size={size === "lg" ? 24 : size === "sm" ? 18 : 20} />
  );

  if (!editable || !onChange) {
    return <span className={`library-identity-icon is-${size}`}>{content}</span>;
  }

  return (
    <button
      aria-label={t("changeLibraryIcon")}
      className={`library-identity-icon is-${size} is-editable${busy ? " is-busy" : ""}`}
      disabled={busy}
      title={t("changeLibraryIcon")}
      type="button"
      onClick={(event) => {
        event.stopPropagation();
        onChange();
      }}
      onKeyDown={(event) => event.stopPropagation()}
    >
      {content}
      <span className="library-identity-icon-edit" aria-hidden="true">
        <ImageUp size={size === "lg" ? 15 : size === "sm" ? 12 : 13} />
      </span>
    </button>
  );
}
