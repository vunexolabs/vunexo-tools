//! The two embedded faces the invoice template is drawn with, plus the glyph
//! metrics the layout needs in order to align, wrap, and truncate text.
//!
//! **Why an embedded font at all:** the PDF base-14 fonts (Helvetica &c.)
//! carry no `₹`, `₦`, `₫`, `₺` … and Vunexo Billing prints in 55 currencies
//! (`domain::currency`). A PDF that renders the amount but silently drops the
//! symbol is worse than one that prints `INR 1,234.00`, so the template ships
//! a face wide enough to cover almost all of them and degrades explicitly for
//! the rest (`can_render`).
//!
//! DejaVu Sans 2.37 — Bitstream Vera license (permissive, redistributable;
//! full text in `assets/fonts/LICENSE.txt`). It covers every currency symbol
//! in `domain::currency` except Bengali `৳` (BDT) and `﷼` (SAR), which fall
//! back to the ISO code. Only the glyphs actually used are written into the
//! output, because `PdfSaveOptions::subset_fonts` defaults to on — the ~1.4 MB
//! of source faces here cost a few KB per invoice.

use printpdf::{FontId, ParsedFont, PdfDocument, PdfFontHandle, Pt};

static REGULAR_TTF: &[u8] = include_bytes!("../../../assets/fonts/DejaVuSans.ttf");
static BOLD_TTF: &[u8] = include_bytes!("../../../assets/fonts/DejaVuSans-Bold.ttf");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontRole {
    Regular,
    Bold,
}

pub struct Fonts {
    regular: ParsedFont,
    bold: ParsedFont,
    regular_id: FontId,
    bold_id: FontId,
}

impl Fonts {
    /// Parses and registers both faces against `doc`. Returns `None` only if
    /// the embedded bytes fail to parse, which would mean the binary itself
    /// is corrupt — the caller turns that into an `InfrastructureError`
    /// rather than panicking mid-render.
    pub fn load(doc: &mut PdfDocument) -> Option<Self> {
        let regular = ParsedFont::from_bytes(REGULAR_TTF, 0, &mut Vec::new())?;
        let bold = ParsedFont::from_bytes(BOLD_TTF, 0, &mut Vec::new())?;
        let regular_id = doc.add_font(&regular);
        let bold_id = doc.add_font(&bold);
        Some(Self {
            regular,
            bold,
            regular_id,
            bold_id,
        })
    }

    pub fn handle(&self, role: FontRole) -> PdfFontHandle {
        match role {
            FontRole::Regular => PdfFontHandle::External(self.regular_id.clone()),
            FontRole::Bold => PdfFontHandle::External(self.bold_id.clone()),
        }
    }

    fn face(&self, role: FontRole) -> &ParsedFont {
        match role {
            FontRole::Regular => &self.regular,
            FontRole::Bold => &self.bold,
        }
    }

    /// Advance width of `text` at `size`, summed from the face's own `hmtx`
    /// metrics. This is what makes right-alignment and column fitting exact
    /// rather than guessed — the layout never assumes a monospace grid.
    ///
    /// A character the face has no glyph for is measured as half an em, the
    /// same rough width the `.notdef` box occupies, so a string containing
    /// one still lands in roughly the right place.
    pub fn text_width(&self, text: &str, role: FontRole, size: Pt) -> Pt {
        let face = self.face(role);
        let units_per_em = f32::from(face.font_metrics.units_per_em.max(1));
        let mut units = 0.0f32;
        for ch in text.chars() {
            units += match face
                .lookup_glyph_index(ch as u32)
                .and_then(|gid| face.get_glyph_width_internal(gid))
            {
                Some(width) => width as f32,
                None => units_per_em / 2.0,
            };
        }
        Pt(units / units_per_em * size.0)
    }

    /// Whether every character of `text` has a real glyph in this face —
    /// used to decide between printing a currency's symbol and its ISO code.
    pub fn can_render(&self, text: &str, role: FontRole) -> bool {
        let face = self.face(role);
        text.chars()
            .all(|ch| ch == ' ' || face.lookup_glyph_index(ch as u32).is_some())
    }

    /// Greedy word wrap to `max_width`. Words longer than the line (a URL, a
    /// long SKU) are split mid-word rather than allowed to overflow the
    /// column, and explicit newlines in the source text are honoured.
    pub fn wrap(&self, text: &str, role: FontRole, size: Pt, max_width: Pt) -> Vec<String> {
        let mut lines = Vec::new();
        for paragraph in text.split('\n') {
            let paragraph = paragraph.trim_end();
            if paragraph.is_empty() {
                lines.push(String::new());
                continue;
            }
            let mut current = String::new();
            for word in paragraph.split_whitespace() {
                let candidate = if current.is_empty() {
                    word.to_string()
                } else {
                    format!("{current} {word}")
                };
                if self.text_width(&candidate, role, size).0 <= max_width.0 {
                    current = candidate;
                    continue;
                }
                if !current.is_empty() {
                    lines.push(std::mem::take(&mut current));
                }
                // The word alone may still not fit; break it by characters.
                for chunk in self.break_word(word, role, size, max_width) {
                    if self.text_width(&chunk, role, size).0 > max_width.0 {
                        lines.push(chunk);
                    } else {
                        current = chunk;
                    }
                }
            }
            if !current.is_empty() {
                lines.push(current);
            }
        }
        if lines.is_empty() {
            lines.push(String::new());
        }
        lines
    }

    fn break_word(&self, word: &str, role: FontRole, size: Pt, max_width: Pt) -> Vec<String> {
        let mut chunks = Vec::new();
        let mut current = String::new();
        for ch in word.chars() {
            let mut candidate = current.clone();
            candidate.push(ch);
            if !current.is_empty() && self.text_width(&candidate, role, size).0 > max_width.0 {
                chunks.push(std::mem::take(&mut current));
                current.push(ch);
            } else {
                current = candidate;
            }
        }
        if !current.is_empty() {
            chunks.push(current);
        }
        chunks
    }

    /// Shortens `text` with an ellipsis until it fits `max_width`. Used where
    /// wrapping isn't an option (a single-line cell) and an overflowing
    /// string would collide with the next column.
    pub fn truncate_to_width(&self, text: &str, role: FontRole, size: Pt, max_width: Pt) -> String {
        if self.text_width(text, role, size).0 <= max_width.0 {
            return text.to_string();
        }
        let mut out = String::new();
        for ch in text.chars() {
            let mut candidate = out.clone();
            candidate.push(ch);
            if self.text_width(&format!("{candidate}…"), role, size).0 > max_width.0 {
                break;
            }
            out = candidate;
        }
        out.push('…');
        out
    }
}
