/** Minimal centered overlay — shared by every "quick add" inline form (ui-ux.md §3's dual entry-point rule). */
export function Modal({ children, onClose }: { children: React.ReactNode; onClose: () => void }) {
  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-4"
      onMouseDown={(e) => {
        if (e.target === e.currentTarget) onClose();
      }}
    >
      <div className="w-full max-w-md">{children}</div>
    </div>
  );
}
