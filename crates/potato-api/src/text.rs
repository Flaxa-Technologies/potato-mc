use serde::{Deserialize, Serialize};

/// Standard Minecraft chat colors matching Adventure (`NamedTextColor`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NamedTextColor {
    Black,
    DarkBlue,
    DarkGreen,
    DarkAqua,
    DarkRed,
    DarkPurple,
    Gold,
    Gray,
    DarkGray,
    Blue,
    Green,
    Aqua,
    Red,
    LightPurple,
    Yellow,
    White,
}

impl NamedTextColor {
    /// Returns the legacy Minecraft section formatting character ('0'-'f').
    pub fn to_char(self) -> char {
        match self {
            Self::Black => '0',
            Self::DarkBlue => '1',
            Self::DarkGreen => '2',
            Self::DarkAqua => '3',
            Self::DarkRed => '4',
            Self::DarkPurple => '5',
            Self::Gold => '6',
            Self::Gray => '7',
            Self::DarkGray => '8',
            Self::Blue => '9',
            Self::Green => 'a',
            Self::Aqua => 'b',
            Self::Red => 'c',
            Self::LightPurple => 'd',
            Self::Yellow => 'e',
            Self::White => 'f',
        }
    }

    /// Parses a legacy color code character.
    pub fn from_char(c: char) -> Option<Self> {
        match c.to_ascii_lowercase() {
            '0' => Some(Self::Black),
            '1' => Some(Self::DarkBlue),
            '2' => Some(Self::DarkGreen),
            '3' => Some(Self::DarkAqua),
            '4' => Some(Self::DarkRed),
            '5' => Some(Self::DarkPurple),
            '6' => Some(Self::Gold),
            '7' => Some(Self::Gray),
            '8' => Some(Self::DarkGray),
            '9' => Some(Self::Blue),
            'a' => Some(Self::Green),
            'b' => Some(Self::Aqua),
            'c' => Some(Self::Red),
            'd' => Some(Self::LightPurple),
            'e' => Some(Self::Yellow),
            'f' => Some(Self::White),
            _ => None,
        }
    }

    /// Returns the name of the color matching MiniMessage tags (e.g., "red", "gold").
    pub fn name(self) -> &'static str {
        match self {
            Self::Black => "black",
            Self::DarkBlue => "dark_blue",
            Self::DarkGreen => "dark_green",
            Self::DarkAqua => "dark_aqua",
            Self::DarkRed => "dark_red",
            Self::DarkPurple => "dark_purple",
            Self::Gold => "gold",
            Self::Gray => "gray",
            Self::DarkGray => "dark_gray",
            Self::Blue => "blue",
            Self::Green => "green",
            Self::Aqua => "aqua",
            Self::Red => "red",
            Self::LightPurple => "light_purple",
            Self::Yellow => "yellow",
            Self::White => "white",
        }
    }
}

/// Text decorations matching Adventure (`TextDecoration`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TextDecoration {
    Obfuscated,
    Bold,
    Strikethrough,
    Underlined,
    Italic,
}

impl TextDecoration {
    pub fn to_char(self) -> char {
        match self {
            Self::Obfuscated => 'k',
            Self::Bold => 'l',
            Self::Strikethrough => 'm',
            Self::Underlined => 'n',
            Self::Italic => 'o',
        }
    }

    pub fn from_char(c: char) -> Option<Self> {
        match c.to_ascii_lowercase() {
            'k' => Some(Self::Obfuscated),
            'l' => Some(Self::Bold),
            'm' => Some(Self::Strikethrough),
            'n' => Some(Self::Underlined),
            'o' => Some(Self::Italic),
            _ => None,
        }
    }
}

/// Rich text component inspired by Paper's Adventure Component API.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Component {
    pub text: String,
    pub color: Option<NamedTextColor>,
    pub bold: bool,
    pub italic: bool,
    pub underlined: bool,
    pub strikethrough: bool,
    pub obfuscated: bool,
    pub extra: Vec<Component>,
}

impl Component {
    /// Creates a component with the specified text.
    pub fn text(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            ..Default::default()
        }
    }

    /// Creates an empty component.
    pub fn empty() -> Self {
        Self::default()
    }

    /// Sets the color of this component.
    pub fn color(mut self, color: NamedTextColor) -> Self {
        self.color = Some(color);
        self
    }

    /// Sets bold styling.
    pub fn bold(mut self) -> Self {
        self.bold = true;
        self
    }

    /// Sets italic styling.
    pub fn italic(mut self) -> Self {
        self.italic = true;
        self
    }

    /// Sets underline styling.
    pub fn underlined(mut self) -> Self {
        self.underlined = true;
        self
    }

    /// Sets strikethrough styling.
    pub fn strikethrough(mut self) -> Self {
        self.strikethrough = true;
        self
    }

    /// Sets obfuscated (magic) styling.
    pub fn obfuscated(mut self) -> Self {
        self.obfuscated = true;
        self
    }

    /// Appends a child component.
    pub fn append(mut self, child: Component) -> Self {
        self.extra.push(child);
        self
    }

    /// Formats this component into a Minecraft legacy formatting string with `§` color codes.
    pub fn to_legacy_string(&self) -> String {
        let mut out = String::new();
        if let Some(col) = self.color {
            out.push('§');
            out.push(col.to_char());
        }
        if self.bold {
            out.push_str("§l");
        }
        if self.italic {
            out.push_str("§o");
        }
        if self.underlined {
            out.push_str("§n");
        }
        if self.strikethrough {
            out.push_str("§m");
        }
        if self.obfuscated {
            out.push_str("§k");
        }
        out.push_str(&self.text);
        for child in &self.extra {
            out.push_str(&child.to_legacy_string());
        }
        out
    }

    /// Strips all formatting and returns plain unformatted text.
    pub fn to_plain_text(&self) -> String {
        let mut out = self.text.clone();
        for child in &self.extra {
            out.push_str(&child.to_plain_text());
        }
        out
    }

    /// Parses text containing `&` color codes into a Component.
    pub fn from_legacy_ampersand(input: &str) -> Self {
        Self::from_code_string(input, '&')
    }

    /// Parses text containing `§` color codes into a Component.
    pub fn from_legacy_section(input: &str) -> Self {
        Self::from_code_string(input, '§')
    }

    fn from_code_string(input: &str, delimiter: char) -> Self {
        let mut root = Component::empty();
        let mut current_text = String::new();
        let mut current_color: Option<NamedTextColor> = None;
        let mut bold = false;
        let mut italic = false;
        let mut underlined = false;
        let mut strikethrough = false;
        let mut obfuscated = false;

        let chars: Vec<char> = input.chars().collect();
        let mut i = 0;

        while i < chars.len() {
            if chars[i] == delimiter && i + 1 < chars.len() {
                let code = chars[i + 1];
                if !current_text.is_empty() {
                    let comp = Component {
                        text: current_text.clone(),
                        color: current_color,
                        bold,
                        italic,
                        underlined,
                        strikethrough,
                        obfuscated,
                        extra: Vec::new(),
                    };
                    root = root.append(comp);
                    current_text.clear();
                }

                if let Some(col) = NamedTextColor::from_char(code) {
                    current_color = Some(col);
                    bold = false;
                    italic = false;
                    underlined = false;
                    strikethrough = false;
                    obfuscated = false;
                } else if let Some(dec) = TextDecoration::from_char(code) {
                    match dec {
                        TextDecoration::Bold => bold = true,
                        TextDecoration::Italic => italic = true,
                        TextDecoration::Underlined => underlined = true,
                        TextDecoration::Strikethrough => strikethrough = true,
                        TextDecoration::Obfuscated => obfuscated = true,
                    }
                } else if code == 'r' || code == 'R' {
                    current_color = None;
                    bold = false;
                    italic = false;
                    underlined = false;
                    strikethrough = false;
                    obfuscated = false;
                }
                i += 2;
            } else {
                current_text.push(chars[i]);
                i += 1;
            }
        }

        if !current_text.is_empty() {
            let comp = Component {
                text: current_text,
                color: current_color,
                bold,
                italic,
                underlined,
                strikethrough,
                obfuscated,
                extra: Vec::new(),
            };
            root = root.append(comp);
        }

        root
    }

    /// Parses simplified MiniMessage tags (e.g., `<red>text</red>`, `<gold><bold>title</bold></gold>`).
    pub fn from_mini_message(input: &str) -> Self {
        // Simple and robust parser for tag-based formatting
        let mut root = Component::empty();
        let mut current_text = String::new();
        let mut color_stack: Vec<NamedTextColor> = Vec::new();
        let mut bold = false;
        let mut italic = false;
        let mut underlined = false;

        let parts: Vec<&str> = input.split('<').collect();
        for (idx, part) in parts.iter().enumerate() {
            if idx == 0 {
                if !part.is_empty() {
                    root = root.append(Component::text(*part));
                }
                continue;
            }

            if let Some(closing_idx) = part.find('>') {
                let tag = &part[..closing_idx];
                let rest = &part[closing_idx + 1..];

                match tag.to_lowercase().as_str() {
                    "red" => color_stack.push(NamedTextColor::Red),
                    "/red" => { color_stack.pop(); }
                    "green" => color_stack.push(NamedTextColor::Green),
                    "/green" => { color_stack.pop(); }
                    "blue" => color_stack.push(NamedTextColor::Blue),
                    "/blue" => { color_stack.pop(); }
                    "gold" => color_stack.push(NamedTextColor::Gold),
                    "/gold" => { color_stack.pop(); }
                    "yellow" => color_stack.push(NamedTextColor::Yellow),
                    "/yellow" => { color_stack.pop(); }
                    "aqua" => color_stack.push(NamedTextColor::Aqua),
                    "/aqua" => { color_stack.pop(); }
                    "bold" | "b" => bold = true,
                    "/bold" | "/b" => bold = false,
                    "italic" | "i" => italic = true,
                    "/italic" | "/i" => italic = false,
                    "underlined" | "u" => underlined = true,
                    "/underlined" | "/u" => underlined = false,
                    _ => {}
                }

                if !rest.is_empty() {
                    let mut comp = Component::text(rest);
                    if let Some(col) = color_stack.last() {
                        comp = comp.color(*col);
                    }
                    if bold {
                        comp = comp.bold();
                    }
                    if italic {
                        comp = comp.italic();
                    }
                    if underlined {
                        comp = comp.underlined();
                    }
                    root = root.append(comp);
                }
            } else {
                current_text.push('<');
                current_text.push_str(part);
            }
        }

        if !current_text.is_empty() {
            root = root.append(Component::text(current_text));
        }

        root
    }
}
