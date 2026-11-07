use gpui_kit::*;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ThemePreset {
    ZedOneDark,
    ZedAyuMirage,
    CatppuccinMocha,
    GtkOriginalPurple,
    ZedOneLight,
}

impl ThemePreset {
    pub const ALL: [ThemePreset; 5] = [
        ThemePreset::ZedOneDark,
        ThemePreset::ZedAyuMirage,
        ThemePreset::CatppuccinMocha,
        ThemePreset::GtkOriginalPurple,
        ThemePreset::ZedOneLight,
    ];

    pub fn name(&self) -> &'static str {
        match self {
            Self::ZedOneDark => "Zed One Dark",
            Self::ZedAyuMirage => "Zed Ayu Mirage",
            Self::CatppuccinMocha => "Catppuccin Mocha",
            Self::GtkOriginalPurple => "GTK Original Purple",
            Self::ZedOneLight => "Zed One Light",
        }
    }

    pub fn short_name(&self) -> &'static str {
        match self {
            Self::ZedOneDark => "One Dark",
            Self::ZedAyuMirage => "Ayu Mirage",
            Self::CatppuccinMocha => "Mocha",
            Self::GtkOriginalPurple => "GTK Purple",
            Self::ZedOneLight => "One Light",
        }
    }

    pub fn colors(&self) -> ThemeColors {
        match self {
            Self::ZedOneDark => ThemeColors {
                bg_app: rgb(0x181a1f),
                bg_panel: rgb(0x21252b),
                bg_elevated: rgb(0x282c34),
                bg_hover: rgb(0x323842),
                border: rgb(0x353b45),
                text: rgb(0xdce0e5),
                text_muted: rgb(0x7f848e),
                accent: rgb(0x61afef),
                success: rgb(0x98c379),
                warning: rgb(0xe5c07b),
                danger: rgb(0xe06c75),
            },
            Self::ZedAyuMirage => ThemeColors {
                bg_app: rgb(0x171b24),
                bg_panel: rgb(0x1f2430),
                bg_elevated: rgb(0x242b38),
                bg_hover: rgb(0x2d3648),
                border: rgb(0x333c4d),
                text: rgb(0xcccac2),
                text_muted: rgb(0x707a8c),
                accent: rgb(0xffcc66),
                success: rgb(0xbae67e),
                warning: rgb(0xffae57),
                danger: rgb(0xff6666),
            },
            Self::CatppuccinMocha => ThemeColors {
                bg_app: rgb(0x11111b),
                bg_panel: rgb(0x1e1e2e),
                bg_elevated: rgb(0x313244),
                bg_hover: rgb(0x45475a),
                border: rgb(0x45475a),
                text: rgb(0xcdd6f4),
                text_muted: rgb(0xa6adc8),
                accent: rgb(0xcba6f7),
                success: rgb(0xa6e3a1),
                warning: rgb(0xf9e2af),
                danger: rgb(0xf38ba8),
            },
            Self::GtkOriginalPurple => ThemeColors {
                bg_app: rgb(0x0d0a12),
                bg_panel: rgb(0x2e253e),
                bg_elevated: rgb(0x3c3150),
                bg_hover: rgb(0x4e445e),
                border: rgb(0x6b637a),
                text: rgb(0xe8e6ec),
                text_muted: rgb(0xa39bb0),
                accent: rgb(0xb48ead),
                success: rgb(0xa3be8c),
                warning: rgb(0xebcb8b),
                danger: rgb(0xbf616a),
            },
            Self::ZedOneLight => ThemeColors {
                bg_app: rgb(0xfafafa),
                bg_panel: rgb(0xf0f0f1),
                bg_elevated: rgb(0xe5e5e6),
                bg_hover: rgb(0xdadadb),
                border: rgb(0xd3d3d4),
                text: rgb(0x242529),
                text_muted: rgb(0x696c77),
                accent: rgb(0x4078f2),
                success: rgb(0x50a14f),
                warning: rgb(0xc18401),
                danger: rgb(0xe45649),
            },
        }
    }
}

#[derive(Clone, Copy)]
pub struct ThemeColors {
    pub bg_app: Rgba,
    pub bg_panel: Rgba,
    pub bg_elevated: Rgba,
    pub bg_hover: Rgba,
    pub border: Rgba,
    pub text: Rgba,
    pub text_muted: Rgba,
    pub accent: Rgba,
    pub success: Rgba,
    pub warning: Rgba,
    pub danger: Rgba,
}
