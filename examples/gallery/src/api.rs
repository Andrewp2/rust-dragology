//! Shared model and vector drawing contract for the native demo ports.
pub use dragology::{Affine, DragSpec, Point, Rect, Vec2};
pub use kurbo::BezPath;
use std::time::Duration;

pub type Color = [u8; 4];
pub const INK: Color = [38, 43, 51, 255];
pub const MUTED: Color = [150, 158, 170, 255];
pub const PAPER: Color = [250, 249, 246, 255];
pub const BLUE: Color = [62, 116, 190, 255];
pub const RED: Color = [216, 83, 72, 255];
pub const GREEN: Color = [59, 145, 119, 255];
pub const GOLD: Color = [225, 167, 58, 255];
pub const PALETTE: [Color; 8] = [
    BLUE,
    RED,
    GREEN,
    GOLD,
    [144, 99, 183, 255],
    [56, 157, 176, 255],
    [214, 122, 167, 255],
    [139, 110, 82, 255],
];

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Modifiers {
    pub alt: bool,
    pub shift: bool,
    pub control: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Shape {
    Rect {
        bounds: Rect,
        radius: f64,
        fill: Color,
        stroke: Option<(Color, f64)>,
    },
    Circle {
        center: Point,
        radius: f64,
        fill: Color,
        stroke: Option<(Color, f64)>,
    },
    Path {
        path: BezPath,
        fill: Option<Color>,
        stroke: Option<(Color, f64)>,
    },
    Text {
        origin: Point,
        text: String,
        size: f64,
        color: Color,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub enum Hit {
    Rect(Rect),
    Circle(Point, f64),
    Path(BezPath, f64),
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TextAnchor {
    #[default]
    Start,
    Middle,
    End,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TextBaseline {
    #[default]
    Top,
    Alphabetic,
    Middle,
    Central,
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum FontFamily {
    #[default]
    Sans,
    Serif,
    Mono,
    Named(String),
}
#[derive(Clone, Debug, PartialEq)]
pub struct TextStyle {
    pub anchor: TextAnchor,
    pub baseline: TextBaseline,
    pub family: FontFamily,
    pub weight: u16,
    pub italic: bool,
    pub letter_spacing: f64,
    pub strikethrough: bool,
}
impl Default for TextStyle {
    fn default() -> Self {
        Self {
            anchor: TextAnchor::Start,
            baseline: TextBaseline::Top,
            family: FontFamily::Sans,
            weight: 400,
            italic: false,
            letter_spacing: 0.,
            strikethrough: false,
        }
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LineCap {
    #[default]
    Butt,
    Round,
    Square,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LineJoin {
    #[default]
    Miter,
    Round,
    Bevel,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FillRule {
    #[default]
    NonZero,
    EvenOdd,
}
#[derive(Clone, Debug, PartialEq)]
pub struct StrokeStyle {
    pub cap: LineCap,
    pub join: LineJoin,
    pub miter_limit: f64,
    pub dash: Vec<f64>,
    pub dash_offset: f64,
    pub non_scaling: bool,
}
impl Default for StrokeStyle {
    fn default() -> Self {
        Self {
            cap: LineCap::Butt,
            join: LineJoin::Miter,
            miter_limit: 4.,
            dash: Vec::new(),
            dash_offset: 0.,
            non_scaling: false,
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct Shadow {
    pub offset: Vec2,
    pub blur: f64,
    pub color: Color,
}
#[derive(Clone, Debug, PartialEq)]
pub enum Clip {
    Rect(Rect),
    Path(BezPath),
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Cursor {
    #[default]
    Default,
    Grab,
    Grabbing,
    Move,
    Pointer,
    NotAllowed,
    ResizeHorizontal,
    ResizeVertical,
    ResizeDiagonalNwSe,
    ResizeDiagonalNeSw,
    Crosshair,
    Text,
}

/// How an object appears at `emerge_from` when it is absent from a blended scene.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum EmergeMode {
    /// Morph direct rectangle bounds when available, otherwise grow from zero.
    #[default]
    Auto,
    /// Grow and fade in from the origin, regardless of rectangle geometry.
    Scale,
    /// Appear at full opacity at the origin, morphing direct rectangle bounds.
    Clone,
}

/// Retained SVG transform operations, in source (right-to-left application) order.
/// Keeping the operations preserves pivoted trajectories during interpolation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TransformComponent {
    Translate(Vec2),
    Rotate { radians: f64, center: Point },
    Scale(Vec2),
}
impl TransformComponent {
    pub fn affine(self) -> Affine {
        match self {
            Self::Translate(v) => Affine::translate(v),
            Self::Rotate { radians, center } => {
                Affine::translate(center.to_vec2())
                    * Affine::rotate(radians)
                    * Affine::translate(-center.to_vec2())
            }
            Self::Scale(v) => Affine::scale_non_uniform(v.x, v.y),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Object {
    pub id: String,
    pub transform: Affine,
    pub transform_components: Option<Vec<TransformComponent>>,
    pub hit: Option<Hit>,
    pub shapes: Vec<Shape>,
    pub opacity: f64,
    /// Grouping only. Transforms remain local-to-scene, including for children.
    pub parent: Option<String>,
    pub interpolate: bool,
    pub emerge_from: Option<String>,
    pub emerge_mode: EmergeMode,
    /// Whether the first rectangle/text are direct SVG children for emergence.
    /// Disable for source groups which wrap their rectangle in an inner group.
    pub emerge_bounds: bool,
    pub text_style: TextStyle,
    pub text_outline: Option<(Color, f64)>,
    pub stroke_style: StrokeStyle,
    pub fill_rule: FillRule,
    pub shadow: Option<Shadow>,
    /// Local-coordinate clipping for this object's shapes.
    pub clip: Option<Clip>,
    pub tooltip: Option<String>,
    pub hit_when_transparent: bool,
    pub transition: bool,
    pub click_handler: bool,
    pub cursor: Cursor,
    /// Gaussian standard deviation in local scene units.
    pub blur: f64,
}
impl Object {
    pub fn new(
        id: impl Into<String>,
        transform: Affine,
        hit: Option<Hit>,
        shapes: Vec<Shape>,
    ) -> Self {
        Self {
            id: id.into(),
            transform,
            transform_components: None,
            hit,
            shapes,
            opacity: 1.,
            parent: None,
            interpolate: true,
            emerge_from: None,
            emerge_mode: EmergeMode::Auto,
            emerge_bounds: true,
            text_style: TextStyle::default(),
            text_outline: None,
            stroke_style: StrokeStyle::default(),
            fill_rule: FillRule::NonZero,
            shadow: None,
            clip: None,
            tooltip: None,
            hit_when_transparent: false,
            transition: true,
            click_handler: false,
            cursor: Cursor::Default,
            blur: 0.,
        }
    }
    pub fn transforms(mut self, components: Vec<TransformComponent>) -> Self {
        self.transform = components
            .iter()
            .fold(Affine::IDENTITY, |m, c| m * c.affine());
        self.transform_components = Some(components);
        self
    }
    /// Prepend a scene-space translation while preserving the source operations.
    pub fn prepend_translation(&mut self, offset: Vec2) {
        self.transform = Affine::translate(offset) * self.transform;
        if let Some(components) = &mut self.transform_components {
            components.insert(0, TransformComponent::Translate(offset));
        }
    }
    /// A centered circular object with a matching hit target.
    pub fn circle(
        id: impl Into<String>,
        center: impl Into<Point>,
        radius: f64,
        color: Color,
    ) -> Self {
        Self::new(
            id,
            Affine::translate(center.into().to_vec2()),
            Some(Hit::Circle(Point::ORIGIN, radius)),
            vec![Shape::Circle {
                center: Point::ORIGIN,
                radius,
                fill: color,
                stroke: None,
            }],
        )
    }
    /// A rectangular object whose local origin is its upper-left corner.
    pub fn rect(id: impl Into<String>, bounds: Rect, radius: f64, color: Color) -> Self {
        let local = Rect::new(0., 0., bounds.width(), bounds.height());
        Self::new(
            id,
            Affine::translate(bounds.origin().to_vec2()),
            Some(Hit::Rect(local)),
            vec![Shape::Rect {
                bounds: local,
                radius,
                fill: color,
                stroke: None,
            }],
        )
    }
    pub fn text(
        id: impl Into<String>,
        origin: impl Into<Point>,
        text: impl Into<String>,
        size: f64,
        color: Color,
    ) -> Self {
        Self::new(
            id,
            Affine::translate(origin.into().to_vec2()),
            None,
            vec![Shape::Text {
                origin: Point::ORIGIN,
                text: text.into(),
                size,
                color,
            }],
        )
    }
    pub fn path(id: impl Into<String>, path: BezPath, color: Color, width: f64) -> Self {
        Self::new(
            id,
            Affine::IDENTITY,
            None,
            vec![Shape::Path {
                path,
                fill: None,
                stroke: Some((color, width)),
            }],
        )
    }
    pub fn line(
        id: impl Into<String>,
        a: impl Into<Point>,
        b: impl Into<Point>,
        color: Color,
        width: f64,
    ) -> Self {
        let mut path = BezPath::new();
        path.move_to(a);
        path.line_to(b);
        Self::path(id, path, color, width)
    }
    pub fn label(
        mut self,
        text: impl Into<String>,
        origin: impl Into<Point>,
        size: f64,
        color: Color,
    ) -> Self {
        self.shapes.push(Shape::Text {
            origin: origin.into(),
            text: text.into(),
            size,
            color,
        });
        self
    }
    pub fn decorative(mut self) -> Self {
        self.hit = None;
        self
    }
    pub fn text_style(mut self, style: TextStyle) -> Self {
        self.text_style = style;
        self
    }
    pub fn text_outline(mut self, color: Color, width: f64) -> Self {
        self.text_outline = Some((color, width));
        self
    }
    pub fn text_anchor(mut self, anchor: TextAnchor) -> Self {
        self.text_style.anchor = anchor;
        self
    }
    pub fn text_baseline(mut self, baseline: TextBaseline) -> Self {
        self.text_style.baseline = baseline;
        self
    }
    pub fn font_family(mut self, family: FontFamily) -> Self {
        self.text_style.family = family;
        self
    }
    pub fn font_weight(mut self, weight: u16) -> Self {
        self.text_style.weight = weight;
        self
    }
    pub fn letter_spacing(mut self, spacing: f64) -> Self {
        self.text_style.letter_spacing = spacing;
        self
    }
    pub fn strikethrough(mut self) -> Self {
        self.text_style.strikethrough = true;
        self
    }
    pub fn clickable(mut self) -> Self {
        self.click_handler = true;
        self
    }
    pub fn italic(mut self) -> Self {
        self.text_style.italic = true;
        self
    }
    pub fn stroke_style(mut self, style: StrokeStyle) -> Self {
        self.stroke_style = style;
        self
    }
    pub fn line_cap(mut self, cap: LineCap) -> Self {
        self.stroke_style.cap = cap;
        self
    }
    pub fn line_join(mut self, join: LineJoin) -> Self {
        self.stroke_style.join = join;
        self
    }
    pub fn dash(mut self, dash: impl Into<Vec<f64>>) -> Self {
        self.stroke_style.dash = dash.into();
        self
    }
    pub fn non_scaling_stroke(mut self) -> Self {
        self.stroke_style.non_scaling = true;
        self
    }
    pub fn fill_rule(mut self, rule: FillRule) -> Self {
        self.fill_rule = rule;
        self
    }
    pub fn shadow(mut self, shadow: Shadow) -> Self {
        self.shadow = Some(shadow);
        self
    }
    pub fn clip(mut self, clip: Clip) -> Self {
        self.clip = Some(clip);
        self
    }
    pub fn tooltip(mut self, tooltip: impl Into<String>) -> Self {
        self.tooltip = Some(tooltip.into());
        self
    }
    pub fn hit_when_transparent(mut self) -> Self {
        self.hit_when_transparent = true;
        self
    }
    pub fn transition(mut self, enabled: bool) -> Self {
        self.transition = enabled;
        self
    }
    pub fn cursor(mut self, cursor: Cursor) -> Self {
        self.cursor = cursor;
        self
    }
    pub fn blur(mut self, radius: f64) -> Self {
        self.blur = radius.max(0.);
        self
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Presentation {
    #[default]
    Fit,
    Scroll,
}
#[derive(Clone, Debug, PartialEq)]
pub struct ControlPlacement {
    pub id: &'static str,
    pub bounds: Rect,
    pub parent: Option<String>,
    pub style: ControlStyle,
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ControlStyle {
    pub select: bool,
    pub foreground: Option<Color>,
    pub background: Option<Color>,
    pub border: Option<Color>,
    pub font_size: Option<f64>,
    pub font_family: Option<FontFamily>,
    pub placeholder: Option<String>,
}
impl ControlPlacement {
    pub fn select(&mut self) -> &mut Self {
        self.style.select = true;
        self
    }
    pub fn foreground(&mut self, color: Color) -> &mut Self {
        self.style.foreground = Some(color);
        self
    }
    pub fn background(&mut self, color: Color) -> &mut Self {
        self.style.background = Some(color);
        self
    }
    pub fn border(&mut self, color: Color) -> &mut Self {
        self.style.border = Some(color);
        self
    }
    pub fn font_size(&mut self, size: f64) -> &mut Self {
        self.style.font_size = Some(size);
        self
    }
    pub fn font_family(&mut self, family: FontFamily) -> &mut Self {
        self.style.font_family = Some(family);
        self
    }
    pub fn placeholder(&mut self, text: impl Into<String>) -> &mut Self {
        self.style.placeholder = Some(text.into());
        self
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Scene {
    pub width: f64,
    pub height: f64,
    pub objects: Vec<Object>,
    pub presentation: Presentation,
    pub inline_controls: Vec<ControlPlacement>,
}
impl Scene {
    pub fn new(width: f64, height: f64) -> Self {
        Self {
            width,
            height,
            objects: Vec::new(),
            presentation: Presentation::Fit,
            inline_controls: Vec::new(),
        }
    }
    pub fn push(&mut self, object: Object) {
        self.objects.push(object);
    }
    pub fn object(&self, id: &str) -> Option<&Object> {
        self.objects.iter().find(|o| o.id == id)
    }
    pub fn scrolling(mut self) -> Self {
        self.presentation = Presentation::Scroll;
        self
    }
    pub fn inline_control(&mut self, id: &'static str, bounds: Rect) -> &mut ControlPlacement {
        self.inline_controls.push(ControlPlacement {
            id,
            bounds,
            parent: None,
            style: ControlStyle::default(),
        });
        self.inline_controls.last_mut().unwrap()
    }
    pub fn inline_control_for(
        &mut self,
        id: &'static str,
        parent: impl Into<String>,
        bounds: Rect,
    ) -> &mut ControlPlacement {
        self.inline_controls.push(ControlPlacement {
            id,
            bounds,
            parent: Some(parent.into()),
            style: ControlStyle::default(),
        });
        self.inline_controls.last_mut().unwrap()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Bool(bool),
    Number(f64),
    Index(usize),
    Text(String),
    Trigger,
}
#[derive(Clone, Debug)]
pub enum Control {
    ActionButton {
        id: &'static str,
        label: String,
        enabled: bool,
        selected: bool,
    },
    Button {
        id: &'static str,
        label: String,
    },
    Toggle {
        id: &'static str,
        label: String,
        value: bool,
    },
    Choice {
        id: &'static str,
        label: String,
        options: Vec<String>,
        selected: usize,
    },
    Slider {
        id: &'static str,
        label: String,
        value: f64,
        min: f64,
        max: f64,
        step: f64,
    },
    Code {
        id: &'static str,
        label: String,
        content: String,
    },
    Text {
        id: &'static str,
        label: String,
        value: String,
    },
}
impl Control {
    pub fn id(&self) -> &'static str {
        match self {
            Self::ActionButton { id, .. }
            | Self::Button { id, .. }
            | Self::Toggle { id, .. }
            | Self::Choice { id, .. }
            | Self::Slider { id, .. }
            | Self::Text { id, .. }
            | Self::Code { id, .. } => id,
        }
    }
    pub fn action_button(
        id: &'static str,
        label: impl Into<String>,
        enabled: bool,
        selected: bool,
    ) -> Self {
        Self::ActionButton {
            id,
            label: label.into(),
            enabled,
            selected,
        }
    }
    pub fn button(id: &'static str, label: impl Into<String>) -> Self {
        Self::Button {
            id,
            label: label.into(),
        }
    }
    pub fn toggle(id: &'static str, label: impl Into<String>, value: bool) -> Self {
        Self::Toggle {
            id,
            label: label.into(),
            value,
        }
    }
}

/// Model rules use the Rust Dragology engine. Optional methods provide the
/// upstream demo's clicks, settings and keyboard behavior.
pub trait Example: 'static {
    type State: Clone + PartialEq + 'static;
    fn initial(&self) -> Self::State;
    fn reset(&self, _state: &Self::State) -> Self::State {
        self.initial()
    }
    fn hover_transition(&self, _state: &Self::State) -> Option<dragology::Transition> {
        None
    }
    fn scene(&self, state: &Self::State) -> Scene;
    /// Optional feedback that belongs to the gesture, not the committed model.
    fn scene_dragging(&self, state: &Self::State, _id: Option<&str>) -> Scene {
        self.scene(state)
    }
    fn scene_interaction(
        &self,
        state: &Self::State,
        dragged: Option<&str>,
        _hovered: Option<&str>,
    ) -> Scene {
        self.scene_dragging(state, dragged)
    }
    fn begin_drag(
        &self,
        _state: &Self::State,
        _id: &str,
        _modifiers: Modifiers,
    ) -> Option<(Self::State, String)> {
        None
    }
    fn drop_transition(&self, _state: &Self::State) -> Option<dragology::Transition> {
        Some(dragology::Transition::default())
    }
    fn action_transition(&self, _state: &Self::State, _id: &str) -> Option<dragology::Transition> {
        Some(dragology::Transition::default())
    }
    /// Override the pointer's local grab position for center-anchored gestures.
    fn anchor(&self, _state: &Self::State, _id: &str, local: Point) -> Point {
        local
    }
    fn drag(
        &self,
        state: &Self::State,
        id: &str,
        modifiers: Modifiers,
    ) -> Option<DragSpec<'static, Self::State>>;
    fn locate(&self, state: &Self::State, id: &str) -> Option<Affine> {
        self.scene(state).object(id).map(|o| o.transform)
    }
    fn click(
        &self,
        _state: &Self::State,
        _id: &str,
        _point: Point,
        _modifiers: Modifiers,
        _count: usize,
    ) -> Option<Self::State> {
        None
    }
    fn controls(&self, _state: &Self::State) -> Vec<Control> {
        Vec::new()
    }
    fn action(&self, _state: &Self::State, _id: &str, _value: Value) -> Option<Self::State> {
        None
    }
    fn key(&self, _state: &Self::State, _key: &str, _modifiers: Modifiers) -> Option<Self::State> {
        None
    }
    fn hover(&self, _state: &Self::State, _point: Point) -> Option<Self::State> {
        None
    }
    fn status(&self, _state: &Self::State) -> String {
        String::new()
    }
    fn pointer_delay(&self, _state: &Self::State) -> Duration {
        Duration::ZERO
    }
    fn can_import(&self) -> bool {
        false
    }
    fn export(&self, _state: &Self::State) -> Option<(String, String)> {
        None
    }
    fn import(&self, _state: &Self::State, _text: &str) -> Result<Self::State, String> {
        Err("This demo does not import files.".into())
    }
}

pub trait Demo {
    fn scene(&self) -> &Scene;
    fn press(&mut self, point: Point, modifiers: Modifiers, clicks: usize);
    fn move_to(&mut self, point: Point, modifiers: Modifiers);
    fn release(&mut self, point: Point, modifiers: Modifiers);
    fn cancel(&mut self);
    fn reset(&mut self);
    fn controls(&self) -> Vec<Control>;
    fn action(&mut self, id: &str, value: Value);
    fn key(&mut self, key: &str, modifiers: Modifiers);
    fn status(&self) -> String;
    fn tooltip(&self) -> Option<String> {
        None
    }
    fn cursor(&self) -> Cursor {
        Cursor::Default
    }
    fn advance(&mut self, elapsed: Duration) -> bool;
    fn set_animated(&mut self, enabled: bool);
    fn can_import(&self) -> bool;
    fn export(&self) -> Option<(String, String)>;
    fn import(&mut self, text: &str) -> Result<(), String>;
}

pub struct Entry {
    pub id: &'static str,
    pub title: &'static str,
    pub description: &'static str,
    pub create: fn() -> Box<dyn Demo>,
}
impl Entry {
    pub const fn new(
        id: &'static str,
        title: &'static str,
        description: &'static str,
        create: fn() -> Box<dyn Demo>,
    ) -> Self {
        Self {
            id,
            title,
            description,
            create,
        }
    }
}
pub fn demo<E: Example>(example: E) -> Box<dyn Demo> {
    Box::new(crate::runtime::ModelDemo::new(example))
}
