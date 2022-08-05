use imgref::ImgVec;
use rgb::{FromSlice, RGBA8};

use crate::theme::Theme;

use super::{adjust_pen, color_to_rgb, Renderer};

pub struct ResvgRenderer {
    cols: usize,
    rows: usize,
    theme: Theme,
    pixel_width: usize,
    pixel_height: usize,
    char_width: f32,
    row_height: f32,
    options: usvg::Options,
    transform: tiny_skia::Transform,
    fit_to: usvg::FitTo,
    header: String,
}

fn color_to_style(color: &vt::Color, theme: &Theme) -> String {
    let c = color_to_rgb(color, theme);

    format!("fill: rgb({},{},{})", c.r, c.g, c.b)
}

fn text_class(pen: &vt::Pen) -> String {
    let mut class = "".to_owned();

    if pen.bold {
        class.push_str("br");
    }

    if pen.italic {
        class.push_str(" it");
    }

    if pen.underline {
        class.push_str(" un");
    }

    class
}

fn text_style(pen: &vt::Pen, theme: &Theme) -> String {
    pen.foreground
        .map(|c| color_to_style(&c, theme))
        .unwrap_or_else(|| "".to_owned())
}

fn rect_style(pen: &vt::Pen, theme: &Theme) -> String {
    pen.background
        .map(|c| color_to_style(&c, theme))
        .unwrap_or_else(|| "".to_owned())
}

impl ResvgRenderer {
    pub fn new(
        cols: usize,
        rows: usize,
        font_db: fontdb::Database,
        font_family: &str,
        theme: Theme,
        zoom: f32,
    ) -> Self {
        let char_width = 100.0 * 1.0 / (cols as f32 + 2.0);
        let font_size = 14.0;
        let row_height = font_size * 1.4;
        let options = usvg::Options {
            fontdb: font_db,
            ..Default::default()
        };
        let fit_to = usvg::FitTo::Zoom(zoom);
        let transform = tiny_skia::Transform::default();
        let header = Self::header(cols, rows, font_family, font_size, row_height, &theme);
        let mut svg = header.clone();
        svg.push_str(Self::footer());
        let tree = usvg::Tree::from_str(&svg, &options.to_ref()).unwrap();
        let screen_size = tree.svg_node().size.to_screen_size();
        let screen_size = fit_to.fit_to(screen_size).unwrap();
        let pixel_width = screen_size.width() as usize;
        let pixel_height = screen_size.height() as usize;

        Self {
            cols,
            rows,
            theme,
            pixel_width,
            pixel_height,
            char_width,
            row_height,
            options,
            transform,
            fit_to,
            header,
        }
    }

    fn header(
        cols: usize,
        rows: usize,
        font_family: &str,
        font_size: f32,
        row_height: f32,
        theme: &Theme,
    ) -> String {
        let width = (cols + 2) as f32 * 8.433333;
        let height = (rows + 1) as f32 * row_height;
        let x = 1.0 * 100.0 / (cols as f32 + 2.0);
        let y = 0.5 * 100.0 / (rows as f32 + 1.0);

        format!(
            r#"<?xml version="1.0"?>
<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" width="{}" height="{}" font-size="{}px" font-family="{}">
<style>
.br {{ font-weight: bold }}
.it {{ font-style: italic }}
.un {{ text-decoration: underline }}
</style>
<rect width="100%" height="100%" rx="{}" ry="{}" style="fill: {}" />
<svg x="{:.3}%" y="{:.3}%" style="fill: {}">"#,
            width, height, font_size, font_family, 4, 4, theme.background, x, y, theme.foreground
        )
    }

    fn footer() -> &'static str {
        "</svg></svg>"
    }

    fn push_lines(
        &self,
        svg: &mut String,
        lines: Vec<Vec<(char, vt::Pen)>>,
        cursor: Option<(usize, usize)>,
    ) {
        svg.push_str(r#"<g style="shape-rendering: optimizeSpeed">"#);

        for (row, line) in lines.iter().enumerate() {
            let y = 100.0 * (row as f32) / (self.rows as f32 + 1.0);

            for (col, (_ch, mut pen)) in line.iter().enumerate() {
                adjust_pen(&mut pen, &cursor, col, row, &self.theme);

                if pen.background.is_none() {
                    continue;
                }

                let x = 100.0 * (col as f32) / (self.cols as f32 + 2.0);
                let style = rect_style(&pen, &self.theme);

                svg.push_str(&format!(
                    r#"<rect x="{:.3}%" y="{:.3}%" width="{:.3}%" height="{:.3}" style="{}" />"#,
                    x, y, self.char_width, self.row_height, style
                ));
            }
        }

        svg.push_str("</g>");
        svg.push_str(r#"<text class="default-text-fill">"#);

        for (row, line) in lines.iter().enumerate() {
            let y = 100.0 * (row as f32) / (self.rows as f32 + 1.0);
            svg.push_str(&format!(r#"<tspan y="{:.3}%">"#, y));
            let mut did_dy = false;

            for (col, (ch, mut pen)) in line.iter().enumerate() {
                if ch == &' ' {
                    continue;
                }

                adjust_pen(&mut pen, &cursor, col, row, &self.theme);

                svg.push_str("<tspan ");

                if !did_dy {
                    svg.push_str(r#"dy="1em" "#);
                    did_dy = true;
                }

                let x = 100.0 * (col as f32) / (self.cols as f32 + 2.0);
                let class = text_class(&pen);
                let style = text_style(&pen, &self.theme);

                svg.push_str(&format!(
                    r#"x="{:.3}%" class="{}" style="{}">"#,
                    x, class, style
                ));

                match ch {
                    '\'' => {
                        svg.push_str("&#39;");
                    }

                    '"' => {
                        svg.push_str("&quot;");
                    }

                    '&' => {
                        svg.push_str("&amp;");
                    }

                    '>' => {
                        svg.push_str("&gt;");
                    }

                    '<' => {
                        svg.push_str("&lt;");
                    }

                    _ => {
                        svg.push(*ch);
                    }
                }

                svg.push_str("</tspan>");
            }

            svg.push_str("</tspan>");
        }

        svg.push_str("</text>");
    }
}

impl Renderer for ResvgRenderer {
    fn render(
        &mut self,
        lines: Vec<Vec<(char, vt::Pen)>>,
        cursor: Option<(usize, usize)>,
    ) -> ImgVec<RGBA8> {
        let mut svg = self.header.clone();
        self.push_lines(&mut svg, lines, cursor);
        svg.push_str(Self::footer());
        let tree = usvg::Tree::from_str(&svg, &self.options.to_ref()).unwrap();

        let mut pixmap =
            tiny_skia::Pixmap::new(self.pixel_width as u32, self.pixel_height as u32).unwrap();

        resvg::render(&tree, self.fit_to, self.transform, pixmap.as_mut()).unwrap();
        let buf = pixmap.take().as_rgba().to_vec();

        ImgVec::new(buf, self.pixel_width, self.pixel_height)
    }

    fn pixel_width(&self) -> usize {
        self.pixel_width
    }

    fn pixel_height(&self) -> usize {
        self.pixel_height
    }
}
