use lazy_static::lazy_static;
use syntect::easy::HighlightLines;
use syntect::highlighting::Color as SynColor;
use syntect::highlighting::{Style, Theme, ThemeSet};
use syntect::parsing::SyntaxSet;
use tui::style::Color;
use tui::text::{Span, Spans, Text};

lazy_static! {
    static ref SYNTAX: Syntax = Syntax {
        syntax: SyntaxSet::load_defaults_nonewlines(),
        themes: ThemeSet::load_defaults()
    };
    static ref THEME: Theme = create_custom_theme();
}

struct Syntax {
    pub syntax: SyntaxSet,
    pub themes: ThemeSet,
}

fn create_custom_theme() -> Theme {
    let mut theme = SYNTAX.themes.themes["Solarized (dark)"].clone();
    theme.settings.foreground = theme.settings.foreground.map(|color| brighter(color, 0.4)); //Some(syntect::highlighting::Color::WHITE);
    theme
}

fn brighter(color: SynColor, factor: f32) -> SynColor {
    SynColor {
        r: color.r + ((255 - color.r) as f32 * factor) as u8,
        g: color.g + ((255 - color.g) as f32 * factor) as u8,
        b: color.b + ((255 - color.b) as f32 * factor) as u8,
        a: color.a,
    }
}

pub fn highlight(lines: &str, extension: &str) -> Option<Vec<Vec<(Style, String)>>> {
    let syntax = match SYNTAX.syntax.find_syntax_by_extension(extension) {
        None => return None,
        Some(syntax) => syntax,
    };

    let mut h = HighlightLines::new(syntax, &THEME);

    // TODO: Due to a bug in tui-rs (?), it is necessary to trim line ends.
    // Otherwise, artifacts of the previous buffer may occur
    let spans: Vec<_> = lines
        .lines()
        .map(|line| {
            h.highlight_line(line.trim_end(), &SYNTAX.syntax)
                .unwrap()
                .into_iter()
                .map(|(s, l)| (s, l.to_string()))
                .collect::<Vec<_>>()
        })
        .collect();

    Some(spans)
}

pub fn as_styled(lines: &'_ [Vec<(Style, String)>]) -> Text<'_> {
    let spans: Vec<_> = lines
        .iter()
        .map(|line| {
            Spans(
                line.iter()
                    .map(|(style, string)| {
                        Span::styled(
                            string,
                            tui::style::Style::default().fg(Color::Rgb(
                                style.foreground.r,
                                style.foreground.g,
                                style.foreground.b,
                            )),
                        )
                    })
                    .collect(),
            )
        })
        .collect();

    Text::from(spans)
}
