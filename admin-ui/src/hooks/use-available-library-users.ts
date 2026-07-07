import { useEffect, useMemo } from "react";
import type { LibraryMember, TeamUser } from "../api";

export function useAvailableLibraryUsers(
  users: TeamUser[],
  members: LibraryMember[],
  selectedUserId: string,
  setSelectedUserId: (userId: string) => void
) {
  const availableUsers = useMemo(
    () => users.filter((user) => !members.some((member) => member.userId === user.id)),
    [members, users]
  );

  useEffect(() => {
    if (!selectedUserId && availableUsers.length > 0) {
      setSelectedUserId(availableUsers[0].id);
    } else if (selectedUserId && !availableUsers.some((user) => user.id === selectedUserId)) {
      setSelectedUserId(availableUsers[0]?.id ?? "");
    }
  }, [availableUsers, selectedUserId, setSelectedUserId]);

  return availableUsers;
}
