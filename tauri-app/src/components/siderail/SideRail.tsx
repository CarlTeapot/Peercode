import type React from "react";
import "./SideRail.css";

export type PanelSection = "collab" | "files" | "you";

const SECTION_META: Record<
  PanelSection,
  { title: string; icon: () => React.ReactElement }
> = {
  collab: { title: "Collaborate", icon: IconCollab },
  files: { title: "Files", icon: IconFiles },
  you: { title: "You", icon: IconUser },
};

interface SideRailProps {
  /** Sections to show, top to bottom (lets tasks add files/you later). */
  sections: PanelSection[];
  active: PanelSection | null;
  onSelect: (section: PanelSection) => void;
  panelOpen: boolean;
  onTogglePanel: () => void;
  theme: "dark" | "light";
  onToggleTheme: () => void;
}

/** Far-left icon rail: panel sections, theme toggle, collapse chevron. */
export function SideRail({
  sections,
  active,
  onSelect,
  panelOpen,
  onTogglePanel,
  theme,
  onToggleTheme,
}: SideRailProps) {
  return (
    <nav className="side-rail" aria-label="Sidebar">
      {sections.map((s) => {
        const Icon = SECTION_META[s].icon;
        return (
          <button
            key={s}
            className={"rail-btn" + (active === s ? " active" : "")}
            title={SECTION_META[s].title}
            aria-pressed={active === s}
            onClick={() => onSelect(s)}
          >
            <Icon />
          </button>
        );
      })}
      <button
        className="rail-btn"
        title={
          theme === "dark" ? "Switch to light mode" : "Switch to dark mode"
        }
        onClick={onToggleTheme}
      >
        <span className="rail-glyph">◐</span>
      </button>
      <div className="rail-spacer" />
      <button
        className="rail-btn"
        title={panelOpen ? "Collapse panel" : "Expand panel"}
        onClick={onTogglePanel}
      >
        <span className="rail-glyph">{panelOpen ? "«" : "»"}</span>
      </button>
    </nav>
  );
}

function IconCollab() {
  return (
    <svg
      width="18"
      height="18"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.8"
      strokeLinecap="round"
      aria-hidden="true"
    >
      <circle cx="6" cy="12" r="2.6" />
      <circle cx="17.5" cy="5.5" r="2.6" />
      <circle cx="17.5" cy="18.5" r="2.6" />
      <path d="M8.4 10.7l6.7-3.8M8.4 13.3l6.7 3.8" />
    </svg>
  );
}

function IconFiles() {
  return (
    <svg
      width="18"
      height="18"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.8"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      <path d="M7 3h7l4 4v14H7z" />
      <path d="M14 3v4h4" />
    </svg>
  );
}

function IconUser() {
  return (
    <svg
      width="18"
      height="18"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.8"
      strokeLinecap="round"
      aria-hidden="true"
    >
      <circle cx="12" cy="8.5" r="3.4" />
      <path d="M5 20c1.3-3.3 4-5 7-5s5.7 1.7 7 5" />
    </svg>
  );
}
