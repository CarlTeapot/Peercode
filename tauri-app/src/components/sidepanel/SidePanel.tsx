import type { PanelSection } from "../siderail/SideRail";
import {
  CollaborateSection,
  type CollaborateSectionProps,
} from "./CollaborateSection";
import { FilesSection } from "./FilesSection";
import { YouSection } from "./YouSection";
import type { FileMenuApi } from "../filemenu/useFileMenu";
import "./SidePanel.css";

const SECTION_TITLES: Record<PanelSection, string> = {
  collab: "Collaborate",
  files: "Files",
  you: "You",
};

interface SidePanelProps {
  section: PanelSection;
  collab: CollaborateSectionProps;
  files: { menu: FileMenuApi };
  you: { username: string; onUsernameChange: (name: string) => void };
}

/** Slide-in panel between the rail and the editor; one section at a time. */
export function SidePanel({ section, collab, files, you }: SidePanelProps) {
  return (
    <aside className="side-panel" aria-label={SECTION_TITLES[section]}>
      <div className="side-panel-title">{SECTION_TITLES[section]}</div>
      {section === "collab" && <CollaborateSection {...collab} />}
      {section === "files" && <FilesSection menu={files.menu} />}
      {section === "you" && (
        <YouSection
          username={you.username}
          onUsernameChange={you.onUsernameChange}
        />
      )}
    </aside>
  );
}
