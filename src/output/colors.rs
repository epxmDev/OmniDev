use std::io::IsTerminal;

use owo_colors::OwoColorize;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Theme {
    pub colors: bool,
    pub ascii: bool,
}

impl Theme {
    pub fn detect(show_colors: bool) -> Self {
        let colors = show_colors
            && std::io::stdout().is_terminal()
            && std::env::var_os("NO_COLOR").is_none()
            && std::env::var("TERM").map_or(true, |term| term != "dumb");
        Self {
            colors,
            ascii: detect_ascii(),
        }
    }

    pub fn plain() -> Self {
        Self {
            colors: false,
            ascii: true,
        }
    }

    pub fn accent(&self, text: &str) -> String {
        if self.colors {
            text.cyan().bold().to_string()
        } else {
            text.to_owned()
        }
    }

    pub fn heading(&self, text: &str) -> String {
        if self.colors {
            text.bright_blue().bold().to_string()
        } else {
            text.to_owned()
        }
    }

    pub fn success(&self, text: &str) -> String {
        if self.colors {
            text.green().to_string()
        } else {
            text.to_owned()
        }
    }

    pub fn warning(&self, text: &str) -> String {
        if self.colors {
            text.yellow().to_string()
        } else {
            text.to_owned()
        }
    }

    pub fn error(&self, text: &str) -> String {
        if self.colors {
            text.red().bold().to_string()
        } else {
            text.to_owned()
        }
    }

    pub fn muted(&self, text: &str) -> String {
        if self.colors {
            text.bright_black().to_string()
        } else {
            text.to_owned()
        }
    }

    pub fn heat(&self, level: u8, glyph: char) -> String {
        let cell = glyph.to_string();
        if !self.colors {
            return cell;
        }
        match level {
            0 => cell.bright_black().to_string(),
            1 => cell.blue().to_string(),
            2 => cell.bright_blue().to_string(),
            3 => cell.cyan().to_string(),
            _ => cell.bright_green().bold().to_string(),
        }
    }

    pub fn symbols(&self) -> Symbols {
        if self.ascii {
            Symbols::ascii()
        } else {
            Symbols::unicode()
        }
    }
}

pub struct Symbols {
    pub success: &'static str,
    pub failure: &'static str,
    pub warning: &'static str,
    pub bullet: &'static str,
    pub arrow: &'static str,
    pub repository: &'static str,
    pub trophy: &'static str,
}

impl Symbols {
    fn unicode() -> Self {
        Self {
            success: "✔",
            failure: "✖",
            warning: "⚠",
            bullet: "•",
            arrow: "→",
            repository: "◆",
            trophy: "★",
        }
    }

    fn ascii() -> Self {
        Self {
            success: "[ok]",
            failure: "[x]",
            warning: "[!]",
            bullet: "*",
            arrow: "->",
            repository: "#",
            trophy: "*",
        }
    }
}

fn detect_ascii() -> bool {
    if std::env::var_os("OMNIDEV_ASCII").is_some() {
        return true;
    }
    if std::env::var("TERM").is_ok_and(|term| term == "dumb") {
        return true;
    }
    let locale = ["LC_ALL", "LC_CTYPE", "LANG"]
        .iter()
        .find_map(|name| std::env::var(name).ok().filter(|value| !value.is_empty()));
    match locale {
        Some(value) => {
            let lowered = value.to_ascii_lowercase();
            !(lowered.contains("utf-8") || lowered.contains("utf8"))
        }
        None => false,
    }
}

pub const LOGO: &str = r"
   ____  __  __ _   _ ___ ____  _______     __
  / __ \|  \/  | \ | |_ _|  _ \| ____\ \   / /
 | |  | | |\/| |  \| || || | | |  _|  \ \ / /
 | |__| | |  | | |\  || || |_| | |___  \ V /
  \____/|_|  |_|_| \_|___|____/|_____|  \_/
";
