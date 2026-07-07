import { Activity, Archive, BarChart3, HardDrive, Library, ListChecks, Server, Settings, ShieldCheck, Users } from "lucide-react";
import { sectionStorageKey } from "./constants";
import type { NavItem, Section } from "./types";

export const navItems: NavItem[] = [
  { id: "overview", icon: BarChart3, label: "overview" },
  { id: "service", icon: Server, label: "service" },
  { id: "libraries", icon: Library, label: "libraries" },
  { id: "users", icon: Users, label: "users" },
  { id: "permissions", icon: ShieldCheck, label: "permissions" },
  { id: "storage", icon: HardDrive, label: "storage" },
  { id: "statistics", icon: Activity, label: "statistics" },
  { id: "activity", icon: ListChecks, label: "activity" },
  { id: "backups", icon: Archive, label: "backups" },
  { id: "settings", icon: Settings, label: "settings" }
];

export function readStoredSection(): Section {
  const stored = localStorage.getItem(sectionStorageKey);
  return navItems.some((item) => item.id === stored) ? (stored as Section) : "overview";
}
