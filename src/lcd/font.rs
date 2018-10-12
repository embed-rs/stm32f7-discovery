use alloc::Vec;

pub struct FontRenderer<'a> {
    font: Font<'a>,
    height: f32,
}

impl<'a> FontRenderer<'a> {
    pub fn new(font_data: &[u8], font_height: f32) -> FontRenderer {
        let collection = FontCollection::from_bytes(font_data);
        // only succeeds if collection consists of one font
        let font = collection.into_font().unwrap();
        FontRenderer {
            font,
            height: font_height,
        }
    }

    pub fn font_height(&self) -> f32 {
        self.height
    }

    pub fn layout(&self, s: &str) -> Vec<PositionedGlyph> {
        let scale = Scale {
            x: self.height,
            y: self.height,
        };

        // The origin of a line of text is at the baseline (roughly where non-descending letters
        // sit). We don't want to clip the text, so we shift it down with an offset when laying
        // it out. v_metrics.ascent is the distance between the baseline and the highest edge of
        // any glyph in the font. That's enough to guarantee that there's no clipping.
        let v_metrics = self.font.v_metrics(scale);
        let offset = point(0.0, v_metrics.ascent);

        self.font.layout(s, scale, offset).collect()
    }

    pub fn render<F>(&self, s: &str, mut draw_pixel: F) -> usize
    where
        F: FnMut(usize, usize, f32),
    {
        let glyphs = self.layout(s);
        let pixel_height = self.height.ceil() as usize;

        // Find the most visually pleasing width to display
        let width = glyphs
            .iter()
            .rev()
            .map(|g| g.position().x as f32 + g.unpositioned().h_metrics().advance_width)
            .next()
            .unwrap_or(0.0)
            .ceil() as usize;

        for g in glyphs {
            if let Some(bb) = g.pixel_bounding_box() {
                g.draw(|x, y, mut v| {
                    assert!(v >= 0.0 - 1.0e-5);
                    assert!(v <= 1.0 + 1.0e-5);
                    if v < 0.0 {
                        v = 0.0;
                    }
                    if v > 1.0 {
                        v = 1.0;
                    }
                    let x = x as i32 + bb.min.x;
                    let y = y as i32 + bb.min.y;
                    // There's still a possibility that the glyph clips the boundaries of the bitmap
                    if x >= 0 && x < width as i32 && y >= 0 && y < pixel_height as i32 {
                        draw_pixel(x as usize, y as usize, v);
                    }
                });
            }
        }
        return width;
    }
}
