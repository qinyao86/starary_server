import type { PageContext, Section } from "../types";
import {
  ActivityPage,
  BackupsPage,
  LibrariesPage,
  PermissionsPage,
  SettingsPage,
  StatisticsPage,
  StoragePage,
  UsersPage
} from ".";

export function renderSection(section: Section, context: PageContext) {
  switch (section) {
    case "libraries":
      return <LibrariesPage {...context} />;
    case "users":
      return <UsersPage {...context} />;
    case "permissions":
      return <PermissionsPage {...context} />;
    case "storage":
      return <StoragePage {...context} />;
    case "statistics":
      return <StatisticsPage {...context} />;
    case "activity":
      return <ActivityPage {...context} />;
    case "backups":
      return <BackupsPage {...context} />;
    case "settings":
      return <SettingsPage {...context} />;
    default:
      return <LibrariesPage {...context} />;
  }
}
