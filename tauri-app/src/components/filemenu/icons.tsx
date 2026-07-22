import type { SVGProps } from "react";

function Svg({ children, ...props }: SVGProps<SVGSVGElement>) {
  return (
    <svg
      width={15}
      height={15}
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth={1.8}
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
      {...props}
    >
      {children}
    </svg>
  );
}

export function IconSave(props: SVGProps<SVGSVGElement>) {
  return (
    <Svg {...props}>
      <path d="M19 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h11l5 5v11a2 2 0 0 1-2 2z" />
      <path d="M17 21v-8H7v8" />
      <path d="M7 3v4h8" />
    </Svg>
  );
}

export function IconSaveAs(props: SVGProps<SVGSVGElement>) {
  return (
    <Svg {...props}>
      <path d="M12 20h9" />
      <path d="M16.5 3.5a2.1 2.1 0 0 1 3 3L7 19l-4 1 1-4Z" />
    </Svg>
  );
}

export function IconFolderOpen(props: SVGProps<SVGSVGElement>) {
  return (
    <Svg {...props}>
      <path d="M6 14 4 20h15.5a1.5 1.5 0 0 0 1.4-1L23 13a1 1 0 0 0-1-1.3H8a2 2 0 0 0-2 1.3z" />
      <path d="M4 20V5a2 2 0 0 1 2-2h4l2 3h7a2 2 0 0 1 2 2v3.7" />
    </Svg>
  );
}

export function IconFork(props: SVGProps<SVGSVGElement>) {
  return (
    <Svg {...props}>
      <circle cx={6} cy={5} r={2.2} />
      <circle cx={18} cy={5} r={2.2} />
      <circle cx={12} cy={19} r={2.2} />
      <path d="M6 7.2V9a2 2 0 0 0 2 2h8a2 2 0 0 0 2-2V7.2" />
      <path d="M12 11v5.8" />
    </Svg>
  );
}

export function IconReveal(props: SVGProps<SVGSVGElement>) {
  return (
    <Svg {...props}>
      <path d="M15 3h6v6" />
      <path d="M10 14 21 3" />
      <path d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6" />
    </Svg>
  );
}

export function IconTrash(props: SVGProps<SVGSVGElement>) {
  return (
    <Svg {...props}>
      <path d="M3 6h18" />
      <path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6" />
      <path d="M8 6V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2" />
    </Svg>
  );
}
