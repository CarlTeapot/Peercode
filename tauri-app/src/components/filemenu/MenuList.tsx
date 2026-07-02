import type { ComponentType, SVGProps } from "react";
import {
  IconExport,
  IconFolderDown,
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
  exportFileName: string | null;
  busy: boolean;
  onSave: () => void;
  onSaveAs: () => void;
  onSaveTo: () => void;
  onExportLinked: () => void;
  onExportAs: () => void;
  onOpenLibrary: () => void;
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
        detail={props.currentName ?? undefined}
        disabled={busy}
        onClick={props.onSave}
      />
      <MenuItem
        icon={IconSaveAs}
        label="Save as…"
        disabled={busy}
        onClick={props.onSaveAs}
      />
      <MenuItem
        icon={IconFolderDown}
        label="Save to…"
        disabled={busy}
        onClick={props.onSaveTo}
      />
      <div className="file-dropdown-separator" />
      <div className="file-dropdown-section">Export</div>
      {props.exportFileName && (
        <MenuItem
          icon={IconExport}
          label="Export to"
          detail={props.exportFileName}
          disabled={busy}
          onClick={props.onExportLinked}
        />
      )}
      <MenuItem
        icon={IconExport}
        label="Export as…"
        disabled={busy}
        onClick={props.onExportAs}
      />
      <div className="file-dropdown-separator" />
      <div className="file-dropdown-section">Open</div>
      <MenuItem
        icon={IconList}
        label="Open…"
        disabled={busy}
        onClick={props.onOpenLibrary}
      />
      <MenuItem
        icon={IconFolderOpen}
        label="Open from…"
        disabled={busy}
        onClick={props.onOpenFrom}
      />
      <MenuItem
        icon={IconFork}
        label="Fork…"
        disabled={busy}
        onClick={props.onFork}
      />
    </>
  );
}
