//! All constants.

/// File name constants.
pub mod filename {
    /// Cargo manifest file
    pub const CARGO_TOML: &str = "Cargo.toml";
    /// Anchor manifest file
    pub const ANCHOR_TOML: &str = "Anchor.toml";
    /// Starting point of a Rust library
    pub const LIB_RS: &str = "lib.rs";
}

/// Directory name constants.
pub mod dirname {
    /// `src` directory
    pub const SRC: &str = "src";
    /// `target` directory
    pub const TARGET: &str = "target";
    /// `deploy` directory under `target` folder
    pub const DEPLOY: &str = "deploy";
    /// `programs_py` directory for Seahorse programs
    pub const PROGRAMS_PY: &str = "programs_py";
}

/// File extension constants.
pub mod extension {
    /// Rust extension
    pub const RS: &str = "rs";
    /// TOML extension
    pub const TOML: &str = "toml";
    /// ELF(.so) extension
    pub const SO: &str = "so";
    /// JSON extension
    pub const JSON: &str = "json";
    /// Python extension
    pub const PY: &str = "py";
}

/// Emoji constants.
pub mod emoji {
    use console::Emoji;

    /// Checkmark emoji
    pub const CHECKMARK: Emoji = Emoji("✔", "+");
    /// Cross emoji
    pub const CROSS: Emoji = Emoji("✖", "X");
}
