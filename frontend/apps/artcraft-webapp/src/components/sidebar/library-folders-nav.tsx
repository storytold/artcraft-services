import { SidebarActiveIndicator } from "./sidebar-active-indicator";
import { useMemo, useState } from "react";
import { Link, useNavigate } from "react-router-dom";
import { AnimatePresence, motion } from "framer-motion";
import {
  ChevronRightIcon,
  FolderIcon,
  FolderOpenIcon,
  Grid3x3Icon,
  PlusIcon,
  StarIcon,
} from "lucide-react";
import { compareFolders } from "@storyteller/ui-gallery-modal";
import { EASE_EMPHASIS } from "../../lib/motion";
import {
  SidebarGroup,
  SidebarGroupContent,
  SidebarGroupLabel,
  SidebarMenu,
  SidebarMenuButton,
  SidebarMenuItem,
} from "../ui/sidebar";
import { useLibraryFoldersStore } from "../../pages/library/library-folders-store";
import { LibraryTagsNav } from "./library-tags-nav";
import { useFolderDrag } from "../../pages/library/use-folder-drag";

/**
 * The "Library" sidebar section: Unsorted + Folders entries, plus the user's
 * root folders while in the library area. Folder rows carry `data-folder-id`
 * (drag-drop target) and right-click opens the page's folder context menu.
 */
export function LibraryFoldersNav({
  pathname,
  onNavClick,
}: {
  pathname: string;
  onNavClick: () => void;
}) {
  const navigate = useNavigate();
  const folderDrag = useFolderDrag();
  const onFolders =
    pathname === "/library/folders" || pathname.startsWith("/library/folder_");
  const onFolderless = pathname === "/library/folderless";
  const onTags =
    pathname === "/library/tags" || pathname.startsWith("/library/tag_");
  const onUnsorted =
    !onFolders &&
    !onFolderless &&
    !onTags &&
    (pathname === "/library" || pathname.startsWith("/library/"));
  const inLibraryArea = onUnsorted || onFolders || onFolderless || onTags;

  const folders = useLibraryFoldersStore((s) => s.folders);
  const activeFolderId = useLibraryFoldersStore((s) => s.activeFolderId);
  const openNewFolderModal = useLibraryFoldersStore(
    (s) => s.openNewFolderModal,
  );
  const setContextMenu = useLibraryFoldersStore((s) => s.setContextMenu);

  // Folders are the primary organization tool — default expanded.
  const [expanded, setExpanded] = useState(true);

  const rootFolders = useMemo(
    () => folders.filter((f) => !f.parentId).sort(compareFolders),
    [folders],
  );

  // Reveal the active folder's ancestors and its immediate children.
  const activeBranch = useMemo(() => {
    const byId = new Map(folders.map((f) => [f.id, f]));
    const seen = new Set<string>();
    let cursor = activeFolderId ? byId.get(activeFolderId) : undefined;
    while (cursor && !seen.has(cursor.id)) {
      seen.add(cursor.id);
      cursor = cursor.parentId ? byId.get(cursor.parentId) : undefined;
    }
    return seen;
  }, [folders, activeFolderId]);
  const [branchOverrides, setBranchOverrides] = useState<{
    activeId: string | null;
    values: Record<string, boolean>;
  }>({ activeId: null, values: {} });
  const visibleFolders = useMemo(() => {
    const children = new Map<string | null, typeof folders>();
    for (const folder of folders) {
      const siblings = children.get(folder.parentId) ?? [];
      siblings.push(folder);
      children.set(folder.parentId, siblings);
    }
    for (const siblings of children.values()) siblings.sort(compareFolders);
    const rows: {
      folder: (typeof folders)[number];
      depth: number;
      hasChildren: boolean;
      open: boolean;
    }[] = [];
    const seen = new Set<string>();
    const visit = (parentId: string | null, depth: number) => {
      for (const folder of children.get(parentId) ?? []) {
        if (seen.has(folder.id)) continue;
        seen.add(folder.id);
        const override =
          branchOverrides.activeId === activeFolderId
            ? branchOverrides.values[folder.id]
            : undefined;
        const open = override ?? activeBranch.has(folder.id);
        rows.push({
          folder,
          depth,
          hasChildren: children.has(folder.id),
          open,
        });
        if (open) visit(folder.id, depth + 1);
      }
    };
    visit(null, 0);
    return rows;
  }, [folders, activeBranch, activeFolderId, branchOverrides]);

  const goToFolder = (id: string) => {
    navigate(`/library/${id}`);
    onNavClick();
  };

  return (
    <SidebarGroup {...folderDrag}>
      <div className="flex items-center justify-between">
        <SidebarGroupLabel>Assets</SidebarGroupLabel>
        {inLibraryArea && (
          <button
            type="button"
            onClick={() => openNewFolderModal(null)}
            aria-label="New folder"
            className="mr-1 flex h-5 w-5 items-center justify-center text-sidebar-foreground/50 hover:bg-sidebar-accent hover:text-sidebar-foreground transition-colors group-data-[collapsible=icon]:hidden"
          >
            <PlusIcon className="text-xs" />
          </button>
        )}
      </div>
      <SidebarGroupContent>
        <SidebarMenu>
          <SidebarMenuItem>
            {onUnsorted && <SidebarActiveIndicator />}
            <SidebarMenuButton asChild isActive={onUnsorted} tooltip="Library">
              <Link to="/library" onClick={onNavClick}>
                <Grid3x3Icon />
                <span>Library</span>
              </Link>
            </SidebarMenuButton>
          </SidebarMenuItem>
          <SidebarMenuItem>
            {onFolderless && <SidebarActiveIndicator />}
            <SidebarMenuButton
              asChild
              isActive={onFolderless}
              tooltip="Unfoldered"
            >
              <Link to="/library/folderless" onClick={onNavClick}>
                <FolderOpenIcon />
                <span>Unfoldered</span>
              </Link>
            </SidebarMenuButton>
          </SidebarMenuItem>
          <SidebarMenuItem>
            {onFolders && !activeFolderId && <SidebarActiveIndicator />}
            <SidebarMenuButton
              asChild
              isActive={onFolders && !activeFolderId}
              tooltip="Folders"
            >
              <Link
                to="/library/folders"
                onClick={onNavClick}
                data-folder-root
                className="[&.folder-drag-over]:bg-primary/20"
              >
                <FolderIcon />
                <span>Folders</span>
              </Link>
            </SidebarMenuButton>
            {inLibraryArea && rootFolders.length > 0 && (
              <button
                type="button"
                onClick={() => setExpanded((v) => !v)}
                aria-label={expanded ? "Collapse folders" : "Expand folders"}
                className="absolute right-1 top-1/2 -translate-y-1/2 flex h-5 w-5 items-center justify-center text-sidebar-foreground/50 hover:bg-sidebar-accent hover:text-sidebar-foreground transition-colors group-data-[collapsible=icon]:hidden"
              >
                <ChevronRightIcon
                  className={`text-[10px] transition-transform duration-200 ${expanded ? "rotate-90" : ""}`}
                />
              </button>
            )}
          </SidebarMenuItem>

          <AnimatePresence initial={false}>
            {inLibraryArea && expanded && rootFolders.length > 0 && (
              <motion.li
                key="folder-rows"
                initial={{ height: 0, opacity: 0 }}
                animate={{ height: "auto", opacity: 1 }}
                exit={{ height: 0, opacity: 0 }}
                transition={{ duration: 0.28, ease: EASE_EMPHASIS }}
                className="list-none overflow-hidden group-data-[collapsible=icon]:hidden"
              >
                <ul className="flex w-full min-w-0 flex-col gap-0.5">
                  {visibleFolders.map(
                    ({ folder, depth, hasChildren, open }) => (
                      <SidebarMenuItem key={folder.id}>
                        {onFolders && activeFolderId === folder.id && (
                          <SidebarActiveIndicator />
                        )}
                        <SidebarMenuButton
                          isActive={onFolders && activeFolderId === folder.id}
                          tooltip={folder.name}
                          data-folder-id={folder.id}
                          draggable
                          style={{ paddingLeft: 28 + depth * 16 }}
                          onClick={() => goToFolder(folder.id)}
                          onContextMenu={(e) => {
                            e.preventDefault();
                            setContextMenu({
                              folderId: folder.id,
                              x: e.clientX,
                              y: e.clientY,
                            });
                          }}
                          className="pl-5 [&.folder-drag-over]:bg-primary/20 [&.folder-drag-over]:text-sidebar-foreground"
                        >
                          <FolderIcon
                            className={folder.colorCode ? "" : "text-primary"}
                            style={
                              folder.colorCode
                                ? { color: folder.colorCode }
                                : undefined
                            }
                          />
                          <span className="truncate">{folder.name}</span>
                          {folder.hasStar && (
                            <StarIcon className="ml-auto text-[10px] text-amber-400" />
                          )}
                        </SidebarMenuButton>
                        {hasChildren && (
                          <button
                            type="button"
                            aria-label={`${open ? "Collapse" : "Expand"} ${folder.name}`}
                            aria-expanded={open}
                            className="absolute top-1/2 -translate-y-1/2 flex h-5 w-5 items-center justify-center rounded hover:bg-sidebar-accent"
                            style={{ left: 4 + depth * 16 }}
                            onClick={() =>
                              setBranchOverrides((previous) => ({
                                activeId: activeFolderId,
                                values: {
                                  ...(previous.activeId === activeFolderId
                                    ? previous.values
                                    : {}),
                                  [folder.id]: !open,
                                },
                              }))
                            }
                          >
                            <ChevronRightIcon
                              className={`h-3 w-3 transition-transform ${open ? "rotate-90" : ""}`}
                            />
                          </button>
                        )}
                      </SidebarMenuItem>
                    ),
                  )}
                </ul>
              </motion.li>
            )}
          </AnimatePresence>

          <LibraryTagsNav pathname={pathname} onNavClick={onNavClick} />
        </SidebarMenu>
      </SidebarGroupContent>
    </SidebarGroup>
  );
}
