use std::fmt;

use muncher::Muncher;
use tui::style::{Color, Style};
use tui::text::Text;

#[derive(Clone, Debug, Default)]
pub struct CtrlChunk {
    ctrl: Vec<String>,
    text: String,
}

impl CtrlChunk {
    pub fn text(text: String) -> Self {
        Self {
            ctrl: Vec::new(),
            text,
        }
    }

    pub fn parse(munch: &mut Muncher) -> Self {
        munch.reset_peek();
        if munch.seek(1) == Some("\x1B") {
            munch.eat();
        }

        let text_or_ctrl = munch.eat_until(|c| *c == '\x1B').collect::<String>();

        if text_or_ctrl.is_empty() {
            return Self {
                ctrl: Vec::new(),
                text: String::new(),
            };
        }

        munch.reset_peek();

        if munch.seek(4) == Some("\x1B[0m") {
            // eat the reset escape code
            let _ = munch.eat_until(|c| *c == 'm');
            munch.eat();

            let mut ctrl_chars = Vec::new();
            loop {
                let ctrl_text = text_or_ctrl.splitn(2, 'm').collect::<Vec<_>>();

                let mut ctrl = vec![ctrl_text[0].replace('[', "")];
                if ctrl[0].contains(';') {
                    ctrl = ctrl[0].split(';').map(|s| s.to_string()).collect();
                }
                ctrl_chars.extend(ctrl);
                if ctrl_text[1].contains('\x1B') {
                    continue;
                } else {
                    let mut text = ctrl_text[1].to_string();

                    let ws = munch.eat_until(|c| !c.is_whitespace()).collect::<String>();
                    text.push_str(&ws);

                    return Self {
                        ctrl: ctrl_chars,
                        text,
                    };
                }
            }
        } else {
            // un control coded text
            Self {
                ctrl: Vec::new(),
                text: text_or_ctrl,
            }
        }
    }
    pub fn into_text<'a>(self) -> Text<'a> {
        let mut style = Style::default();
        if self.ctrl.len() > 2 {
            match &self.ctrl[2] {
                ctrl if ctrl == "0" => {
                    style = style.fg(Color::Black);
                }
                ctrl if ctrl == "1" => {
                    style = style.fg(Color::Red);
                }
                ctrl if ctrl == "2" => {
                    style = style.fg(Color::Green);
                }
                ctrl if ctrl == "3" => {
                    style = style.fg(Color::Yellow);
                }
                ctrl if ctrl == "4" => {
                    style = style.fg(Color::Blue);
                }
                ctrl if ctrl == "5" => {
                    style = style.fg(Color::Magenta);
                }
                ctrl if ctrl == "6" => {
                    style = style.fg(Color::Cyan);
                }
                ctrl if ctrl == "7" => {
                    style = style.fg(Color::White);
                }
                // Bright Colors
                ctrl if ctrl == "8" => {
                    style = style.fg(Color::DarkGray);
                }
                ctrl if ctrl == "9" => {
                    style = style.fg(Color::LightRed);
                }
                ctrl if ctrl == "10" => {
                    style = style.fg(Color::LightGreen);
                }
                ctrl if ctrl == "11" => {
                    style = style.fg(Color::LightYellow);
                }
                ctrl if ctrl == "12" => {
                    style = style.fg(Color::LightBlue);
                }
                ctrl if ctrl == "13" => {
                    style = style.fg(Color::LightMagenta);
                }
                ctrl if ctrl == "14" => {
                    style = style.fg(Color::LightCyan);
                }
                // tui has no "Bright White" color code equivalent
                // White
                ctrl if ctrl == "15" => {
                    style = style.fg(Color::White);
                }
                // _ => panic!("control sequence not found"),
                _ => return Text::raw(self.text),
            }
        } else {
            return Text::raw(self.text);
        }
        Text::styled(self.text, style)
    }
}

impl fmt::Display for CtrlChunk {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let ctrl_code = self
            .ctrl
            .iter()
            .map(|c| {
                if c == "38;5;" {
                    format!("\x1B]{}", c)
                } else {
                    format!("\x1B[{}", c)
                }
            })
            .collect::<String>();
        if ctrl_code.is_empty() && self.text.is_empty() {
            Ok(())
        } else {
            write!(f, "{}{}", ctrl_code, self.text)
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct CtrlChars {
    parsed: Vec<CtrlChunk>,
}

impl fmt::Display for CtrlChars {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = self
            .parsed
            .iter()
            .map(CtrlChunk::to_string)
            .collect::<String>();
        write!(f, "{}", text)
    }
}

impl CtrlChars {
    pub fn parse(input: &str) -> Self {
        let mut parsed = Vec::new();

        let mut munch = Muncher::new(input);
        let pre_ctrl = munch.eat_until(|c| *c == '\x1B').collect::<String>();
        parsed.push(CtrlChunk::text(pre_ctrl));

        loop {
            if munch.is_done() {
                break;
            } else {
                parsed.push(CtrlChunk::parse(&mut munch))
            }
        }
        Self { parsed }
    }

    pub fn into_text<'a>(self) -> Vec<Text<'a>> {
        self.parsed.into_iter().map(CtrlChunk::into_text).collect()
    }
}
