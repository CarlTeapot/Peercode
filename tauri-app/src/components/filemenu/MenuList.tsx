import type { ComponentType, SVGProps } from "react";
import {
  IconFolderOpen,
  IconFork,
  IconList,
  IconSave,
  IconSaveAs,
} from "./icons";

interface MenuItemProps {
  icon: ComponentType<SVGProps<SVGSVGElement>>;
  label: string;
  detail?: string;
  disabled: boolean;
  onClick: () => void;
}

function MenuItem({
  icon: Icon,
  label,
  detail,
  disabled,
  onClick,
}: MenuItemProps) {
  return (
    <button
      className="file-dropdown-item"
      onClick={onClick}
      disabled={disabled}
    >
      <Icon className="file-dropdown-item-icon" />
      <span className="file-dropdown-item-label">{label}</span>
      {detail && <span className="file-dropdown-item-detail">{detail}</span>}
    </button>
  );
}

interface MenuListProps {
  currentName: string | null;
  busy: boolean;
  onSave: () => void;
  onSaveAs: () => void;
  onOpenRecents: () => void;
  onOpenFrom: () => void;
  onFork: () => void;
}

export function MenuList(props: MenuListProps) {
  const { busy } = props;
  return (
    <>
      <div className="file-dropdown-section">Document</div>
      <MenuItem
        icon={IconSave}
        label="Save"
        detail={props.currentName ?? "untitled"}
        disabled={busy}
        onClick={props.onSave}
      />
      <MenuItem
        icon={IconSaveAs}
        label="Save as…"
        disabled={busy}
        onClick={props.onSaveAs}
      />
      <div className="file-dropdown-separator" />
      <div className="file-dropdown-section">Open</div>
      <MenuItem
        icon={IconList}
        label="Open…"
        disabled={busy}
        onClick={props.onOpenRecents}
      />
      <MenuItem
        icon={IconFolderOpen}
        label="Open from…"
        disabled={busy}
        onClick={props.onOpenFrom}
      />
      <MenuItem
        icon={IconFork}
        label="Fork"
        disabled={busy}
        onClick={props.onFork}
      />
    </>
  );
}
