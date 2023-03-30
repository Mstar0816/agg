mod fontdue;
mod resvg;

use imgref::ImgVec;
use rgb::{RGB8, RGBA8};

use crate::theme::Theme;

pub trait Renderer {
    fn render(
        &mut self,
        lines: Vec<Vec<(char, avt::Pen)>>,
        cursor: Option<(usize, usize)>,
    ) -> ImgVec<RGBA8>;
    fn pixel_size(&self) -> (usize, usize);
}

pub struct Settings {
    pub terminal_size: (usize, usize),
    pub font_db: fontdb::Database,
    pub font_families: Vec<String>,
    pub font_size: usize,
    pub line_height: f64,
    pub theme: Theme,
}

pub fn resvg(settings: Settings) -> resvg::ResvgRenderer {
    resvg::ResvgRenderer::new(settings)
}

pub fn fontdue(settings: Settings) -> fontdue::FontdueRenderer {
    fontdue::FontdueRenderer::new(settings)
}

struct TextAttrs {
    foreground: Option<avt::Color>,
    background: Option<avt::Color>,
    bold: bool,
    italic: bool,
    underline: bool,
}

fn text_attrs(
    pen: &mut avt::Pen,
    cursor: &Option<(usize, usize)>,
    x: usize,
    y: usize,
    theme: &Theme,
) -> TextAttrs {
    let mut foreground = pen.foreground();
    let mut background = pen.background();
    let inverse = cursor.map_or(false, |(cx, cy)| cx == x && cy == y);

    if pen.is_bold() {
        if let Some(avt::Color::Indexed(n)) = foreground {
            if n < 8 {
                foreground = Some(avt::Color::Indexed(n + 8));
            }
        }
    }

    if pen.is_blink() {
        if let Some(avt::Color::Indexed(n)) = background {
            if n < 8 {
                background = Some(avt::Color::Indexed(n + 8));
            }
        }
    }

    if pen.is_inverse() ^ inverse {
        let fg = background.unwrap_or(avt::Color::RGB(theme.background));
        let bg = foreground.unwrap_or(avt::Color::RGB(theme.foreground));
        foreground = Some(fg);
        background = Some(bg);
    }

    TextAttrs {
        foreground,
        background,
        bold: pen.is_bold(),
        italic: pen.is_italic(),
        underline: pen.is_underline(),
    }
}

fn color_to_rgb(c: &avt::Color, theme: &Theme) -> RGB8 {
    match c {
        avt::Color::RGB(c) => *c,
        avt::Color::Indexed(c) => theme.color(*c),
    }
}
