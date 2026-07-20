//! Minimal ProPresenter presentation protobuf model.
//!
//! Field numbers and wire types are pinned to
//! greyshirtguy/ProPresenter7-Proto commit
//! `1b63dda196eb7e079721a8a4a7e7773520cb5ad2`. The corresponding schema and
//! license are vendored under `vendor/propresenter/`.

use prost::Message;

#[derive(Clone, PartialEq, Message)]
pub(crate) struct Presentation {
    #[prost(message, optional, tag = "1")]
    pub application_info: Option<ApplicationInfo>,
    #[prost(message, optional, tag = "2")]
    pub uuid: Option<RvUuid>,
    #[prost(string, tag = "3")]
    pub name: String,
    #[prost(string, tag = "6")]
    pub category: String,
    #[prost(string, tag = "7")]
    pub notes: String,
    #[prost(message, optional, tag = "10")]
    pub selected_arrangement: Option<RvUuid>,
    #[prost(message, repeated, tag = "11")]
    pub arrangements: Vec<Arrangement>,
    #[prost(message, repeated, tag = "12")]
    pub cue_groups: Vec<CueGroup>,
    #[prost(message, repeated, tag = "13")]
    pub cues: Vec<Cue>,
    #[prost(message, optional, tag = "14")]
    pub ccli: Option<Ccli>,
    #[prost(int32, tag = "19")]
    pub content_destination: i32,
    #[prost(string, tag = "22")]
    pub music_key: String,
    #[prost(message, optional, tag = "23")]
    pub music: Option<Music>,
}

#[derive(Clone, PartialEq, Message)]
pub(crate) struct ApplicationInfo {
    #[prost(int32, tag = "1")]
    pub platform: i32,
    #[prost(message, optional, tag = "2")]
    pub platform_version: Option<Version>,
    #[prost(int32, tag = "3")]
    pub application: i32,
    #[prost(message, optional, tag = "4")]
    pub application_version: Option<Version>,
}

#[derive(Clone, PartialEq, Message)]
pub(crate) struct Version {
    #[prost(uint32, tag = "1")]
    pub major_version: u32,
    #[prost(uint32, tag = "2")]
    pub minor_version: u32,
    #[prost(uint32, tag = "3")]
    pub patch_version: u32,
    #[prost(string, tag = "4")]
    pub build: String,
}

#[derive(Clone, PartialEq, Message)]
pub(crate) struct Ccli {
    #[prost(string, tag = "1")]
    pub author: String,
    #[prost(string, tag = "2")]
    pub artist_credits: String,
    #[prost(string, tag = "3")]
    pub song_title: String,
    #[prost(string, tag = "4")]
    pub publisher: String,
    #[prost(uint32, tag = "5")]
    pub copyright_year: u32,
    #[prost(uint32, tag = "6")]
    pub song_number: u32,
    #[prost(bool, tag = "7")]
    pub display: bool,
    #[prost(string, tag = "8")]
    pub album: String,
    #[prost(bytes = "vec", tag = "9")]
    pub artwork: Vec<u8>,
}

#[derive(Clone, PartialEq, Message)]
pub(crate) struct Arrangement {
    #[prost(message, optional, tag = "1")]
    pub uuid: Option<RvUuid>,
    #[prost(string, tag = "2")]
    pub name: String,
    #[prost(message, repeated, tag = "3")]
    pub group_identifiers: Vec<RvUuid>,
}

#[derive(Clone, PartialEq, Message)]
pub(crate) struct CueGroup {
    #[prost(message, optional, tag = "1")]
    pub group: Option<Group>,
    #[prost(message, repeated, tag = "2")]
    pub cue_identifiers: Vec<RvUuid>,
}

#[derive(Clone, PartialEq, Message)]
pub(crate) struct Group {
    #[prost(message, optional, tag = "1")]
    pub uuid: Option<RvUuid>,
    #[prost(string, tag = "2")]
    pub name: String,
    #[prost(message, optional, tag = "3")]
    pub color: Option<Color>,
    #[prost(message, optional, tag = "5")]
    pub application_group_identifier: Option<RvUuid>,
    #[prost(string, tag = "6")]
    pub application_group_name: String,
}

#[derive(Clone, PartialEq, Message)]
pub(crate) struct Cue {
    #[prost(message, optional, tag = "1")]
    pub uuid: Option<RvUuid>,
    #[prost(string, tag = "2")]
    pub name: String,
    #[prost(message, repeated, tag = "10")]
    pub actions: Vec<Action>,
    #[prost(bool, tag = "12")]
    pub is_enabled: bool,
}

#[derive(Clone, PartialEq, Message)]
pub(crate) struct Action {
    #[prost(message, optional, tag = "1")]
    pub uuid: Option<RvUuid>,
    #[prost(string, tag = "2")]
    pub name: String,
    #[prost(bool, tag = "6")]
    pub is_enabled: bool,
    #[prost(int32, tag = "9")]
    pub action_type: i32,
    #[prost(message, optional, tag = "23")]
    pub slide: Option<SlideType>,
}

#[derive(Clone, PartialEq, Message)]
pub(crate) struct SlideType {
    #[prost(message, optional, tag = "2")]
    pub presentation: Option<PresentationSlide>,
}

#[derive(Clone, PartialEq, Message)]
pub(crate) struct PresentationSlide {
    #[prost(message, optional, tag = "1")]
    pub base_slide: Option<Slide>,
}

#[derive(Clone, PartialEq, Message)]
pub(crate) struct Slide {
    #[prost(message, repeated, tag = "1")]
    pub elements: Vec<SlideElement>,
    #[prost(message, optional, tag = "6")]
    pub size: Option<Size>,
    #[prost(message, optional, tag = "7")]
    pub uuid: Option<RvUuid>,
}

#[derive(Clone, PartialEq, Message)]
pub(crate) struct SlideElement {
    #[prost(message, optional, tag = "1")]
    pub element: Option<GraphicsElement>,
    #[prost(uint32, tag = "4")]
    pub info: u32,
}

#[derive(Clone, PartialEq, Message)]
pub(crate) struct GraphicsElement {
    #[prost(message, optional, tag = "1")]
    pub uuid: Option<RvUuid>,
    #[prost(string, tag = "2")]
    pub name: String,
    #[prost(message, optional, tag = "3")]
    pub bounds: Option<Rect>,
    #[prost(double, tag = "5")]
    pub opacity: f64,
    #[prost(message, optional, tag = "13")]
    pub text: Option<GraphicsText>,
    #[prost(bool, tag = "16")]
    pub hidden: bool,
}

#[derive(Clone, PartialEq, Message)]
pub(crate) struct GraphicsText {
    #[prost(message, optional, tag = "3")]
    pub attributes: Option<TextAttributes>,
    #[prost(bytes = "vec", tag = "5")]
    pub rtf_data: Vec<u8>,
    #[prost(int32, tag = "6")]
    pub vertical_alignment: i32,
    #[prost(int32, tag = "7")]
    pub scale_behavior: i32,
    #[prost(message, optional, tag = "8")]
    pub margins: Option<EdgeInsets>,
    #[prost(message, optional, tag = "12")]
    pub chord_pro: Option<ChordProSettings>,
}

#[derive(Clone, PartialEq, Message)]
pub(crate) struct ChordProSettings {
    #[prost(bool, tag = "1")]
    pub enabled: bool,
    #[prost(int32, tag = "2")]
    pub notation: i32,
    #[prost(message, optional, tag = "3")]
    pub color: Option<Color>,
}

#[derive(Clone, PartialEq, Message)]
pub(crate) struct TextAttributes {
    #[prost(message, optional, tag = "1")]
    pub font: Option<Font>,
    #[prost(message, optional, tag = "3")]
    pub text_solid_fill: Option<Color>,
    #[prost(message, optional, tag = "6")]
    pub paragraph_style: Option<ParagraphStyle>,
    #[prost(message, repeated, tag = "13")]
    pub custom_attributes: Vec<CustomAttribute>,
}

#[derive(Clone, PartialEq, Message)]
pub(crate) struct CustomAttribute {
    #[prost(message, optional, tag = "1")]
    pub range: Option<IntRange>,
    #[prost(string, tag = "7")]
    pub chord: String,
}

#[derive(Clone, PartialEq, Message)]
pub(crate) struct Font {
    #[prost(string, tag = "1")]
    pub name: String,
    #[prost(double, tag = "2")]
    pub size: f64,
    #[prost(bool, tag = "4")]
    pub italic: bool,
    #[prost(bool, tag = "8")]
    pub bold: bool,
    #[prost(string, tag = "9")]
    pub family: String,
    #[prost(string, tag = "10")]
    pub face: String,
}

#[derive(Clone, PartialEq, Message)]
pub(crate) struct ParagraphStyle {
    #[prost(int32, tag = "1")]
    pub alignment: i32,
    #[prost(double, tag = "5")]
    pub line_height_multiple: f64,
}

#[derive(Clone, PartialEq, Message)]
pub(crate) struct Music {
    #[prost(string, tag = "1")]
    pub original_music_key: String,
    #[prost(string, tag = "2")]
    pub user_music_key: String,
    #[prost(message, optional, tag = "3")]
    pub original: Option<MusicKeyScale>,
    #[prost(message, optional, tag = "4")]
    pub user: Option<MusicKeyScale>,
}

#[derive(Clone, PartialEq, Message)]
pub(crate) struct MusicKeyScale {
    #[prost(int32, tag = "1")]
    pub music_key: i32,
    #[prost(int32, tag = "2")]
    pub music_scale: i32,
}

#[derive(Clone, PartialEq, Message)]
pub(crate) struct RvUuid {
    #[prost(string, tag = "1")]
    pub string: String,
}

#[derive(Clone, PartialEq, Message)]
pub(crate) struct IntRange {
    #[prost(int32, tag = "1")]
    pub start: i32,
    #[prost(int32, tag = "2")]
    pub end: i32,
}

#[derive(Clone, PartialEq, Message)]
pub(crate) struct Color {
    #[prost(float, tag = "1")]
    pub red: f32,
    #[prost(float, tag = "2")]
    pub green: f32,
    #[prost(float, tag = "3")]
    pub blue: f32,
    #[prost(float, tag = "4")]
    pub alpha: f32,
}

#[derive(Clone, PartialEq, Message)]
pub(crate) struct Point {
    #[prost(double, tag = "1")]
    pub x: f64,
    #[prost(double, tag = "2")]
    pub y: f64,
}

#[derive(Clone, PartialEq, Message)]
pub(crate) struct Size {
    #[prost(double, tag = "1")]
    pub width: f64,
    #[prost(double, tag = "2")]
    pub height: f64,
}

#[derive(Clone, PartialEq, Message)]
pub(crate) struct Rect {
    #[prost(message, optional, tag = "1")]
    pub origin: Option<Point>,
    #[prost(message, optional, tag = "2")]
    pub size: Option<Size>,
}

#[derive(Clone, PartialEq, Message)]
pub(crate) struct EdgeInsets {
    #[prost(double, tag = "1")]
    pub left: f64,
    #[prost(double, tag = "2")]
    pub right: f64,
    #[prost(double, tag = "3")]
    pub top: f64,
    #[prost(double, tag = "4")]
    pub bottom: f64,
}
