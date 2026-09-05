/**
 * Minimal inline-SVG icon set — hand-rolled, bundled at build time, no icon
 * font or CDN (product-expense-manager.md's offline constraint). Outline
 * style, 24x24 viewBox, `currentColor` stroke so every icon inherits its
 * container's text color/theme automatically.
 */
import type { SVGProps } from "react";

export type IconProps = SVGProps<SVGSVGElement>;

const base = {
  viewBox: "0 0 24 24",
  fill: "none",
  stroke: "currentColor",
  strokeWidth: 1.5,
  strokeLinecap: "round" as const,
  strokeLinejoin: "round" as const,
};

export function DashboardIcon(props: IconProps) {
  return (
    <svg {...base} {...props}>
      <rect x="3.75" y="3.75" width="7" height="7" rx="1.25" />
      <rect x="13.25" y="3.75" width="7" height="4.5" rx="1.25" />
      <rect x="13.25" y="10.75" width="7" height="9.5" rx="1.25" />
      <rect x="3.75" y="13.25" width="7" height="7" rx="1.25" />
    </svg>
  );
}

export function ExpensesIcon(props: IconProps) {
  return (
    <svg {...base} {...props}>
      <path d="M6 3.75h9.5L20 8.25V19.5a0.75 0.75 0 0 1-0.75 0.75H6a0.75 0.75 0 0 1-0.75-0.75V4.5A0.75 0.75 0 0 1 6 3.75Z" />
      <path d="M15 3.75V8h4.25" />
      <path d="M9 12.5h6M9 15.5h6M9 9.5h2" />
    </svg>
  );
}

export function VendorsIcon(props: IconProps) {
  return (
    <svg {...base} {...props}>
      <path d="M4 9.5 5.5 4.5h13L20 9.5" />
      <path d="M4 9.5a2 2 0 0 0 4 0 2 2 0 0 0 4 0 2 2 0 0 0 4 0 2 2 0 0 0 4 0" />
      <path d="M5 9.5V19a0.75 0.75 0 0 0 0.75 0.75H10V15h4v4.75h4.25A0.75 0.75 0 0 0 19 19V9.5" />
    </svg>
  );
}

export function CategoriesIcon(props: IconProps) {
  return (
    <svg {...base} {...props}>
      <path d="M12.5 4.25 20 8.5v7L12.5 19.75 5 15.5v-7L12.5 4.25Z" />
      <path d="M5 8.5l7.5 4.25L20 8.5" />
      <path d="M12.5 12.75v7" />
    </svg>
  );
}

export function ReportsIcon(props: IconProps) {
  return (
    <svg {...base} {...props}>
      <path d="M6 3.75h8L19.25 9v11a0.75 0.75 0 0 1-0.75 0.75H6a0.75 0.75 0 0 1-0.75-0.75V4.5A0.75 0.75 0 0 1 6 3.75Z" />
      <path d="M14 3.75V9h5.25" />
      <path d="M9 13.5v3M12 11.75v4.75M15 14.75v2.5" />
    </svg>
  );
}

export function SettingsIcon(props: IconProps) {
  return (
    <svg {...base} {...props}>
      <circle cx="12" cy="12" r="2.75" />
      <path d="M12 3.75v2M12 18.25v2M20.25 12h-2M5.75 12h-2M17.66 6.34l-1.42 1.42M7.76 16.24l-1.42 1.42M17.66 17.66l-1.42-1.42M7.76 7.76 6.34 6.34" />
    </svg>
  );
}

export function SunIcon(props: IconProps) {
  return (
    <svg {...base} {...props}>
      <circle cx="12" cy="12" r="4" />
      <path d="M12 2.75v2M12 19.25v2M21.25 12h-2M4.75 12h-2M18.5 5.5l-1.4 1.4M6.9 17.1l-1.4 1.4M18.5 18.5l-1.4-1.4M6.9 6.9 5.5 5.5" />
    </svg>
  );
}

export function MoonIcon(props: IconProps) {
  return (
    <svg {...base} {...props}>
      <path d="M20 14.5A8.5 8.5 0 0 1 9.5 4a7 7 0 1 0 10.5 10.5Z" />
    </svg>
  );
}

export function PlusIcon(props: IconProps) {
  return (
    <svg {...base} {...props}>
      <path d="M12 5v14M5 12h14" />
    </svg>
  );
}

export function TrashIcon(props: IconProps) {
  return (
    <svg {...base} {...props}>
      <path d="M4.75 7h14.5" />
      <path d="M9.5 7V4.75a1 1 0 0 1 1-1h3a1 1 0 0 1 1 1V7" />
      <path d="M6.75 7l0.6 12.1a1 1 0 0 0 1 0.95h7.3a1 1 0 0 0 1-0.95L17.25 7" />
      <path d="M10 10.75v6M14 10.75v6" />
    </svg>
  );
}

export function PencilIcon(props: IconProps) {
  return (
    <svg {...base} {...props}>
      <path d="M16.5 3.5a1.914 1.914 0 0 1 2.707 2.707L7.5 17.914 3.5 18.5l0.586-4L16.5 3.5Z" />
      <path d="M14.5 5.5l3 3" />
    </svg>
  );
}

export function XIcon(props: IconProps) {
  return (
    <svg {...base} {...props}>
      <path d="M6 6l12 12M18 6 6 18" />
    </svg>
  );
}

export function ChevronLeftIcon(props: IconProps) {
  return (
    <svg {...base} {...props}>
      <path d="M14.5 5.5 8 12l6.5 6.5" />
    </svg>
  );
}

export function SearchIcon(props: IconProps) {
  return (
    <svg {...base} {...props}>
      <circle cx="11" cy="11" r="6.5" />
      <path d="M20 20l-4.35-4.35" />
    </svg>
  );
}

export function ReceiptIcon(props: IconProps) {
  return (
    <svg {...base} {...props}>
      <path d="M6.5 3.75h11a0.75 0.75 0 0 1 0.75 0.75v15.4a0.35 0.35 0 0 1-0.55 0.29l-1.86-1.24a0.35 0.35 0 0 0-0.39 0l-1.73 1.16a0.35 0.35 0 0 1-0.39 0l-1.73-1.16a0.35 0.35 0 0 0-0.39 0l-1.73 1.16a0.35 0.35 0 0 1-0.39 0l-1.73-1.16a0.35 0.35 0 0 0-0.39 0L5.85 20.14a0.35 0.35 0 0 1-0.55-0.29V4.5a0.75 0.75 0 0 1 0.75-0.75Z" />
      <path d="M9 8.5h6M9 12h6M9 15.5h3.5" />
    </svg>
  );
}
