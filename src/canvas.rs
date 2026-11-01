//! WHATWG Canvas 2D API for osbrowser.
//!
//! Provides the exact same WHATWG types and trait methods as webcore,
//! but without the 80+ software rasterizer dependencies (webcanvas, accelerate,
//! cosmic-text, tiny-skia, swash, skrifa).

use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub const BLACK: Self = Self { r: 0, g: 0, b: 0, a: 255 };
    pub const WHITE: Self = Self { r: 255, g: 255, b: 255, a: 255 };
    pub const TRANSPARENT: Self = Self { r: 0, g: 0, b: 0, a: 0 };

    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    pub fn to_css(&self) -> String {
        if self.a == 255 {
            format!("#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
        } else {
            let a = self.a as f32 / 255.0;
            format!("rgba({}, {}, {}, {:.3})", self.r, self.g, self.b, a)
        }
    }
}

pub fn parse_color_css(css: &str) -> Option<Color> {
    let s = css.trim();
    if let Some(hex) = s.strip_prefix('#') {
        if hex.len() == 6 {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
            return Some(Color::rgb(r, g, b));
        }
    }
    None
}

static TRACE_ENABLED: AtomicBool = AtomicBool::new(false);

pub fn set_trace_enabled(enabled: bool) {
    TRACE_ENABLED.store(enabled, Ordering::Relaxed);
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum LineCap {
    #[default]
    Butt,
    Round,
    Square,
}

impl LineCap {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Butt => "butt",
            Self::Round => "round",
            Self::Square => "square",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum LineJoin {
    #[default]
    Miter,
    Round,
    Bevel,
}

impl LineJoin {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Miter => "miter",
            Self::Round => "round",
            Self::Bevel => "bevel",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum TextAlign {
    #[default]
    Start,
    End,
    Left,
    Right,
    Center,
}

impl TextAlign {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "start" => Some(Self::Start),
            "end" => Some(Self::End),
            "left" => Some(Self::Left),
            "right" => Some(Self::Right),
            "center" => Some(Self::Center),
            _ => None,
        }
    }
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Start => "start",
            Self::End => "end",
            Self::Left => "left",
            Self::Right => "right",
            Self::Center => "center",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum TextBaseline {
    Top,
    Hanging,
    Middle,
    #[default]
    Alphabetic,
    Ideographic,
    Bottom,
}

impl TextBaseline {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "top" => Some(Self::Top),
            "hanging" => Some(Self::Hanging),
            "middle" => Some(Self::Middle),
            "alphabetic" => Some(Self::Alphabetic),
            "ideographic" => Some(Self::Ideographic),
            "bottom" => Some(Self::Bottom),
            _ => None,
        }
    }
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Top => "top",
            Self::Hanging => "hanging",
            Self::Middle => "middle",
            Self::Alphabetic => "alphabetic",
            Self::Ideographic => "ideographic",
            Self::Bottom => "bottom",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum FillRule {
    #[default]
    NonZero,
    EvenOdd,
}

impl FillRule {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "evenodd" => Some(Self::EvenOdd),
            _ => Some(Self::NonZero),
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::NonZero => "nonzero",
            Self::EvenOdd => "evenodd",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum CompositeOp {
    #[default]
    SourceOver,
    SourceIn,
    SourceOut,
    SourceAtop,
    DestinationOver,
    DestinationIn,
    DestinationOut,
    DestinationAtop,
    Lighter,
    Copy,
    Xor,
    Multiply,
    Screen,
    Overlay,
    Darken,
    Lighten,
}

impl CompositeOp {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "source-over" => Some(Self::SourceOver),
            "copy" => Some(Self::Copy),
            _ => None,
        }
    }
    pub fn as_str(self) -> &'static str {
        match self {
            Self::SourceOver => "source-over",
            Self::SourceIn => "source-in",
            Self::SourceOut => "source-out",
            Self::SourceAtop => "source-atop",
            Self::DestinationOver => "destination-over",
            Self::DestinationIn => "destination-in",
            Self::DestinationOut => "destination-out",
            Self::DestinationAtop => "destination-atop",
            Self::Lighter => "lighter",
            Self::Copy => "copy",
            Self::Xor => "xor",
            Self::Multiply => "multiply",
            Self::Screen => "screen",
            Self::Overlay => "overlay",
            Self::Darken => "darken",
            Self::Lighten => "lighten",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum SmoothingQuality {
    #[default]
    Low,
    Medium,
    High,
}

impl SmoothingQuality {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "low" => Some(Self::Low),
            "medium" => Some(Self::Medium),
            "high" => Some(Self::High),
            _ => None,
        }
    }
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Direction {
    #[default]
    Inherit,
    Ltr,
    Rtl,
}

impl Direction {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "ltr" => Some(Self::Ltr),
            "rtl" => Some(Self::Rtl),
            "inherit" => Some(Self::Inherit),
            _ => None,
        }
    }
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ltr => "ltr",
            Self::Rtl => "rtl",
            Self::Inherit => "inherit",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum FontKerning {
    #[default]
    Auto,
    Normal,
    None,
}

impl FontKerning {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "auto" => Some(Self::Auto),
            "normal" => Some(Self::Normal),
            "none" => Some(Self::None),
            _ => None,
        }
    }
    pub fn as_str(&self) -> &'static str { "auto" }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum FontStretch {
    #[default]
    Normal,
}

impl FontStretch {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "normal" => Some(Self::Normal),
            _ => None,
        }
    }
    pub fn as_str(&self) -> &'static str { "normal" }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum FontVariantCaps {
    #[default]
    Normal,
}

impl FontVariantCaps {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "normal" => Some(Self::Normal),
            _ => None,
        }
    }
    pub fn as_str(&self) -> &'static str { "normal" }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum TextRendering {
    #[default]
    Auto,
}

impl TextRendering {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "auto" => Some(Self::Auto),
            _ => None,
        }
    }
    pub fn as_str(&self) -> &'static str { "auto" }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum FontWeight {
    #[default]
    Normal,
    Bold,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum FontStyle {
    #[default]
    Normal,
    Italic,
}

#[derive(Clone, Debug, Default)]
pub struct Font {
    pub family: String,
    pub size: f32,
    pub weight: FontWeight,
    pub style: FontStyle,
}

impl Font {
    pub fn to_css(&self) -> String {
        let style = match self.style {
            FontStyle::Normal => "",
            FontStyle::Italic => "italic ",
        };
        let weight = match self.weight {
            FontWeight::Normal => "",
            FontWeight::Bold => "bold ",
        };
        let family = if self.family.is_empty() { "sans-serif" } else { &self.family };
        format!("{style}{weight}{}px {family}", self.size)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Matrix {
    pub a: f32,
    pub b: f32,
    pub c: f32,
    pub d: f32,
    pub e: f32,
    pub f: f32,
}

impl Matrix {
    pub const IDENTITY: Self = Self {
        a: 1.0, b: 0.0, c: 0.0, d: 1.0, e: 0.0, f: 0.0,
    };
}

#[derive(Clone, Debug, Default)]
pub struct Shadow {
    pub color: Color,
    pub blur: f32,
    pub offset_x: f32,
    pub offset_y: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Image {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
    pub origin_clean: bool,
}

impl Image {
    pub fn from_rgba(width: u32, height: u32, pixels: Vec<u8>) -> Self {
        Self { width, height, pixels, origin_clean: true }
    }
    pub fn with_origin_clean(mut self, clean: bool) -> Self {
        self.origin_clean = clean;
        self
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ImageData {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
    pub color_space: &'static str,
}

impl Default for ImageData {
    fn default() -> Self {
        Self { width: 0, height: 0, data: Vec::new(), color_space: "srgb" }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ColorStop {
    pub offset: f32,
    pub color: Color,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum GradientKind {
    Linear { x0: f32, y0: f32, x1: f32, y1: f32 },
    Radial { x0: f32, y0: f32, r0: f32, x1: f32, y1: f32, r1: f32 },
    Conic { start_angle: f32, x: f32, y: f32 },
}

#[derive(Clone, Debug, PartialEq)]
pub struct Gradient {
    pub kind: GradientKind,
    pub stops: Vec<ColorStop>,
}

impl Gradient {
    pub fn linear(x0: f32, y0: f32, x1: f32, y1: f32) -> Self {
        Self { kind: GradientKind::Linear { x0, y0, x1, y1 }, stops: Vec::new() }
    }
    pub fn radial(x0: f32, y0: f32, r0: f32, x1: f32, y1: f32, r1: f32) -> Self {
        Self { kind: GradientKind::Radial { x0, y0, r0, x1, y1, r1 }, stops: Vec::new() }
    }
    pub fn conic(start_angle: f32, x: f32, y: f32) -> Self {
        Self { kind: GradientKind::Conic { start_angle, x, y }, stops: Vec::new() }
    }
    pub fn add_color_stop(&mut self, offset: f32, color: Color) {
        self.stops.push(ColorStop { offset, color });
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Repetition {
    #[default]
    Repeat,
    RepeatX,
    RepeatY,
    NoRepeat,
}

impl Repetition {
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim() {
            "" | "repeat" => Some(Self::Repeat),
            "repeat-x" => Some(Self::RepeatX),
            "repeat-y" => Some(Self::RepeatY),
            "no-repeat" => Some(Self::NoRepeat),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Pattern {
    pub image: Image,
    pub repetition: Repetition,
    pub transform: Matrix,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Paint {
    Color(Color),
    Gradient(Gradient),
    Pattern(Pattern),
}

#[derive(Clone, Debug, PartialEq)]
pub enum PathOp {
    ClosePath,
    MoveTo(f32, f32),
    LineTo(f32, f32),
    QuadraticCurveTo { cx: f32, cy: f32, x: f32, y: f32 },
    BezierCurveTo { cx1: f32, cy1: f32, cx2: f32, cy2: f32, x: f32, y: f32 },
    ArcTo { x1: f32, y1: f32, x2: f32, y2: f32, radius: f32 },
    Rect { x: f32, y: f32, w: f32, h: f32 },
    RoundRect { x: f32, y: f32, w: f32, h: f32, radii: [f32; 4] },
    Arc { x: f32, y: f32, r: f32, start: f32, end: f32, ccw: bool },
    Ellipse { x: f32, y: f32, rx: f32, ry: f32, rotation: f32, start: f32, end: f32, ccw: bool },
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Path2D {
    pub ops: Vec<PathOp>,
}

#[derive(Clone, Debug, Default, serde::Deserialize)]
pub struct TextMetrics {
    pub width: f32,
    #[serde(default)]
    pub actual_bounding_box_left: f32,
    #[serde(default)]
    pub actual_bounding_box_right: f32,
    #[serde(default)]
    pub font_bounding_box_ascent: f32,
    #[serde(default)]
    pub font_bounding_box_descent: f32,
    #[serde(default)]
    pub actual_bounding_box_ascent: f32,
    #[serde(default)]
    pub actual_bounding_box_descent: f32,
    #[serde(default)]
    pub em_height_ascent: f32,
    #[serde(default)]
    pub em_height_descent: f32,
    #[serde(default)]
    pub hanging_baseline: f32,
    #[serde(default)]
    pub alphabetic_baseline: f32,
    #[serde(default)]
    pub ideographic_baseline: f32,
}

pub(crate) fn reset() {}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ContextAttributes {
    pub alpha: bool,
    pub desynchronized: bool,
    pub color_space: &'static str,
    pub color_type: &'static str,
    pub will_read_frequently: bool,
}

impl Default for ContextAttributes {
    fn default() -> Self {
        ContextAttributes {
            alpha: true,
            desynchronized: false,
            color_space: "srgb",
            color_type: "unorm8",
            will_read_frequently: false,
        }
    }
}

fn decode_base64(input: &str) -> Vec<u8> {
    const TABLE: &[u8; 128] = &{
        let mut t = [255u8; 128];
        let chars = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut i = 0;
        while i < 64 {
            t[chars[i] as usize] = i as u8;
            i += 1;
        }
        t
    };
    let mut out = Vec::with_capacity(input.len() * 3 / 4);
    let mut buf = 0u32;
    let mut bits = 0;
    for &b in input.as_bytes() {
        if b == b'=' || b.is_ascii_whitespace() { continue; }
        if b >= 128 { continue; }
        let val = TABLE[b as usize];
        if val == 255 { continue; }
        buf = (buf << 6) | (val as u32);
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((buf >> bits) as u8);
        }
    }
    out
}

pub(crate) fn with_context<R>(page: u64, node: u32, f: impl FnOnce(&mut dyn Canvas) -> R) -> R {
    let mut canvas = BrowserCanvas::new(page, node);
    let result = f(&mut canvas);
    canvas.flush();
    result
}

/// Canvas implementation that redirects commands directly to the real system browser
/// via osbrowser's WebSocket/HTTP bridge (`canvas_draw`, `canvas_query`, and `canvas_image`).
pub struct BrowserCanvas {
    page: u64,
    node: u32,
    commands: Vec<serde_json::Value>,

    // Drawing state
    fill_style: String,
    stroke_style: String,
    font_css: String,
    filter_css: String,
    shadow_color_css: String,
    shadow_blur: f32,
    shadow_offset_x: f32,
    shadow_offset_y: f32,
    line_width: f32,
    line_cap: LineCap,
    line_join: LineJoin,
    miter_limit: f32,
    global_alpha: f32,
    image_smoothing: bool,
    smoothing_quality: SmoothingQuality,
    composite: CompositeOp,
    text_align: TextAlign,
    text_baseline: TextBaseline,
    direction: Direction,
    letter_spacing: String,
    word_spacing: String,
    font_kerning: FontKerning,
    font_stretch: FontStretch,
    font_variant_caps: FontVariantCaps,
    text_rendering: TextRendering,
    lang: String,
    line_dash: Vec<f32>,
    line_dash_offset: f32,
    transform: Matrix,
}

impl BrowserCanvas {
    pub fn new(page: u64, node: u32) -> Self {
        Self {
            page,
            node,
            commands: Vec::new(),
            fill_style: "#000000".into(),
            stroke_style: "#000000".into(),
            font_css: "10px sans-serif".into(),
            filter_css: "none".into(),
            shadow_color_css: "rgba(0, 0, 0, 0)".into(),
            shadow_blur: 0.0,
            shadow_offset_x: 0.0,
            shadow_offset_y: 0.0,
            line_width: 1.0,
            line_cap: LineCap::Butt,
            line_join: LineJoin::Miter,
            miter_limit: 10.0,
            global_alpha: 1.0,
            image_smoothing: true,
            smoothing_quality: SmoothingQuality::Low,
            composite: CompositeOp::SourceOver,
            text_align: TextAlign::Start,
            text_baseline: TextBaseline::Alphabetic,
            direction: Direction::Inherit,
            letter_spacing: "0px".into(),
            word_spacing: "0px".into(),
            font_kerning: FontKerning::Auto,
            font_stretch: FontStretch::Normal,
            font_variant_caps: FontVariantCaps::Normal,
            text_rendering: TextRendering::Auto,
            lang: String::new(),
            line_dash: Vec::new(),
            line_dash_offset: 0.0,
            transform: Matrix::IDENTITY,
        }
    }

    pub fn flush(&mut self) {
        if !self.commands.is_empty() {
            let cmds = std::mem::take(&mut self.commands);
            crate::page::write(self.page, "canvas_draw", serde_json::json!([self.node, cmds]));
            crate::page::flush();
        }
    }
}

impl Drop for BrowserCanvas {
    fn drop(&mut self) {
        self.flush();
    }
}

impl Canvas for BrowserCanvas {
    fn text_baseline(&self) -> TextBaseline { self.text_baseline }
    fn line_cap(&self) -> LineCap { self.line_cap }
    fn line_join(&self) -> LineJoin { self.line_join }
    fn text_align(&self) -> TextAlign { self.text_align }
    fn word_spacing_css(&self) -> String { self.word_spacing.clone() }
    fn letter_spacing_css(&self) -> String { self.letter_spacing.clone() }
    fn font_kerning(&self) -> FontKerning { self.font_kerning }
    fn font_stretch(&self) -> FontStretch { self.font_stretch }
    fn font_variant_caps(&self) -> FontVariantCaps { self.font_variant_caps }
    fn text_rendering(&self) -> TextRendering { self.text_rendering }
    fn direction(&self) -> Direction { self.direction }
    fn lang(&self) -> &str { &self.lang }
    fn current_font_css(&self) -> String { self.font_css.clone() }
    fn fill_style_css(&self) -> String { self.fill_style.clone() }
    fn stroke_style_css(&self) -> String { self.stroke_style.clone() }
    fn filter_css(&self) -> String { self.filter_css.clone() }
    fn global_composite_operation(&self) -> CompositeOp { self.composite }
    fn image_smoothing_quality(&self) -> SmoothingQuality { self.smoothing_quality }
    fn shadow_color_css(&self) -> String { self.shadow_color_css.clone() }
    fn shadow_blur(&self) -> f32 { self.shadow_blur }
    fn shadow_offset_x(&self) -> f32 { self.shadow_offset_x }
    fn shadow_offset_y(&self) -> f32 { self.shadow_offset_y }
    fn get_transform(&self) -> Matrix { self.transform }
    fn get_line_dash(&self) -> Vec<f32> { self.line_dash.clone() }
    fn is_context_lost(&self) -> bool {
        crate::page::read(self.page, "canvas_query", serde_json::json!([self.node, "isContextLost", []]))
    }
    fn context_attributes(&self) -> ContextAttributes { ContextAttributes::default() }

    fn measure_text(&mut self, text: &str) -> TextMetrics {
        self.flush();
        crate::page::read(
            self.page,
            "canvas_query",
            serde_json::json!([self.node, "measureText", [text]]),
        )
    }

    fn is_point_in_path(&self, x: f32, y: f32, rule: FillRule) -> bool {
        crate::page::read(
            self.page,
            "canvas_query",
            serde_json::json!([self.node, "isPointInPath", [x, y, rule.as_str()]]),
        )
    }

    fn is_point_in_stroke(&self, x: f32, y: f32) -> bool {
        crate::page::read(
            self.page,
            "canvas_query",
            serde_json::json!([self.node, "isPointInStroke", [x, y]]),
        )
    }

    fn get_image_data_signed(&self, sx: i32, sy: i32, sw: i32, sh: i32) -> Result<ImageData, &'static str> {
        let resp: serde_json::Value = crate::page::read(
            self.page,
            "canvas_query",
            serde_json::json!([self.node, "getImageData", [sx, sy, sw, sh]]),
        );
        let width = resp["width"].as_u64().unwrap_or(0) as u32;
        let height = resp["height"].as_u64().unwrap_or(0) as u32;
        let b64 = resp["data"].as_str().unwrap_or("");
        let data = decode_base64(b64);
        Ok(ImageData {
            width,
            height,
            data,
            color_space: "srgb",
        })
    }

    fn snapshot_image_source(&self, x: i32, y: i32, w: u32, h: u32) -> Option<(ImageData, bool)> {
        let img = self.get_image_data_signed(x, y, w as i32, h as i32).ok()?;
        Some((img, true))
    }

    fn reset(&mut self) {
        self.commands.push(serde_json::json!(["reset"]));
    }

    fn save(&mut self) {
        self.commands.push(serde_json::json!(["save"]));
    }

    fn restore(&mut self) {
        self.commands.push(serde_json::json!(["restore"]));
    }

    fn set_fill_color(&mut self, c: Color) {
        self.fill_style = c.to_css();
        self.commands.push(serde_json::json!(["fillColor", [c.r, c.g, c.b, c.a]]));
    }

    fn set_stroke_color(&mut self, c: Color) {
        self.stroke_style = c.to_css();
        self.commands.push(serde_json::json!(["strokeColor", [c.r, c.g, c.b, c.a]]));
    }

    fn set_line_width(&mut self, w: f32) {
        self.line_width = w;
        self.commands.push(serde_json::json!(["set", "lineWidth", w]));
    }

    fn set_line_dash(&mut self, d: &[f32]) {
        self.line_dash = d.to_vec();
        self.commands.push(serde_json::json!(["setLineDash", d]));
    }

    fn set_line_cap(&mut self, cap: LineCap) {
        self.line_cap = cap;
        self.commands.push(serde_json::json!(["set", "lineCap", cap.as_str()]));
    }

    fn set_line_join(&mut self, join: LineJoin) {
        self.line_join = join;
        self.commands.push(serde_json::json!(["set", "lineJoin", join.as_str()]));
    }

    fn set_global_alpha(&mut self, a: f32) {
        self.global_alpha = a;
        self.commands.push(serde_json::json!(["set", "globalAlpha", a]));
    }

    fn set_image_smoothing(&mut self, on: bool) {
        self.image_smoothing = on;
        self.commands.push(serde_json::json!(["set", "imageSmoothingEnabled", on]));
    }

    fn set_font(&mut self, f: &Font) {
        let css = f.to_css();
        self.font_css = css.clone();
        self.commands.push(serde_json::json!(["set", "font", css]));
    }

    fn translate(&mut self, x: f32, y: f32) {
        self.commands.push(serde_json::json!(["translate", x, y]));
    }

    fn scale(&mut self, x: f32, y: f32) {
        self.commands.push(serde_json::json!(["scale", x, y]));
    }

    fn rotate(&mut self, a: f32) {
        self.commands.push(serde_json::json!(["rotate", a]));
    }

    fn begin_path(&mut self) {
        self.commands.push(serde_json::json!(["beginPath"]));
    }

    fn close_path(&mut self) {
        self.commands.push(serde_json::json!(["closePath"]));
    }

    fn move_to(&mut self, x: f32, y: f32) {
        self.commands.push(serde_json::json!(["moveTo", x, y]));
    }

    fn line_to(&mut self, x: f32, y: f32) {
        self.commands.push(serde_json::json!(["lineTo", x, y]));
    }

    fn arc(&mut self, x: f32, y: f32, r: f32, s: f32, e: f32, ccw: bool) {
        self.commands.push(serde_json::json!(["arc", x, y, r, s, e, ccw]));
    }

    fn bezier_curve_to(&mut self, cp1x: f32, cp1y: f32, cp2x: f32, cp2y: f32, x: f32, y: f32) {
        self.commands.push(serde_json::json!(["bezierCurveTo", cp1x, cp1y, cp2x, cp2y, x, y]));
    }

    fn quadratic_curve_to(&mut self, cpx: f32, cpy: f32, x: f32, y: f32) {
        self.commands.push(serde_json::json!(["quadraticCurveTo", cpx, cpy, x, y]));
    }

    fn rect(&mut self, x: f32, y: f32, w: f32, h: f32) {
        self.commands.push(serde_json::json!(["rect", x, y, w, h]));
    }

    fn ellipse(&mut self, x: f32, y: f32, rx: f32, ry: f32) {
        self.commands.push(serde_json::json!(["ellipse", x, y, rx, ry, 0.0, 0.0, std::f32::consts::TAU, false]));
    }

    fn transform(&mut self, a: f32, b: f32, c: f32, d: f32, e: f32, f: f32) {
        self.commands.push(serde_json::json!(["transform", a, b, c, d, e, f]));
    }

    fn reset_transform(&mut self) {
        self.commands.push(serde_json::json!(["resetTransform"]));
    }

    fn set_miter_limit(&mut self, limit: f32) {
        self.miter_limit = limit;
        self.commands.push(serde_json::json!(["set", "miterLimit", limit]));
    }

    fn set_line_dash_offset(&mut self, offset: f32) {
        self.line_dash_offset = offset;
        self.commands.push(serde_json::json!(["set", "lineDashOffset", offset]));
    }

    fn set_text_align(&mut self, a: TextAlign) {
        self.text_align = a;
        self.commands.push(serde_json::json!(["set", "textAlign", a.as_str()]));
    }

    fn set_text_baseline(&mut self, b: TextBaseline) {
        self.text_baseline = b;
        self.commands.push(serde_json::json!(["set", "textBaseline", b.as_str()]));
    }

    fn fill(&mut self) {
        self.commands.push(serde_json::json!(["fill"]));
    }

    fn stroke(&mut self) {
        self.commands.push(serde_json::json!(["stroke"]));
    }

    fn clip(&mut self) {
        self.commands.push(serde_json::json!(["clip"]));
    }

    fn fill_rect(&mut self, x: f32, y: f32, w: f32, h: f32) {
        self.commands.push(serde_json::json!(["fillRect", x, y, w, h]));
    }

    fn stroke_rect(&mut self, x: f32, y: f32, w: f32, h: f32) {
        self.commands.push(serde_json::json!(["strokeRect", x, y, w, h]));
    }

    fn clear_rect(&mut self, x: f32, y: f32, w: f32, h: f32) {
        self.commands.push(serde_json::json!(["clearRect", x, y, w, h]));
    }

    fn fill_text(&mut self, text: &str, x: f32, y: f32) {
        self.commands.push(serde_json::json!(["fillText", text, x, y]));
    }

    fn stroke_text(&mut self, text: &str, x: f32, y: f32) {
        self.commands.push(serde_json::json!(["strokeText", text, x, y]));
    }

    fn put_image_data(&mut self, img: &Image, dx: f32, dy: f32) {
        self.flush();
        crate::page::write_pixels(
            self.page,
            serde_json::json!({
                "node": self.node,
                "command": ["putImageData", { "width": img.width, "height": img.height }, dx, dy],
            }),
            img.pixels.clone(),
        );
    }

    fn draw_image(&mut self, img: &Image, dx: f32, dy: f32, dw: f32, dh: f32) {
        self.flush();
        crate::page::write_pixels(
            self.page,
            serde_json::json!({
                "node": self.node,
                "command": ["drawImage", { "width": img.width, "height": img.height }, dx, dy, dw, dh],
            }),
            img.pixels.clone(),
        );
    }

    fn set_fill_style_css(&mut self, css: &str) {
        self.fill_style = css.to_string();
        self.commands.push(serde_json::json!(["set", "fillStyle", css]));
    }

    fn set_stroke_style_css(&mut self, css: &str) {
        self.stroke_style = css.to_string();
        self.commands.push(serde_json::json!(["set", "strokeStyle", css]));
    }

    fn set_font_css(&mut self, css: &str) {
        self.font_css = css.to_string();
        self.commands.push(serde_json::json!(["set", "font", css]));
    }

    fn set_shadow_color_css(&mut self, css: &str) {
        self.shadow_color_css = css.to_string();
        self.commands.push(serde_json::json!(["set", "shadowColor", css]));
    }

    fn set_filter(&mut self, css: &str) {
        self.filter_css = css.to_string();
        self.commands.push(serde_json::json!(["set", "filter", css]));
    }

    fn set_global_composite_operation(&mut self, op: CompositeOp) {
        self.composite = op;
        self.commands.push(serde_json::json!(["set", "globalCompositeOperation", op.as_str()]));
    }

    fn set_image_smoothing_quality(&mut self, q: SmoothingQuality) {
        self.smoothing_quality = q;
        self.commands.push(serde_json::json!(["set", "imageSmoothingQuality", q.as_str()]));
    }

    fn set_shadow(&mut self, s: &Shadow) {
        self.shadow_color_css = s.color.to_css();
        self.shadow_blur = s.blur;
        self.shadow_offset_x = s.offset_x;
        self.shadow_offset_y = s.offset_y;
        self.commands.push(serde_json::json!([
            "shadow",
            [s.color.r, s.color.g, s.color.b, s.color.a],
            s.blur,
            s.offset_x,
            s.offset_y
        ]));
    }

    fn set_direction(&mut self, d: Direction) {
        self.direction = d;
        self.commands.push(serde_json::json!(["set", "direction", d.as_str()]));
    }

    fn set_letter_spacing(&mut self, s: &str) {
        self.letter_spacing = s.to_string();
        self.commands.push(serde_json::json!(["set", "letterSpacing", s]));
    }

    fn set_word_spacing(&mut self, s: &str) {
        self.word_spacing = s.to_string();
        self.commands.push(serde_json::json!(["set", "wordSpacing", s]));
    }

    fn set_font_kerning(&mut self, k: FontKerning) {
        self.font_kerning = k;
        self.commands.push(serde_json::json!(["set", "fontKerning", k.as_str()]));
    }

    fn set_font_stretch(&mut self, s: FontStretch) {
        self.font_stretch = s;
        self.commands.push(serde_json::json!(["set", "fontStretch", s.as_str()]));
    }

    fn set_font_variant_caps(&mut self, c: FontVariantCaps) {
        self.font_variant_caps = c;
        self.commands.push(serde_json::json!(["set", "fontVariantCaps", c.as_str()]));
    }

    fn set_text_rendering(&mut self, r: TextRendering) {
        self.text_rendering = r;
        self.commands.push(serde_json::json!(["set", "textRendering", r.as_str()]));
    }

    fn set_lang(&mut self, l: &str) {
        self.lang = l.to_string();
        self.commands.push(serde_json::json!(["set", "lang", l]));
    }

    fn arc_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, r: f32) {
        self.commands.push(serde_json::json!(["arcTo", x1, y1, x2, y2, r]));
    }

    fn round_rect_radii(&mut self, x: f32, y: f32, w: f32, h: f32, radii: [f32; 4]) {
        self.commands.push(serde_json::json!(["roundRect", x, y, w, h, radii]));
    }

    fn ellipse_arc(
        &mut self,
        x: f32,
        y: f32,
        rx: f32,
        ry: f32,
        rotation: f32,
        start: f32,
        end: f32,
        ccw: bool,
    ) {
        self.commands.push(serde_json::json!(["ellipse", x, y, rx, ry, rotation, start, end, ccw]));
    }

    fn fill_with_rule(&mut self, rule: FillRule) {
        self.commands.push(serde_json::json!(["fill", rule.as_str()]));
    }

    fn clip_with_rule(&mut self, rule: FillRule) {
        self.commands.push(serde_json::json!(["clip", rule.as_str()]));
    }

    fn fill_text_constrained(&mut self, text: &str, x: f32, y: f32, max: f32) {
        self.commands.push(serde_json::json!(["fillText", text, x, y, max]));
    }

    fn stroke_text_constrained(&mut self, text: &str, x: f32, y: f32, max: f32) {
        self.commands.push(serde_json::json!(["strokeText", text, x, y, max]));
    }

    fn put_image_data_dirty(
        &mut self,
        data: &ImageData,
        dx: f32,
        dy: f32,
        _dirty_x: i32,
        _dirty_y: i32,
        _dirty_w: i32,
        _dirty_h: i32,
    ) {
        self.flush();
        crate::page::write_pixels(
            self.page,
            serde_json::json!({
                "node": self.node,
                "command": ["putImageData", { "width": data.width, "height": data.height }, dx, dy],
            }),
            data.data.clone(),
        );
    }
}

/// The WHATWG Canvas trait.
pub trait Canvas {
    fn text_baseline(&self) -> TextBaseline { TextBaseline::Alphabetic }
    fn line_cap(&self) -> LineCap { LineCap::Butt }
    fn line_join(&self) -> LineJoin { LineJoin::Miter }
    fn text_align(&self) -> TextAlign { TextAlign::Start }
    fn word_spacing_css(&self) -> String { "0px".into() }
    fn letter_spacing_css(&self) -> String { "0px".into() }
    fn font_kerning(&self) -> FontKerning { FontKerning::Auto }
    fn font_stretch(&self) -> FontStretch { FontStretch::Normal }
    fn font_variant_caps(&self) -> FontVariantCaps { FontVariantCaps::Normal }
    fn text_rendering(&self) -> TextRendering { TextRendering::Auto }
    fn direction(&self) -> Direction { Direction::Inherit }
    fn lang(&self) -> &str { "" }
    fn current_font_css(&self) -> String { "10px sans-serif".into() }
    fn fill_style_css(&self) -> String { "#000000".into() }
    fn stroke_style_css(&self) -> String { "#000000".into() }
    fn filter_css(&self) -> String { "none".into() }
    fn global_composite_operation(&self) -> CompositeOp { CompositeOp::SourceOver }
    fn image_smoothing_quality(&self) -> SmoothingQuality { SmoothingQuality::Low }
    fn shadow_color_css(&self) -> String { "rgba(0, 0, 0, 0)".into() }
    fn shadow_blur(&self) -> f32 { 0.0 }
    fn shadow_offset_x(&self) -> f32 { 0.0 }
    fn shadow_offset_y(&self) -> f32 { 0.0 }
    fn get_transform(&self) -> Matrix { Matrix::IDENTITY }
    fn get_line_dash(&self) -> Vec<f32> { Vec::new() }
    fn is_context_lost(&self) -> bool { false }
    fn context_attributes(&self) -> ContextAttributes { ContextAttributes::default() }

    fn measure_text(&mut self, _text: &str) -> TextMetrics { TextMetrics::default() }
    fn get_image_data_signed(&self, _sx: i32, _sy: i32, _sw: i32, _sh: i32) -> Result<ImageData, &'static str> {
        Ok(ImageData::default())
    }
    fn snapshot_image_source(&self, _x: i32, _y: i32, _w: u32, _h: u32) -> Option<(ImageData, bool)> {
        None
    }
    fn to_data_url_checked(&self, _mime: &str, _quality: Option<f32>) -> Result<String, &'static str> {
        Ok("data:,".to_string())
    }
    fn to_blob_checked(&self, _mime: &str, _quality: Option<f32>) -> Result<Option<Vec<u8>>, &'static str> {
        Ok(None)
    }
    fn is_point_in_path(&self, _x: f32, _y: f32, _rule: FillRule) -> bool { false }
    fn is_point_in_stroke(&self, _x: f32, _y: f32) -> bool { false }
    fn is_point_in_path2d(&self, _path: &Path2D, _x: f32, _y: f32, _rule: FillRule) -> bool { false }
    fn is_point_in_stroke2d(&self, _path: &Path2D, _x: f32, _y: f32) -> bool { false }

    fn reset(&mut self) {}
    fn save(&mut self) {}
    fn restore(&mut self) {}
    fn set_fill_color(&mut self, _c: Color) {}
    fn set_stroke_color(&mut self, _c: Color) {}
    fn set_line_width(&mut self, _w: f32) {}
    fn set_line_dash(&mut self, _d: &[f32]) {}
    fn set_line_cap(&mut self, _cap: LineCap) {}
    fn set_line_join(&mut self, _join: LineJoin) {}
    fn set_global_alpha(&mut self, _a: f32) {}
    fn set_image_smoothing(&mut self, _on: bool) {}
    fn set_font(&mut self, _f: &Font) {}
    fn translate(&mut self, _x: f32, _y: f32) {}
    fn scale(&mut self, _x: f32, _y: f32) {}
    fn rotate(&mut self, _a: f32) {}
    fn begin_path(&mut self) {}
    fn close_path(&mut self) {}
    fn move_to(&mut self, _x: f32, _y: f32) {}
    fn line_to(&mut self, _x: f32, _y: f32) {}
    fn arc(&mut self, _x: f32, _y: f32, _r: f32, _s: f32, _e: f32, _ccw: bool) {}
    fn bezier_curve_to(&mut self, _a: f32, _b: f32, _cc: f32, _d: f32, _e: f32, _f: f32) {}
    fn quadratic_curve_to(&mut self, _a: f32, _b: f32, _cc: f32, _d: f32) {}
    fn rect(&mut self, _x: f32, _y: f32, _w: f32, _h: f32) {}
    fn ellipse(&mut self, _x: f32, _y: f32, _rx: f32, _ry: f32) {}
    fn transform(&mut self, _a: f32, _b: f32, _cc: f32, _d: f32, _e: f32, _f: f32) {}
    fn reset_transform(&mut self) {}
    fn set_miter_limit(&mut self, _limit: f32) {}
    fn set_line_dash_offset(&mut self, _offset: f32) {}
    fn set_text_align(&mut self, _a: TextAlign) {}
    fn set_text_baseline(&mut self, _b: TextBaseline) {}
    fn fill(&mut self) {}
    fn stroke(&mut self) {}
    fn clip(&mut self) {}
    fn fill_rect(&mut self, _x: f32, _y: f32, _w: f32, _h: f32) {}
    fn stroke_rect(&mut self, _x: f32, _y: f32, _w: f32, _h: f32) {}
    fn clear_rect(&mut self, _x: f32, _y: f32, _w: f32, _h: f32) {}
    fn fill_text(&mut self, _text: &str, _x: f32, _y: f32) {}
    fn stroke_text(&mut self, _text: &str, _x: f32, _y: f32) {}
    fn put_image_data(&mut self, _img: &Image, _dx: f32, _dy: f32) {}
    fn draw_image(&mut self, _img: &Image, _dx: f32, _dy: f32, _dw: f32, _dh: f32) {}
    fn set_fill_style_css(&mut self, _css: &str) {}
    fn set_stroke_style_css(&mut self, _css: &str) {}
    fn set_font_css(&mut self, _css: &str) {}
    fn set_shadow_color_css(&mut self, _css: &str) {}
    fn set_filter(&mut self, _css: &str) {}
    fn set_global_composite_operation(&mut self, _op: CompositeOp) {}
    fn set_image_smoothing_quality(&mut self, _q: SmoothingQuality) {}
    fn set_shadow(&mut self, _s: &Shadow) {}
    fn set_direction(&mut self, _d: Direction) {}
    fn set_letter_spacing(&mut self, _s: &str) {}
    fn set_word_spacing(&mut self, _s: &str) {}
    fn set_font_kerning(&mut self, _k: FontKerning) {}
    fn set_font_stretch(&mut self, _s: FontStretch) {}
    fn set_font_variant_caps(&mut self, _c: FontVariantCaps) {}
    fn set_text_rendering(&mut self, _r: TextRendering) {}
    fn set_lang(&mut self, _l: &str) {}
    fn arc_to(&mut self, _x1: f32, _y1: f32, _x2: f32, _y2: f32, _r: f32) {}
    fn round_rect_radii(&mut self, _x: f32, _y: f32, _w: f32, _h: f32, _radii: [f32; 4]) {}
    fn ellipse_arc(&mut self, _x: f32, _y: f32, _rx: f32, _ry: f32, _rotation: f32, _start: f32, _end: f32, _ccw: bool) {}
    fn fill_with_rule(&mut self, _rule: FillRule) {}
    fn clip_with_rule(&mut self, _rule: FillRule) {}
    fn fill_text_constrained(&mut self, _text: &str, _x: f32, _y: f32, _max: f32) {}
    fn stroke_text_constrained(&mut self, _text: &str, _x: f32, _y: f32, _max: f32) {}
    fn set_fill_paint(&mut self, _paint: &Paint) {}
    fn set_stroke_paint(&mut self, _paint: &Paint) {}
    fn put_image_data_dirty(
        &mut self,
        _data: &ImageData,
        _dx: f32,
        _dy: f32,
        _dirty_x: i32,
        _dirty_y: i32,
        _dirty_w: i32,
        _dirty_h: i32,
    ) {}
    fn draw_focus_if_needed(&mut self, _focused: bool) {}
    fn fill_path(&mut self, _path: &Path2D, _rule: FillRule) {}
    fn stroke_path(&mut self, _path: &Path2D) {}
    fn clip_path(&mut self, _path: &Path2D, _rule: FillRule) {}
    fn append_path(&mut self, _path: &Path2D) {}
}
