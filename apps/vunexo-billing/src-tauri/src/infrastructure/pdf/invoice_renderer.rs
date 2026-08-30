//! The single V1 invoice template (`.ai/product.md` — "exactly one template
//! in V1, no template builder, no theme engine"), composed operator by
//! operator rather than by printing a webview, so column alignment, page
//! breaks, and typography are all under this file's control.
//!
//! It receives an `InvoicePdfDocument` in which every value is already a
//! finished string (`domain::invoice_pdf`). Its only jobs are measurement,
//! placement, and pagination — it must not format a number, a date, or a tax
//! label, and it has no access to an invoice, a customer, or a setting.

use std::path::Path;

use printpdf::{PdfDocument, PdfPage, PdfSaveOptions, Pt, RawImage, XObjectId};

use crate::application::ports::infrastructure_error::InfrastructureError;
use crate::application::ports::invoice_pdf_renderer::InvoicePdfRenderer;
use crate::domain::invoice_pdf::{InvoicePdfDocument, LogoProbe, PdfParty, TotalWeight};

use super::canvas::{rgb, Align, Canvas, TextStyle};
use super::fonts::{FontRole, Fonts};

// ── Page geometry, in millimetres ────────────────────────────────────────
const PAGE_WIDTH: f32 = 210.0; // A4
const PAGE_HEIGHT: f32 = 297.0;
const MARGIN_LEFT: f32 = 16.0;
const MARGIN_RIGHT: f32 = 16.0;
const MARGIN_TOP: f32 = 15.0;
/// Where body content must stop: leaves room for the footer rule and the
/// page counter beneath it.
const BODY_BOTTOM: f32 = PAGE_HEIGHT - 20.0;
const CONTENT_LEFT: f32 = MARGIN_LEFT;
const CONTENT_RIGHT: f32 = PAGE_WIDTH - MARGIN_RIGHT;
const CONTENT_WIDTH: f32 = CONTENT_RIGHT - CONTENT_LEFT;

const LOGO_BOX_WIDTH: f32 = 30.0;
const LOGO_BOX_HEIGHT: f32 = 20.0;
const LOGO_GUTTER: f32 = 6.0;

// ── Line-item column edges, in millimetres from the page left ────────────
const COL_POSITION: f32 = CONTENT_LEFT;
const COL_DESCRIPTION: f32 = CONTENT_LEFT + 8.0;
const COL_QUANTITY_RIGHT: f32 = CONTENT_LEFT + 104.0;
const COL_RATE_RIGHT: f32 = CONTENT_LEFT + 130.0;
const COL_TAX_RIGHT: f32 = CONTENT_LEFT + 146.0;
const COL_AMOUNT_RIGHT: f32 = CONTENT_RIGHT;
/// The description column has to stop short of the quantity column, with a
/// gutter, or a long product name collides with the numbers.
const DESCRIPTION_WIDTH: f32 = COL_QUANTITY_RIGHT - COL_DESCRIPTION - 22.0;

// ── Totals block ─────────────────────────────────────────────────────────
const TOTALS_LEFT: f32 = CONTENT_LEFT + 104.0;
const TOTALS_ROW_HEIGHT: f32 = 6.0;

// ── Type scale, in points ────────────────────────────────────────────────
const SIZE_TITLE: Pt = Pt(19.0);
const SIZE_BUSINESS_NAME: Pt = Pt(15.0);
const SIZE_PARTY_NAME: Pt = Pt(11.0);
const SIZE_BODY: Pt = Pt(9.0);
const SIZE_SMALL: Pt = Pt(8.0);
const SIZE_LABEL: Pt = Pt(7.5);
const SIZE_TOTAL: Pt = Pt(11.0);

const LINE_BODY: f32 = 4.4;
const LINE_SMALL: f32 = 4.0;

// ── Palette ──────────────────────────────────────────────────────────────
const INK: printpdf::Color = rgb(24, 27, 34);
const MUTED: printpdf::Color = rgb(107, 114, 128);
const ACCENT: printpdf::Color = rgb(15, 76, 129);
const RULE: printpdf::Color = rgb(203, 209, 217);
const RULE_LIGHT: printpdf::Color = rgb(219, 224, 231);
const BAND: printpdf::Color = rgb(228, 234, 242);

// ── Named type styles ────────────────────────────────────────────────────
const fn style(role: FontRole, size: Pt, color: printpdf::Color) -> TextStyle {
    TextStyle { role, size, color }
}

/// `TAX INVOICE` in the top right.
const TITLE: TextStyle = style(FontRole::Bold, SIZE_TITLE, ACCENT);
const BUSINESS_NAME: TextStyle = style(FontRole::Bold, SIZE_BUSINESS_NAME, INK);
const PARTY_NAME: TextStyle = style(FontRole::Bold, SIZE_PARTY_NAME, INK);
/// The small all-caps section markers: `BILL TO`, `NOTES`, the column heads.
const SECTION_LABEL: TextStyle = style(FontRole::Bold, SIZE_LABEL, MUTED);
/// A meta row's value (`INV-2026-0007`) under its caption.
const META_VALUE: TextStyle = style(FontRole::Bold, SIZE_BODY, INK);
/// The page counter in the footer.
const FOOTNOTE: TextStyle = style(FontRole::Regular, SIZE_LABEL, MUTED);
const BODY: TextStyle = style(FontRole::Regular, SIZE_BODY, INK);
const BODY_MUTED: TextStyle = style(FontRole::Regular, SIZE_BODY, MUTED);
const SMALL: TextStyle = style(FontRole::Regular, SIZE_SMALL, INK);
const SMALL_MUTED: TextStyle = style(FontRole::Regular, SIZE_SMALL, MUTED);
/// The invoice total and the balance due.
const TOTAL_STRONG: TextStyle = style(FontRole::Bold, SIZE_TOTAL, ACCENT);

/// `InvoicePdfRenderer` backed by `printpdf`. Stateless — one instance is
/// shared by every render.
pub struct PrintpdfInvoiceRenderer;

impl PrintpdfInvoiceRenderer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for PrintpdfInvoiceRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl InvoicePdfRenderer for PrintpdfInvoiceRenderer {
    fn render(&self, document: &InvoicePdfDocument) -> Result<Vec<u8>, InfrastructureError> {
        let mut pdf = PdfDocument::new(&document.title);
        let fonts = Fonts::load(&mut pdf).ok_or_else(|| {
            InfrastructureError::Io("embedded PDF fonts failed to parse".to_string())
        })?;
        let logo = load_logo(&mut pdf, document.logo_path.as_deref());

        let pages = Layout::new(&fonts, document, logo).run();
        let page_count = pages.len();
        let pages = pages
            .into_iter()
            .enumerate()
            .map(|(index, ops)| {
                let mut canvas = Canvas::new(&fonts, PAGE_HEIGHT);
                canvas.hline(
                    CONTENT_LEFT,
                    CONTENT_RIGHT,
                    PAGE_HEIGHT - 16.0,
                    Pt(0.4),
                    RULE_LIGHT,
                );
                canvas.text(
                    PAGE_WIDTH / 2.0,
                    PAGE_HEIGHT - 11.0,
                    &format!("Page {} of {}", index + 1, page_count),
                    &FOOTNOTE,
                    Align::Center,
                );
                let mut all = ops;
                all.extend(canvas.into_ops());
                PdfPage::new(printpdf::Mm(PAGE_WIDTH), printpdf::Mm(PAGE_HEIGHT), all)
            })
            .collect();

        Ok(pdf
            .with_pages(pages)
            .save(&PdfSaveOptions::default(), &mut Vec::new()))
    }

    fn probe_logo(&self, path: &Path) -> LogoProbe {
        // Deliberately the same two steps, in the same order, as `load_logo`
        // — a probe that answered a different question than the renderer
        // asks would be worse than no probe.
        let Ok(bytes) = std::fs::read(path) else {
            return LogoProbe::NotFound;
        };
        match RawImage::decode_from_bytes(&bytes, &mut Vec::new()) {
            Ok(image) => LogoProbe::Ok {
                width_px: image.width as u32,
                height_px: image.height as u32,
            },
            Err(_) => LogoProbe::Unreadable,
        }
    }
}

/// A missing or unreadable logo is never fatal: the invoice is still a valid
/// document without it, and failing the whole render because someone moved an
/// image file would be a worse outcome than printing the business name alone.
fn load_logo(pdf: &mut PdfDocument, path: Option<&str>) -> Option<Logo> {
    let bytes = std::fs::read(path?).ok()?;
    let image = RawImage::decode_from_bytes(&bytes, &mut Vec::new()).ok()?;
    let (width, height) = (image.width, image.height);
    Some(Logo {
        id: pdf.add_image(&image),
        pixel_width: width,
        pixel_height: height,
    })
}

struct Logo {
    id: XObjectId,
    pixel_width: usize,
    pixel_height: usize,
}

/// Walks the document top to bottom, emitting page after page. `y` is the
/// cursor in millimetres from the top of the current page.
struct Layout<'a> {
    fonts: &'a Fonts,
    document: &'a InvoicePdfDocument,
    logo: Option<Logo>,
    /// `₹` when the embedded face can draw the symbol, otherwise `INR ` — see
    /// the module note on `domain::invoice_pdf`.
    money_prefix: String,
    finished_pages: Vec<Vec<printpdf::Op>>,
    canvas: Canvas<'a>,
    y: f32,
}

impl<'a> Layout<'a> {
    fn new(fonts: &'a Fonts, document: &'a InvoicePdfDocument, logo: Option<Logo>) -> Self {
        let money_prefix = match document.currency.symbol.as_deref() {
            Some(symbol) if fonts.can_render(symbol, FontRole::Regular) => symbol.to_string(),
            _ => format!("{} ", document.currency.code),
        };
        let mut layout = Self {
            fonts,
            document,
            logo,
            money_prefix,
            finished_pages: Vec::new(),
            canvas: Canvas::new(fonts, PAGE_HEIGHT),
            y: MARGIN_TOP,
        };
        layout.stamp_watermark();
        layout
    }

    fn run(mut self) -> Vec<Vec<printpdf::Op>> {
        self.draw_letterhead();
        self.draw_parties();
        self.draw_line_items();
        self.draw_totals();
        self.draw_footer_blocks();
        self.finished_pages.push(self.canvas.into_ops());
        self.finished_pages
    }

    fn money(&self, amount: &str) -> String {
        // A negative amount keeps its sign in front of the symbol
        // (`-₹500.00`), which is how a deduction reads on an invoice.
        match amount.strip_prefix('-') {
            Some(rest) => format!("-{}{}", self.money_prefix, rest),
            None => format!("{}{}", self.money_prefix, amount),
        }
    }

    fn stamp_watermark(&mut self) {
        if let Some(watermark) = &self.document.watermark {
            self.canvas.watermark(watermark, PAGE_WIDTH);
        }
    }

    /// Closes the current page and starts a fresh one, re-stamping the
    /// watermark so a multi-page draft is marked on every sheet.
    fn new_page(&mut self) {
        let finished = std::mem::replace(&mut self.canvas, Canvas::new(self.fonts, PAGE_HEIGHT));
        self.finished_pages.push(finished.into_ops());
        self.y = MARGIN_TOP;
        self.stamp_watermark();
    }

    /// Breaks to a new page unless `needed` millimetres still fit.
    fn ensure_space(&mut self, needed: f32) {
        if self.y + needed > BODY_BOTTOM {
            self.new_page();
        }
    }

    // ── Letterhead: logo, business identity, document title and meta ─────

    fn draw_letterhead(&mut self) {
        let business_left = match &self.logo {
            Some(logo) => {
                self.canvas.image(
                    &logo.id,
                    CONTENT_LEFT,
                    self.y,
                    LOGO_BOX_WIDTH,
                    LOGO_BOX_HEIGHT,
                    logo.pixel_width,
                    logo.pixel_height,
                );
                CONTENT_LEFT + LOGO_BOX_WIDTH + LOGO_GUTTER
            }
            None => CONTENT_LEFT,
        };

        let identity_bottom = self.draw_business_identity(business_left);
        let meta_bottom = self.draw_title_and_meta();
        let logo_bottom = if self.logo.is_some() {
            self.y + LOGO_BOX_HEIGHT
        } else {
            self.y
        };

        self.y = identity_bottom.max(meta_bottom).max(logo_bottom) + 5.0;
        self.canvas
            .hline(CONTENT_LEFT, CONTENT_RIGHT, self.y, Pt(0.9), ACCENT);
        self.y += 7.0;
    }

    /// Returns the y the block ended at, so the letterhead can align to
    /// whichever of its three columns is tallest.
    fn draw_business_identity(&mut self, left: f32) -> f32 {
        // Stop short of the title/meta column on the right.
        let available = CONTENT_RIGHT - 62.0 - left;
        let mut y = self.y + 5.0;

        let name = self.fonts.truncate_to_width(
            &self.document.business.name,
            FontRole::Bold,
            SIZE_BUSINESS_NAME,
            printpdf::Pt(mm_to_pt(available)),
        );
        self.canvas
            .text(left, y, &name, &BUSINESS_NAME, Align::Left);
        y += 5.6;

        for line in &self.document.business.address_lines {
            let line = self.fonts.truncate_to_width(
                line,
                FontRole::Regular,
                SIZE_SMALL,
                printpdf::Pt(mm_to_pt(available)),
            );
            self.canvas.text(left, y, &line, &SMALL_MUTED, Align::Left);
            y += LINE_SMALL;
        }
        for (label, value) in &self.document.business.details {
            let text = self.fonts.truncate_to_width(
                &format!("{label}: {value}"),
                FontRole::Regular,
                SIZE_SMALL,
                printpdf::Pt(mm_to_pt(available)),
            );
            self.canvas.text(left, y, &text, &SMALL_MUTED, Align::Left);
            y += LINE_SMALL;
        }
        y
    }

    fn draw_title_and_meta(&mut self) -> f32 {
        let mut y = self.y + 5.5;
        self.canvas
            .text(CONTENT_RIGHT, y, &self.document.title, &TITLE, Align::Right);
        y += 7.0;

        for (label, value) in &self.document.meta {
            self.canvas.text(
                CONTENT_RIGHT,
                y,
                &format!("{label}  "),
                &SMALL_MUTED,
                Align::Right,
            );
            y += LINE_SMALL;
            self.canvas
                .text(CONTENT_RIGHT, y, value, &META_VALUE, Align::Right);
            y += LINE_BODY + 1.2;
        }
        y
    }

    // ── Bill-to block ────────────────────────────────────────────────────

    fn draw_parties(&mut self) {
        let Some(customer) = &self.document.customer else {
            return;
        };
        let block_height = 8.0 + party_height(customer);
        self.ensure_space(block_height);

        self.canvas
            .text(CONTENT_LEFT, self.y, "BILL TO", &SECTION_LABEL, Align::Left);
        self.y += 5.4;
        self.canvas.text(
            CONTENT_LEFT,
            self.y,
            &customer.name,
            &PARTY_NAME,
            Align::Left,
        );
        self.y += 5.0;
        for line in &customer.address_lines {
            self.canvas
                .text(CONTENT_LEFT, self.y, line, &SMALL_MUTED, Align::Left);
            self.y += LINE_SMALL;
        }
        for (label, value) in &customer.details {
            self.canvas.text(
                CONTENT_LEFT,
                self.y,
                &format!("{label}: {value}"),
                &SMALL_MUTED,
                Align::Left,
            );
            self.y += LINE_SMALL;
        }
        self.y += 5.0;
    }

    // ── Line items ───────────────────────────────────────────────────────

    fn draw_line_items(&mut self) {
        self.ensure_space(24.0);
        self.draw_table_header();

        for item in &self.document.line_items {
            let description_lines = self.fonts.wrap(
                &item.description,
                FontRole::Regular,
                SIZE_BODY,
                printpdf::Pt(mm_to_pt(DESCRIPTION_WIDTH)),
            );
            let mut extra_lines = description_lines.len().saturating_sub(1) as f32;
            if item.discount_note.is_some() {
                extra_lines += 1.0;
            }
            let row_height = 7.4 + extra_lines * LINE_BODY;

            if self.y + row_height > BODY_BOTTOM {
                self.new_page();
                self.draw_table_header();
            }

            let baseline = self.y + 5.0;
            self.canvas.text(
                COL_POSITION,
                baseline,
                &item.position.to_string(),
                &SMALL_MUTED,
                Align::Left,
            );

            let mut description_baseline = baseline;
            for line in &description_lines {
                self.canvas.text(
                    COL_DESCRIPTION,
                    description_baseline,
                    line,
                    &BODY,
                    Align::Left,
                );
                description_baseline += LINE_BODY;
            }
            if let Some(note) = &item.discount_note {
                self.canvas.text(
                    COL_DESCRIPTION,
                    description_baseline,
                    note,
                    &SMALL_MUTED,
                    Align::Left,
                );
            }

            let quantity = match item.unit.trim() {
                "" => item.quantity.clone(),
                unit => format!("{} {}", item.quantity, unit),
            };
            self.canvas
                .text(COL_QUANTITY_RIGHT, baseline, &quantity, &BODY, Align::Right);
            self.canvas.text(
                COL_RATE_RIGHT,
                baseline,
                &self.money(&item.rate),
                &BODY,
                Align::Right,
            );
            self.canvas.text(
                COL_TAX_RIGHT,
                baseline,
                &item.tax_rate,
                &BODY_MUTED,
                Align::Right,
            );
            self.canvas.text(
                COL_AMOUNT_RIGHT,
                baseline,
                &self.money(&item.amount),
                &BODY,
                Align::Right,
            );

            self.y += row_height;
            self.canvas
                .hline(CONTENT_LEFT, CONTENT_RIGHT, self.y, Pt(0.4), RULE_LIGHT);
        }
    }

    fn draw_table_header(&mut self) {
        const HEADER_HEIGHT: f32 = 7.6;
        self.canvas
            .fill_rect(CONTENT_LEFT, self.y, CONTENT_WIDTH, HEADER_HEIGHT, BAND);
        let baseline = self.y + 5.2;
        self.canvas
            .text(COL_POSITION, baseline, "#", &SECTION_LABEL, Align::Left);
        self.canvas.text(
            COL_DESCRIPTION,
            baseline,
            "DESCRIPTION",
            &SECTION_LABEL,
            Align::Left,
        );
        self.canvas.text(
            COL_QUANTITY_RIGHT,
            baseline,
            "QTY",
            &SECTION_LABEL,
            Align::Right,
        );
        self.canvas.text(
            COL_RATE_RIGHT,
            baseline,
            "RATE",
            &SECTION_LABEL,
            Align::Right,
        );
        self.canvas
            .text(COL_TAX_RIGHT, baseline, "TAX", &SECTION_LABEL, Align::Right);
        self.canvas.text(
            COL_AMOUNT_RIGHT,
            baseline,
            "AMOUNT",
            &SECTION_LABEL,
            Align::Right,
        );
        self.y += HEADER_HEIGHT;
        self.canvas
            .hline(CONTENT_LEFT, CONTENT_RIGHT, self.y, Pt(0.5), RULE);
    }

    // ── Totals ───────────────────────────────────────────────────────────

    fn draw_totals(&mut self) {
        let strong_rows = self
            .document
            .totals
            .iter()
            .filter(|row| row.weight == TotalWeight::Strong)
            .count() as f32;
        let block_height = 4.0
            + self.document.totals.len() as f32 * TOTALS_ROW_HEIGHT
            + strong_rows * 2.0
            + if self.document.overpayment_note.is_some() {
                6.0
            } else {
                0.0
            };
        // The totals must never be orphaned from the table onto a page of
        // their own accidentally — but if they genuinely do not fit, a clean
        // break is better than overprinting the footer.
        self.ensure_space(block_height);
        self.y += 4.0;

        for row in &self.document.totals {
            // A strong row (the total, the balance due) is set larger, in
            // the accent colour, over a rule; a component row is quiet, with
            // only its amount in full-strength ink.
            let (label_style, amount_style) = match row.weight {
                TotalWeight::Normal => (&BODY_MUTED, &BODY),
                TotalWeight::Strong => (&TOTAL_STRONG, &TOTAL_STRONG),
            };
            if row.weight == TotalWeight::Strong {
                self.canvas
                    .hline(TOTALS_LEFT, CONTENT_RIGHT, self.y, Pt(0.6), RULE);
                self.y += 2.0;
            }
            let baseline = self.y + 4.4;
            self.canvas
                .text(TOTALS_LEFT, baseline, &row.label, label_style, Align::Left);
            self.canvas.text(
                COL_AMOUNT_RIGHT,
                baseline,
                &self.money(&row.amount),
                amount_style,
                Align::Right,
            );
            self.y += TOTALS_ROW_HEIGHT;
        }

        if let Some(note) = &self.document.overpayment_note {
            self.canvas.text(
                COL_AMOUNT_RIGHT,
                self.y + 4.0,
                &self.money_note(note),
                &SMALL_MUTED,
                Align::Right,
            );
            self.y += 6.0;
        }
        self.y += 6.0;
    }

    /// The overpayment note arrives as `Overpaid by 1,234.00` — the amount is
    /// the tail, so the currency prefix goes in front of it, not the sentence.
    fn money_note(&self, note: &str) -> String {
        match note.rsplit_once(' ') {
            Some((head, amount)) => format!("{head} {}", self.money(amount)),
            None => note.to_string(),
        }
    }

    // ── Notes, terms, payment details ────────────────────────────────────

    fn draw_footer_blocks(&mut self) {
        let left_column_width = 96.0;
        let right_column_left = CONTENT_LEFT + 104.0;
        let right_column_width = CONTENT_RIGHT - right_column_left;

        let left_blocks: Vec<(&str, &String)> = [
            self.document.notes.as_ref().map(|n| ("NOTES", n)),
            self.document
                .terms
                .as_ref()
                .map(|t| ("TERMS & CONDITIONS", t)),
        ]
        .into_iter()
        .flatten()
        .collect();

        if left_blocks.is_empty() && self.document.payment_details.is_empty() {
            return;
        }

        let left_height = self.blocks_height(&left_blocks, left_column_width);
        let payment_blocks: Vec<(&str, &String)> = self
            .document
            .payment_details
            .iter()
            .map(|(label, value)| (label.as_str(), value))
            .collect();
        let right_height = self.blocks_height(&payment_blocks, right_column_width);

        self.ensure_space(left_height.max(right_height) + 6.0);
        self.canvas
            .hline(CONTENT_LEFT, CONTENT_RIGHT, self.y, Pt(0.4), RULE_LIGHT);
        self.y += 6.0;

        let top = self.y;
        let left_end = self.draw_blocks(&left_blocks, CONTENT_LEFT, left_column_width, top);
        let right_end =
            self.draw_blocks(&payment_blocks, right_column_left, right_column_width, top);
        self.y = left_end.max(right_end);
    }

    fn blocks_height(&self, blocks: &[(&str, &String)], width: f32) -> f32 {
        blocks
            .iter()
            .map(|(_, body)| {
                let lines = self.fonts.wrap(
                    body,
                    FontRole::Regular,
                    SIZE_SMALL,
                    printpdf::Pt(mm_to_pt(width)),
                );
                5.0 + lines.len() as f32 * LINE_SMALL + 4.0
            })
            .sum()
    }

    fn draw_blocks(&mut self, blocks: &[(&str, &String)], left: f32, width: f32, top: f32) -> f32 {
        let mut y = top;
        for (label, body) in blocks {
            self.canvas
                .text(left, y, label, &SECTION_LABEL, Align::Left);
            y += 5.0;
            for line in self.fonts.wrap(
                body,
                FontRole::Regular,
                SIZE_SMALL,
                printpdf::Pt(mm_to_pt(width)),
            ) {
                self.canvas.text(left, y, &line, &SMALL, Align::Left);
                y += LINE_SMALL;
            }
            y += 4.0;
        }
        y
    }
}

fn party_height(party: &PdfParty) -> f32 {
    5.4 + 5.0 + (party.address_lines.len() + party.details.len()) as f32 * LINE_SMALL
}

fn mm_to_pt(mm: f32) -> f32 {
    printpdf::Mm(mm).into_pt().0
}

#[cfg(test)]
mod tests {
    use printpdf::PdfParseOptions;

    use super::*;
    use crate::domain::invoice_pdf::{CurrencyDisplay, PdfLineItem, PdfTotalRow};

    fn party(name: &str) -> PdfParty {
        PdfParty {
            name: name.to_string(),
            address_lines: vec!["1 Mill Road".to_string(), "Bengaluru 560001".to_string()],
            details: vec![("GSTIN".to_string(), "29AAAAA0000A1Z5".to_string())],
        }
    }

    fn line_item(position: usize, description: &str) -> PdfLineItem {
        PdfLineItem {
            position,
            description: description.to_string(),
            quantity: "2".to_string(),
            unit: "pcs".to_string(),
            rate: "1,000.00".to_string(),
            tax_rate: "18%".to_string(),
            amount: "2,360.00".to_string(),
            discount_note: None,
        }
    }

    fn document(currency_code: &str, symbol: Option<&str>) -> InvoicePdfDocument {
        InvoicePdfDocument {
            title: "TAX INVOICE".to_string(),
            watermark: None,
            currency: CurrencyDisplay {
                symbol: symbol.map(str::to_string),
                code: currency_code.to_string(),
            },
            business: party("Acme Traders"),
            logo_path: None,
            customer: Some(party("Beta Buyers")),
            meta: vec![
                ("Invoice No.".to_string(), "INV-2026-0007".to_string()),
                ("Date".to_string(), "09/03/2026".to_string()),
            ],
            line_items: vec![line_item(1, "Handmade Widget")],
            totals: vec![
                PdfTotalRow {
                    label: "Subtotal".to_string(),
                    amount: "2,000.00".to_string(),
                    weight: TotalWeight::Normal,
                },
                PdfTotalRow {
                    label: "Total".to_string(),
                    amount: "2,360.00".to_string(),
                    weight: TotalWeight::Strong,
                },
            ],
            notes: Some("Thanks for your business.".to_string()),
            terms: Some("Payment within 15 days.".to_string()),
            payment_details: vec![("UPI ID".to_string(), "acme@upi".to_string())],
            overpayment_note: None,
        }
    }

    fn render(document: &InvoicePdfDocument) -> Vec<u8> {
        PrintpdfInvoiceRenderer::new()
            .render(document)
            .expect("rendering must succeed")
    }

    /// Text per page, as it actually landed in the content stream — this is
    /// what proves the glyphs and the CMap survived font subsetting, not just
    /// that some bytes came out.
    fn text_by_page(bytes: &[u8]) -> Vec<String> {
        let parsed = PdfDocument::parse(bytes, &PdfParseOptions::default(), &mut Vec::new())
            .expect("printpdf must be able to read back its own output");
        parsed
            .extract_text()
            .into_iter()
            .map(|page| page.join(" "))
            .collect()
    }

    #[test]
    fn renders_a_single_page_pdf_containing_the_document_text() {
        let bytes = render(&document("INR", Some("₹")));
        assert!(bytes.starts_with(b"%PDF-"), "output must be a PDF");

        let pages = text_by_page(&bytes);
        assert_eq!(pages.len(), 1);
        let page = &pages[0];
        for expected in [
            "TAX INVOICE",
            "Acme Traders",
            "Beta Buyers",
            "INV-2026-0007",
            "Handmade Widget",
            "Subtotal",
            "Total",
            "Thanks for your business.",
            "acme@upi",
            "Page 1 of 1",
        ] {
            assert!(page.contains(expected), "page text is missing {expected:?}");
        }
    }

    #[test]
    fn money_is_prefixed_with_the_symbol_when_the_embedded_font_can_draw_it() {
        let page = text_by_page(&render(&document("INR", Some("₹"))))[0].clone();
        assert!(page.contains("₹2,360.00"), "got: {page}");
    }

    #[test]
    fn money_falls_back_to_the_iso_code_when_the_symbol_has_no_glyph() {
        // BDT's `৳` is one of the two symbols DejaVu Sans does not carry —
        // printing the amount with the symbol silently dropped would be worse
        // than printing the code.
        let page = text_by_page(&render(&document("BDT", Some("৳"))))[0].clone();
        assert!(page.contains("BDT 2,360.00"), "got: {page}");
        assert!(!page.contains("৳"));
    }

    #[test]
    fn an_unknown_currency_with_no_symbol_still_prints_its_code() {
        let page = text_by_page(&render(&document("XYZ", None)))[0].clone();
        assert!(page.contains("XYZ 2,360.00"), "got: {page}");
    }

    #[test]
    fn a_negative_amount_keeps_its_sign_in_front_of_the_symbol() {
        let mut doc = document("INR", Some("₹"));
        doc.totals.insert(
            1,
            PdfTotalRow {
                label: "Discount".to_string(),
                amount: "-200.00".to_string(),
                weight: TotalWeight::Normal,
            },
        );
        let page = text_by_page(&render(&doc))[0].clone();
        assert!(page.contains("-₹200.00"), "got: {page}");
    }

    #[test]
    fn long_invoices_paginate_and_repeat_the_column_headers() {
        let mut doc = document("INR", Some("₹"));
        doc.line_items = (1..=60)
            .map(|i| line_item(i, &format!("Item {i}")))
            .collect();

        let pages = text_by_page(&render(&doc));
        assert!(pages.len() > 1, "60 line items must not fit on one page");
        for (index, page) in pages.iter().enumerate() {
            assert!(
                page.contains(&format!("Page {} of {}", index + 1, pages.len())),
                "page {} is missing its counter",
                index + 1
            );
        }
        // Every page that carries rows carries the header band above them.
        assert!(pages[0].contains("DESCRIPTION"));
        assert!(pages[1].contains("DESCRIPTION"));
        // The first and last items land on the first and last pages.
        assert!(pages[0].contains("Item 1"));
        assert!(pages.last().unwrap().contains("Item 60"));
    }

    #[test]
    fn the_draft_watermark_is_stamped_on_every_page() {
        let mut doc = document("INR", Some("₹"));
        doc.watermark = Some("DRAFT".to_string());
        doc.line_items = (1..=60)
            .map(|i| line_item(i, &format!("Item {i}")))
            .collect();

        let pages = text_by_page(&render(&doc));
        assert!(pages.len() > 1);
        for page in &pages {
            assert!(page.contains("DRAFT"), "every sheet of a draft is stamped");
        }
    }

    #[test]
    fn a_very_long_description_wraps_instead_of_overflowing_its_column() {
        let mut doc = document("INR", Some("₹"));
        let long =
            "Bespoke reclaimed teak dining table with hand-turned legs and a natural oil finish";
        doc.line_items = vec![line_item(1, long)];

        let bytes = render(&doc);
        let page = text_by_page(&bytes)[0].clone();
        // The words all survive; they simply arrive across several lines.
        for word in ["Bespoke", "hand-turned", "finish"] {
            assert!(page.contains(word), "wrapped text lost {word:?}");
        }
        assert_eq!(text_by_page(&bytes).len(), 1);
    }

    #[test]
    fn an_invoice_with_no_customer_or_line_items_still_renders() {
        // A brand-new draft previewed before anything has been filled in.
        let mut doc = document("INR", Some("₹"));
        doc.customer = None;
        doc.line_items = Vec::new();
        doc.notes = None;
        doc.terms = None;
        doc.payment_details = Vec::new();

        let pages = text_by_page(&render(&doc));
        assert_eq!(pages.len(), 1);
        assert!(pages[0].contains("Acme Traders"));
    }

    #[test]
    fn probing_reports_a_usable_logo_with_its_pixel_size() {
        // A 1x1 PNG, so the probe has something real to decode.
        let png: &[u8] = &[
            0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48,
            0x44, 0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x02, 0x00, 0x00,
            0x00, 0x90, 0x77, 0x53, 0xDE, 0x00, 0x00, 0x00, 0x0C, 0x49, 0x44, 0x41, 0x54, 0x08,
            0xD7, 0x63, 0xF8, 0xCF, 0xC0, 0x00, 0x00, 0x03, 0x01, 0x01, 0x00, 0x18, 0xDD, 0x8D,
            0xB0, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
        ];
        let path = std::env::temp_dir().join("vunexo-probe-ok.png");
        std::fs::write(&path, png).unwrap();
        assert_eq!(
            PrintpdfInvoiceRenderer::new().probe_logo(&path),
            LogoProbe::Ok {
                width_px: 1,
                height_px: 1
            }
        );
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn probing_distinguishes_a_moved_file_from_an_undecodable_one() {
        let renderer = PrintpdfInvoiceRenderer::new();
        assert_eq!(
            renderer.probe_logo(Path::new("/definitely/not/a/real/logo.png")),
            LogoProbe::NotFound
        );

        let path = std::env::temp_dir().join("vunexo-probe-bad.png");
        std::fs::write(&path, b"this is not an image").unwrap();
        assert_eq!(renderer.probe_logo(&path), LogoProbe::Unreadable);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn a_missing_logo_file_does_not_fail_the_render() {
        let mut doc = document("INR", Some("₹"));
        doc.logo_path = Some("/definitely/not/a/real/logo.png".to_string());
        let pages = text_by_page(&render(&doc));
        assert!(pages[0].contains("Acme Traders"));
    }

    /// Not an assertion — a way to *look* at the template while working on
    /// it. `cargo test sample_for_visual_inspection -- --ignored` writes one
    /// filled-in invoice (wrapped description, line discount, discount, GST
    /// split, part payment) to `$VUNEXO_PDF_SAMPLE`, or to the temp dir.
    #[test]
    #[ignore = "writes a sample PDF for eyeballing the template; not an assertion"]
    fn sample_for_visual_inspection() {
        let mut doc = document("INR", Some("₹"));
        doc.line_items = vec![
            line_item(
                1,
                "Bespoke reclaimed teak dining table with hand-turned legs",
            ),
            line_item(2, "Upholstered dining chair"),
            PdfLineItem {
                discount_note: Some("Less 12.5%".to_string()),
                ..line_item(3, "Sideboard, three drawers")
            },
        ];
        doc.totals = vec![
            PdfTotalRow {
                label: "Subtotal".into(),
                amount: "6,000.00".into(),
                weight: TotalWeight::Normal,
            },
            PdfTotalRow {
                label: "Discount (10%)".into(),
                amount: "-600.00".into(),
                weight: TotalWeight::Normal,
            },
            PdfTotalRow {
                label: "CGST".into(),
                amount: "486.00".into(),
                weight: TotalWeight::Normal,
            },
            PdfTotalRow {
                label: "SGST".into(),
                amount: "486.00".into(),
                weight: TotalWeight::Normal,
            },
            PdfTotalRow {
                label: "Total".into(),
                amount: "6,372.00".into(),
                weight: TotalWeight::Strong,
            },
            PdfTotalRow {
                label: "Amount Paid".into(),
                amount: "-2,000.00".into(),
                weight: TotalWeight::Normal,
            },
            PdfTotalRow {
                label: "Balance Due".into(),
                amount: "4,372.00".into(),
                weight: TotalWeight::Strong,
            },
        ];
        doc.meta
            .push(("Due Date".to_string(), "24/03/2026".to_string()));
        let out = std::env::var("VUNEXO_PDF_SAMPLE")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|_| std::env::temp_dir().join("vunexo-invoice-sample.pdf"));
        std::fs::write(&out, render(&doc)).unwrap();
        println!("wrote {}", out.display());
    }
}
