interface PearLogoProps {
  size?: number;
  className?: string;
}

/** App logo: white pear glyph on an accent rounded-square tile. */
export function PearLogo({ size = 24, className }: PearLogoProps) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 24 24"
      className={className}
      aria-hidden="true"
      focusable="false"
    >
      <rect width="24" height="24" rx="5.5" fill="var(--accent)" />
      {/* pear body */}
      <path
        d="M12 8.4c-.7 0-1.3.7-1.7 1.9-.3 1-.8 1.6-1.4 2.3-.8.9-1.3 1.9-1.3 3.1 0 2.5 1.9 4.3 4.4 4.3s4.4-1.8 4.4-4.3c0-1.2-.5-2.2-1.3-3.1-.6-.7-1.1-1.3-1.4-2.3-.4-1.2-1-1.9-1.7-1.9z"
        fill="white"
      />
      {/* stem */}
      <path
        d="M12 8.4c0-1.1.3-2 1-2.7"
        stroke="white"
        strokeWidth="1.4"
        strokeLinecap="round"
        fill="none"
      />
      {/* leaf */}
      <path
        d="M13.2 5.9c.4-1.4 1.5-2.3 3.1-2.4-.2 1.6-1.2 2.6-3.1 2.4z"
        fill="white"
      />
    </svg>
  );
}
