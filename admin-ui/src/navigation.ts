import { BarChart3, DatabaseBackup, HardDrive, Library, Settings, ShieldCheck, Users } from "lucide-react";
import { sectionStorageKey } from "./constants";
import type { NavItem, Section } from "./types";

export const navItems: NavItem[] = [
  { id: "libraries", icon: Library, label: "libraries" },
  { id: "storage", icon: HardDrive, label: "storage" },
  { id: "users", icon: Users, label: "users" },
  { id: "permissions", icon: ShieldCheck, label: "permissions" },
  { id: "statistics", icon: BarChart3, label: "statistics" },
  { id: "backups", icon: DatabaseBackup, label: "data" },
  { id: "settings", icon: Settings, label: "settings" }
];

export function readStoredSection(): Section {
  const pathSection = sectionFromPath(window.location.pathname);
  if (pathSection) return pathSection;
  const stored = localStorage.getItem(sectionStorageKey);
  if (stored === "overview") return "statistics";
  return navItems.some((item) => item.id === stored) ? (stored as Section) : "libraries";
}

export function sectionFromPath(pathname: string): Section | null {
  const segment = pathname.replace(/^\/admin\/?/, "").split("/")[0];
  return navItems.some((item) => item.id === segment) ? (segment as Section) : null;
}

export function sectionPath(section: Section): string {
  return `/admin/${section}`;
}
