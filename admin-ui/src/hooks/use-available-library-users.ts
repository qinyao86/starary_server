import { useMemo } from "react";
import type { LibraryMember, TeamUser } from "../api";

export function useAvailableLibraryUsers(
  users: TeamUser[],
  members: LibraryMember[]
) {
  return useMemo(
    () => users.filter((user) => !members.some((member) => member.userId === user.id)),
    [members, users]
  );
}
