import type { LucideIcon } from "lucide-react";
import type { ActivityItem, CurrentUser, LibraryMember, ServerInfo, StorageRoot, TeamLibrary, TeamUser } from "./api";
import type { createTranslator, Language, TranslationKey } from "./i18n";

export type Section =
  | "overview"
  | "libraries"
  | "users"
  | "permissions"
  | "storage"
  | "statistics"
  | "activity"
  | "backups"
  | "settings";

export type DeploymentMode = "local" | "cloud";
export type ColorTheme = "light" | "dark";
export type ApiState = "loading" | "connected" | "unavailable";

export type NavItem = { id: Section; icon: LucideIcon; label: TranslationKey };

export type TranslatorContext = {
  t: ReturnType<typeof createTranslator>;
};

export type PageContext = TranslatorContext & {
  language: Language;
  setLanguage: (language: Language) => void;
  deploymentMode: DeploymentMode;
  setDeploymentMode: (mode: DeploymentMode) => void;
  serviceRunning: boolean;
  serverInfo: ServerInfo | null;
  currentUser: CurrentUser | null;
  libraries: TeamLibrary[];
  users: TeamUser[];
  storageRoots: StorageRoot[];
  libraryMembers: LibraryMember[];
  selectedLibrary: TeamLibrary | null;
  selectedLibraryId: string;
  setSelectedLibraryId: (id: string) => void;
  token: string;
  assetTotal: number;
  activityItems: ActivityItem[];
  libraryActivityItems: ActivityItem[];
  refreshAll: () => Promise<void>;
  setMessage: (message: string | null) => void;
  previewMode: boolean;
};
