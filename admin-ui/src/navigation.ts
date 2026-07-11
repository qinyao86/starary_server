import { Archive, BarChart3, HardDrive, Library, ListChecks, Settings, ShieldCheck, Users } from "lucide-react";
import { sectionStorageKey } from "./constants";
import type { NavItem, Section } from "./types";

export const navItems: NavItem[] = [
  { id: "libraries", icon: Library, label: "libraries" },
  { id: "storage", icon: HardDrive, label: "storage" },
  { id: "users", icon: Users, label: "users" },
  { id: "permissions", icon: ShieldCheck, label: "permissions" },
  { id: "statistics", icon: BarChart3, label: "statistics" },
  { id: "activity", icon: ListChecks, label: "activity" },
  { id: "backups", icon: Archive, label: "backups" },
  { id: "settings", icon: Settings, label: "settings" }
];

export function readStoredSection(): Section {
  const stored = localStorage.getItem(sectionStorageKey);
  if (stored === "overview") return "statistics";
  return navItems.some((item) => item.id === stored) ? (stored as Section) : "libraries";
}
