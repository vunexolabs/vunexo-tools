import { useCallback, useEffect, useRef, useState } from "react";
import { chooseSavePath } from "../lib/tauri/client";
import { renderInvoicePdf, saveInvoicePdf } from "../lib/tauri/commands";
import { invoicePdfObjectUrl } from "../lib/tauri/types";

/**
 * The two PDF actions from user-flows.md §7 — preview it, or save it through
 * the OS dialog — in one place, so the editor and the invoices list can't
 * drift apart on how either behaves.
 *
 * Preview and save both render server-side from the invoice's *saved* state.
 * Callers holding unsaved edits must flush them first; the editor's autosave
 * normally has, but "Save & PDF" makes it explicit.
 */
export function useInvoicePdf() {
  const [previewUrl, setPreviewUrl] = useState<string | null>(null);
  const [previewTitle, setPreviewTitle] = useState<string>("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<unknown>(null);
  // Object URLs are revoked explicitly — the browser will happily keep a copy
  // of every PDF ever previewed alive for the life of the window otherwise.
  const previewUrlRef = useRef<string | null>(null);
  previewUrlRef.current = previewUrl;

  useEffect(
    () => () => {
      if (previewUrlRef.current) URL.revokeObjectURL(previewUrlRef.current);
    },
    [],
  );

  const closePreview = useCallback(() => {
    setPreviewUrl((current) => {
      if (current) URL.revokeObjectURL(current);
      return null;
    });
  }, []);

  const preview = useCallback(async (invoiceId: number) => {
    setError(null);
    setBusy(true);
    try {
      const rendered = await renderInvoicePdf(invoiceId);
      setPreviewTitle(rendered.file_name);
      setPreviewUrl((current) => {
        if (current) URL.revokeObjectURL(current);
        return invoicePdfObjectUrl(rendered);
      });
    } catch (err) {
      setError(err);
    } finally {
      setBusy(false);
    }
  }, []);

  /**
   * Opens the OS save dialog, then writes the PDF. Resolves to the path
   * written, or `null` if the user dismissed the dialog — a dismissal is not
   * an error and must not surface as one.
   */
  const saveAs = useCallback(async (invoiceId: number, suggestedName: string) => {
    setError(null);
    setBusy(true);
    try {
      const path = await chooseSavePath({
        defaultPath: suggestedName,
        filters: [{ name: "PDF", extensions: ["pdf"] }],
      });
      if (!path) return null;
      await saveInvoicePdf(invoiceId, path);
      return path;
    } catch (err) {
      setError(err);
      return null;
    } finally {
      setBusy(false);
    }
  }, []);

  /**
   * The file name the save dialog should offer. Mirrors the backend's own
   * `suggested_file_name`, which is what actually names the file if the user
   * types nothing — kept here so the dialog opens pre-filled rather than blank.
   */
  const suggestedFileName = useCallback(
    (invoiceNumber: string | null, invoiceId: number) =>
      `Invoice-${invoiceNumber?.trim() ? invoiceNumber.trim().replace(/[/\\:*?"<>|]/g, "-") : `draft-${invoiceId}`}.pdf`,
    [],
  );

  return { previewUrl, previewTitle, busy, error, preview, closePreview, saveAs, suggestedFileName };
}
