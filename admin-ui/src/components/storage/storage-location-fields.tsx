import type { TranslatorContext } from "../../types";
import { TextField } from "../common";

export function StorageLocationFields({
  kind,
  location,
  allowedKinds,
  locationLabel,
  t,
  onKindChange,
  onLocationChange
}: TranslatorContext & {
  kind: string;
  location: string;
  allowedKinds?: string[];
  locationLabel?: string;
  onKindChange: (kind: string) => void;
  onLocationChange: (location: string) => void;
}) {
  const options = [
    { value: "server_filesystem", label: t("storageKindServerFilesystem") },
    { value: "smb", label: t("storageKindSmb") },
    { value: "s3", label: t("storageKindS3") }
  ].filter((option) => !allowedKinds || allowedKinds.includes(option.value));
  const placeholder =
    kind === "s3"
      ? t("objectStorageLocationPlaceholder")
      : kind === "server_filesystem"
        ? t("localFolderLocationPlaceholder")
        : t("sharedFolderLocationPlaceholder");

  return (
    <div className="storage-location-fields">
      <TextField
        autoFocus
        required
        label={locationLabel ?? t("libraryStorageLocation")}
        placeholder={placeholder}
        value={location}
        onChange={onLocationChange}
      />
      <div className="storage-kind-radios" role="radiogroup" aria-label={t("storageLocationType")}>
        {options.map((option) => (
          <label className="storage-kind-radio" key={option.value}>
            <input
              checked={kind === option.value}
              name="library-storage-kind"
              type="radio"
              value={option.value}
              onChange={() => onKindChange(option.value)}
            />
            <span>{option.label}</span>
          </label>
        ))}
      </div>
    </div>
  );
}
