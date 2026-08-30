//! Drawing primitives for the invoice template.
//!
//! Everything above this file thinks in **millimetres from the top-left of
//! the page**, the way the template is designed on paper; PDF's own
//! coordinate space is points from the bottom-left. That flip is confined to
//! `Canvas::y` so no layout code ever has to remember which way is up.

use printpdf::{
    Color, Line, LinePoint, Mm, Op, PaintMode, Point, Pt, Rect, Rgb, TextItem, TextMatrix,
    WindingOrder, XObjectId, XObjectTransform,
};

use super::fonts::{FontRole, Fonts};

pub const fn rgb(r: u8, g: u8, b: u8) -> Color {
    Color::Rgb(Rgb {
        r: r as f32 / 255.0,
        g: g as f32 / 255.0,
        b: b as f32 / 255.0,
        icc_profile: None,
    })
}

/// How one run of text looks. Bundled rather than passed as three loose
/// arguments so the template can name its type styles once (`SIZE_TOTAL` in
/// accent bold, small muted labels, …) and call sites stay readable.
#[derive(Debug, Clone)]
pub struct TextStyle {
    pub role: FontRole,
    pub size: Pt,
    pub color: Color,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Align {
    Left,
    Right,
    Center,
}

pub struct Canvas<'a> {
    fonts: &'a Fonts,
    page_height_mm: f32,
    ops: Vec<Op>,
}

impl<'a> Canvas<'a> {
    pub fn new(fonts: &'a Fonts, page_height_mm: f32) -> Self {
        Self {
            fonts,
            page_height_mm,
            ops: Vec::new(),
        }
    }

    pub fn into_ops(self) -> Vec<Op> {
        self.ops
    }

    /// mm-from-top -> Pt-from-bottom.
    fn y(&self, mm_from_top: f32) -> Pt {
        Mm(self.page_height_mm - mm_from_top).into_pt()
    }

    /// Draws one line of text with `baseline` measured from the page top.
    /// `x` is the left, right, or centre edge depending on `align`.
    pub fn text(
        &mut self,
        x_mm: f32,
        baseline_mm: f32,
        text: &str,
        style: &TextStyle,
        align: Align,
    ) {
        if text.is_empty() {
            return;
        }
        let TextStyle { role, size, color } = style;
        let (role, size) = (*role, *size);
        let width = self.fonts.text_width(text, role, size);
        let left_pt = match align {
            Align::Left => Mm(x_mm).into_pt(),
            Align::Right => Pt(Mm(x_mm).into_pt().0 - width.0),
            Align::Center => Pt(Mm(x_mm).into_pt().0 - width.0 / 2.0),
        };
        self.ops.extend([
            Op::StartTextSection,
            Op::SetFont {
                font: self.fonts.handle(role),
                size,
            },
            Op::SetFillColor { col: color.clone() },
            Op::SetTextCursor {
                pos: Point {
                    x: left_pt,
                    y: self.y(baseline_mm),
                },
            },
            Op::ShowText {
                items: vec![TextItem::Text(text.to_string())],
            },
            Op::EndTextSection,
        ]);
    }

    /// Filled rectangle, `y_mm` being its **top** edge.
    pub fn fill_rect(&mut self, x_mm: f32, y_mm: f32, width_mm: f32, height_mm: f32, color: Color) {
        self.ops.extend([
            Op::SaveGraphicsState,
            Op::SetFillColor { col: color },
            Op::DrawRectangle {
                rectangle: Rect {
                    x: Mm(x_mm).into_pt(),
                    y: self.y(y_mm + height_mm),
                    width: Mm(width_mm).into_pt(),
                    height: Mm(height_mm).into_pt(),
                    mode: Some(PaintMode::Fill),
                    winding_order: Some(WindingOrder::NonZero),
                },
            },
            Op::RestoreGraphicsState,
        ]);
    }

    /// Horizontal rule from `x1` to `x2` at `y_mm`.
    pub fn hline(&mut self, x1_mm: f32, x2_mm: f32, y_mm: f32, thickness: Pt, color: Color) {
        let y = self.y(y_mm);
        self.ops.extend([
            Op::SaveGraphicsState,
            Op::SetOutlineColor { col: color },
            Op::SetOutlineThickness { pt: thickness },
            Op::DrawLine {
                line: Line {
                    points: vec![
                        LinePoint {
                            p: Point {
                                x: Mm(x1_mm).into_pt(),
                                y,
                            },
                            bezier: false,
                        },
                        LinePoint {
                            p: Point {
                                x: Mm(x2_mm).into_pt(),
                                y,
                            },
                            bezier: false,
                        },
                    ],
                    is_closed: false,
                },
            },
            Op::RestoreGraphicsState,
        ]);
    }

    /// Places an image so that it fits inside `width_mm × height_mm` with its
    /// aspect ratio intact, centred horizontally in that box and anchored to
    /// the box's top. `pixel_width`/`pixel_height` come from the decoded
    /// image; the DPI is derived from them so the image lands at exactly the
    /// requested physical size instead of printpdf's 300-DPI default.
    #[allow(clippy::too_many_arguments)]
    pub fn image(
        &mut self,
        id: &XObjectId,
        x_mm: f32,
        y_mm: f32,
        width_mm: f32,
        height_mm: f32,
        pixel_width: usize,
        pixel_height: usize,
    ) {
        if pixel_width == 0 || pixel_height == 0 {
            return;
        }
        let scale = (width_mm / pixel_width as f32).min(height_mm / pixel_height as f32);
        let drawn_width_mm = pixel_width as f32 * scale;
        let drawn_height_mm = pixel_height as f32 * scale;
        // `dpi` is what turns pixels into physical size: inches = px / dpi.
        let dpi = pixel_width as f32 / (drawn_width_mm / 25.4);
        let left_mm = x_mm + (width_mm - drawn_width_mm) / 2.0;

        self.ops.push(Op::UseXobject {
            id: id.clone(),
            transform: XObjectTransform {
                translate_x: Some(Mm(left_mm).into_pt()),
                translate_y: Some(self.y(y_mm + drawn_height_mm)),
                dpi: Some(dpi),
                ..Default::default()
            },
        });
    }

    /// The diagonal DRAFT / CANCELLED stamp. Drawn first, under everything
    /// else, in a light grey so it never fights the invoice's own text.
    ///
    /// `TextMatrix::TranslateRotate` rotates about the point it is given,
    /// which is also where the text starts — so centring the stamp means
    /// stepping back half its length along the baseline direction, then
    /// nudging down the perpendicular to put the glyphs' middle (rather than
    /// their baseline) on the page centre.
    pub fn watermark(&mut self, text: &str, page_width_mm: f32) {
        const SIZE: Pt = Pt(96.0);
        const ROTATION_DEGREES: f32 = 45.0;

        let width = self.fonts.text_width(text, FontRole::Bold, SIZE);
        let radians = (360.0 - ROTATION_DEGREES).to_radians();
        // The matrix printpdf builds is [cos, -sin, sin, cos, x, y]: text
        // advances along (cos, -sin) and the line's "up" is (sin, cos).
        let (along_x, along_y) = (radians.cos(), -radians.sin());
        let (up_x, up_y) = (radians.sin(), radians.cos());

        let centre_x = Mm(page_width_mm / 2.0).into_pt().0;
        let centre_y = self.y(self.page_height_mm / 2.0).0;
        let half = width.0 / 2.0;
        let baseline_drop = SIZE.0 * 0.35;
        let origin_x = centre_x - along_x * half - up_x * baseline_drop;
        let origin_y = centre_y - along_y * half - up_y * baseline_drop;

        self.ops.extend([
            Op::SaveGraphicsState,
            Op::StartTextSection,
            Op::SetFont {
                font: self.fonts.handle(FontRole::Bold),
                size: SIZE,
            },
            Op::SetFillColor {
                col: rgb(234, 234, 239),
            },
            Op::SetTextMatrix {
                matrix: TextMatrix::TranslateRotate(Pt(origin_x), Pt(origin_y), ROTATION_DEGREES),
            },
            Op::ShowText {
                items: vec![TextItem::Text(text.to_string())],
            },
            Op::EndTextSection,
            Op::RestoreGraphicsState,
        ]);
    }
}
