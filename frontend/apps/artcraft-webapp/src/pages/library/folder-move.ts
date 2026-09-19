interface FolderNode {
  id: string;
  name: string;
  parentId: string | null;
}

/** Exclude the source subtree so a move can never create a parent cycle. */
export function getFolderMoveDestinations<T extends FolderNode>(
  folders: T[],
  sourceId: string,
): T[] {
  const children = new Map<string, string[]>();
  for (const folder of folders) {
    if (!folder.parentId) continue;
    const siblings = children.get(folder.parentId) ?? [];
    siblings.push(folder.id);
    children.set(folder.parentId, siblings);
  }
  const excluded = new Set<string>();
  const pending = [sourceId];
  while (pending.length) {
    const id = pending.pop()!;
    if (excluded.has(id)) continue;
    excluded.add(id);
    pending.push(...(children.get(id) ?? []));
  }
  return folders.filter((folder) => !excluded.has(folder.id));
}

/** Full paths distinguish identically named folders in different branches. */
export function getFolderPath(
  folder: FolderNode,
  byId: Map<string, FolderNode>,
): string {
  const names: string[] = [];
  const seen = new Set<string>();
  let current: FolderNode | undefined = folder;
  while (current && !seen.has(current.id)) {
    seen.add(current.id);
    names.unshift(current.name);
    current = current.parentId ? byId.get(current.parentId) : undefined;
  }
  return names.join(" / ");
}
