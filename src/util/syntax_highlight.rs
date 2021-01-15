use lazy_static::lazy_static;
use syntect::easy::HighlightLines;
use syntect::highlighting::{Style, ThemeSet};
use syntect::parsing::SyntaxSet;
use syntect::util::LinesWithEndings;
use tui::style::Color;
use tui::text::{Span, Spans, Text};

lazy_static! {
    static ref SYNTAX: Syntax = Syntax {
        syntax: SyntaxSet::load_defaults_newlines(),
        themes: ThemeSet::load_defaults()
    };
}

struct Syntax {
    pub syntax: SyntaxSet,
    pub themes: ThemeSet,
}

pub fn highlight(lines: &str, extension: &str) -> Option<Vec<Vec<(Style, String)>>> {
    let syntax = match SYNTAX.syntax.find_syntax_by_extension(extension) {
        None => return None,
        Some(syntax) => syntax,
    };

    let mut h = HighlightLines::new(syntax, &SYNTAX.themes.themes["Solarized (dark)"]);

    let spans: Vec<_> = LinesWithEndings::from(lines)
        .map(|line| {
            h.highlight(&line, &SYNTAX.syntax)
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
