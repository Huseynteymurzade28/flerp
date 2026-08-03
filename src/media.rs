use clap::ValueEnum;
use image::DynamicImage;
use ratatui::layout::Rect;
use ratatui::Frame;
use ratatui_image::picker::{Picker, ProtocolType};
use ratatui_image::protocol::StatefulProtocol;
use ratatui_image::{Resize, StatefulImage};

/// Which terminal graphics protocol to use.
///
/// Detection asks the terminal and trusts the answer, but the query is swallowed
/// by some multiplexers and SSH setups, leaving a capable terminal stuck on
/// half-blocks. This lets the user say what their terminal can really do.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
#[value(rename_all = "lower")]
pub enum GraphicsMode {
    /// Ask the terminal what it supports.
    Auto,
    Kitty,
    Sixel,
    Iterm2,
    /// Coloured character cells; works everywhere, but is an approximation.
    Halfblocks,
}

impl GraphicsMode {
    fn protocol(self) -> Option<ProtocolType> {
        match self {
            GraphicsMode::Auto => None,
            GraphicsMode::Kitty => Some(ProtocolType::Kitty),
            GraphicsMode::Sixel => Some(ProtocolType::Sixel),
            GraphicsMode::Iterm2 => Some(ProtocolType::Iterm2),
            GraphicsMode::Halfblocks => Some(ProtocolType::Halfblocks),
        }
    }
}

/// One renderable image, whatever its origin.
#[derive(Clone)]
pub struct MediaItem {
    /// Stable identity, used to decide whether the encoded protocol state can be
    /// reused across frames.
    pub key: String,
    pub title: String,
    pub detail: String,
    /// 1-based page this image sits on, for paged documents.
    pub page: Option<usize>,
    pub image: DynamicImage,
}

/// Holds the terminal graphics protocol and the currently encoded image.
///
/// This lives outside `AppState` on purpose: the encoded protocol state is
/// neither cloneable nor meaningful to persist, and re-encoding on every frame
/// would flood the terminal.
pub struct MediaRenderer {
    picker: Option<Picker>,
    protocol: Option<StatefulProtocol>,
    current_key: Option<String>,
    error: Option<String>,
}

impl MediaRenderer {
    /// Query the terminal for its graphics capabilities.
    ///
    /// Must run before the alternate screen is entered and before raw mode is
    /// enabled, because the query writes an escape sequence to stdout and reads
    /// the reply.
    pub fn detect() -> Self {
        Self::with_mode(GraphicsMode::Auto)
    }

    /// Build a renderer, overriding the detected protocol when `mode` names one.
    ///
    /// The query still runs even for an override, because it also reports the
    /// terminal's cell size, which is what image scaling depends on.
    pub fn with_mode(mode: GraphicsMode) -> Self {
        let mut renderer = match Picker::from_query_stdio() {
            Ok(picker) => Self {
                picker: Some(picker),
                protocol: None,
                current_key: None,
                error: None,
            },
            // A terminal that will not answer the query can still show
            // half-blocks, which beats showing nothing at all.
            Err(error) => Self {
                error: (mode == GraphicsMode::Auto)
                    .then(|| format!("graphics protocol query failed: {error}")),
                ..Self::halfblocks()
            },
        };

        if let (Some(protocol), Some(picker)) = (mode.protocol(), renderer.picker.as_mut()) {
            picker.set_protocol_type(protocol);
        }

        renderer
    }

    /// Half-block fallback with an assumed cell size, for terminals that cannot
    /// be queried.
    pub fn halfblocks() -> Self {
        Self {
            picker: Some(Picker::halfblocks()),
            protocol: None,
            current_key: None,
            error: None,
        }
    }

    /// Human-readable name of the protocol in use.
    pub fn backend_label(&self) -> &'static str {
        match self.picker.as_ref().map(Picker::protocol_type) {
            Some(ProtocolType::Kitty) => "Kitty graphics",
            Some(ProtocolType::Sixel) => "Sixel",
            Some(ProtocolType::Iterm2) => "iTerm2 inline",
            Some(ProtocolType::Halfblocks) => "Unicode half-blocks",
            None => "unavailable",
        }
    }

    /// True when the terminal can show real pixels rather than coloured cells.
    pub fn is_pixel_perfect(&self) -> bool {
        matches!(
            self.picker.as_ref().map(Picker::protocol_type),
            Some(ProtocolType::Kitty) | Some(ProtocolType::Sixel) | Some(ProtocolType::Iterm2)
        )
    }

    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    /// Prepare `item` for display, reusing the existing protocol state when the
    /// same image is still selected.
    pub fn select(&mut self, item: &MediaItem) {
        if self.current_key.as_deref() == Some(item.key.as_str()) {
            return;
        }

        let Some(picker) = self.picker.as_mut() else {
            return;
        };

        self.protocol = Some(picker.new_resize_protocol(item.image.clone()));
        self.current_key = Some(item.key.clone());
    }

    pub fn clear(&mut self) {
        self.protocol = None;
        self.current_key = None;
    }

    /// Draw the selected image into `area`. Returns false when there is nothing
    /// to draw, so the caller can show a placeholder instead.
    pub fn render(&mut self, frame: &mut Frame, area: Rect) -> bool {
        let Some(protocol) = self.protocol.as_mut() else {
            return false;
        };

        frame.render_stateful_widget(
            StatefulImage::default().resize(Resize::Fit(None)),
            area,
            protocol,
        );

        if let Err(error) = protocol.last_encoding_result().unwrap_or(Ok(())) {
            self.error = Some(error.to_string());
        }

        true
    }
}
