use core::str;
use std::sync::Arc;
use std::{ops::Deref, rc::Rc, sync::Weak};

use cosmic::iced::advanced::graphics::text::Raw;
use cosmic::iced::event::Status;
use cosmic::iced::keyboard::key::Named;
use cosmic::iced::keyboard::Event as KeyEvent;
use cosmic::iced::keyboard::Key;
use cosmic::iced::Event;
use cosmic::iced_core::renderer::Renderer as _;
use cosmic::iced_core::text::Renderer as _;
use cosmic::iced_renderer::graphics::text::cosmic_text::fontdb::Family;
use cosmic::Renderer;
use cosmic::{
    iced::{
        alignment::{Horizontal, Vertical},
        Border, Color, Font, Length, Pixels, Point, Radius, Rectangle, Shadow, Size,
    },
    iced_core::{
        layout,
        renderer::Quad,
        text::{LineHeight, Shaping, Wrapping},
        Text,
    },
    iced_wgpu::graphics::text::cosmic_text::{
        fontdb::{self, Database, FaceInfo, Query, Source},
        Font as ExtraFont, Stretch, Style, Weight,
    },
    widget::Widget,
};

use unicode_width::UnicodeWidthChar;

#[derive(Clone)]
pub struct TerminalDisplay<Message> {
    // TODO: try the performance when the display bundle is in a fixed size array
    //
    // TODO: add a visible column amount and visible row amount
    pub cells: Vec<Vec<DisplayCell>>,
    pub glyph_size: f32,
    pub font: String,
    pub line_height: f32,
    pub font_source: Source,
    pub font_index: u32,
    pub on_input: Rc<Box<dyn Fn(char) -> Message>>,
}

// a bundle is a grou pof cells that have the exact same style
#[derive(Clone, Debug)]
pub struct DisplayBundle {
    // TODO: try the performance when it is a fixed size array instead of a vector
    pub characters: Vec<char>,
    pub style: DisplayStyle,

    pub unicode_positions: Vec<usize>,

    // with these maybe try the perofrmance when its a pointer
    pub character_start: usize, // where the character starts in the row
    pub character_end: usize,   // where the character ends in the row
}
#[derive(Clone, Copy)]
pub struct DisplayCell {
    pub character: char,
    pub style: Option<DisplayStyle>,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct DisplayStyle {
    // TODO: terminal color management
    pub background: Option<usize>,
    pub foreground: Option<usize>,
    pub style_metadata: usize,
}

impl<Message> TerminalDisplay<Message> {
    pub fn new(
        font_name: String,
        line_height: f32,
        stdin_read: Box<dyn Fn(char) -> Message>,
    ) -> Self {
        let mut database = Database::new();
        database.load_system_fonts();

        let font_id = database
            .faces()
            .filter(|face_info| {
                face_info.families.iter().any(|family| {
                    // println!("family 0: {:?} family 1 (language): {:?}", family.0, family.1);
                    family.0 == font_name
                })
            })
            .collect::<Vec<_>>();

        // let font = ExtraFont::new(
        //     &database,
        // ).expect("Expected to find font");

        // IMPORTANT: rework this in a much more straightforward way rather than this workaround
        let font = font_id
            .first()
            .unwrap_or_else(|| panic!("Could not find font name {}", font_name));

        // if !font.monospaced {
        //     panic!("Expected a monospaced font")
        // }

        // let data = with_source_font_data(font.source.clone(), |font_data| {
        //     let face = ttf_parser::Face::parse(font_data, font.index).unwrap();
        //     let hor_advance = face.glyph_hor_advance(face.glyph_index(' ')?)? as f32;
        //     let upem = face.units_per_em() as f32;
        //     Some(hor_advance / upem)
        // });

        Self {
            cells: vec![Vec::new()],
            font: font_name,
            glyph_size: get_glyph_size(font.source.clone(), font.index, ' ') * line_height + 0.05,
            font_source: font.source.clone(),
            font_index: font.index,
            line_height,
            on_input: Rc::new(stdin_read),
        }
    }
}

fn get_glyph_size(font_source: Source, index: u32, glyph: char) -> f32 {
    let source = match font_source {
        Source::File(ref path) => &std::fs::read(path).ok().unwrap(),
        Source::Binary(ref data) => data.as_ref().as_ref(),
        Source::SharedFile(_, ref data) => data.as_ref().as_ref(),
    };
    let face = ttf_parser::Face::parse(source, index).unwrap();
    println!("got glyph: {:?}", glyph);
    let hor_advance = face
        .glyph_hor_advance(face.glyph_index(glyph).unwrap())
        .unwrap() as f32;
    let upem = face.units_per_em() as f32;

    hor_advance / upem
}

fn with_source_font_data<P, T>(source: fontdb::Source, p: P) -> Option<T>
where
    P: FnOnce(&[u8]) -> T,
{
    match source {
        Source::File(ref path) => {
            let data = std::fs::read(path).ok()?;

            Some(p(&data))
        }
        Source::Binary(ref data) => Some(p(data.as_ref().as_ref())),
        Source::SharedFile(_, ref data) => Some(p(data.as_ref().as_ref())),
    }
}

impl DisplayStyle {
    pub fn none() -> Self {
        Self {
            background: None,
            foreground: None,
            style_metadata: 0,
        }
    }
}

impl<Message> Widget<Message, cosmic::Theme, Renderer> for TerminalDisplay<Message>
where
    Message: Clone,
{
    fn layout(
        &self,
        tree: &mut cosmic::iced_core::widget::Tree,
        renderer: &Renderer,
        limits: &cosmic::iced_core::layout::Limits,
    ) -> cosmic::iced_core::layout::Node {
        let limits = limits.width(Length::Fill).height(Length::Fill);

        // TODO: do some update logic later
        let size = Size::new(limits.max().width, self.line_height);
        layout::Node::new(limits.resolve(Length::Fill, Length::Fill, size))
    }
    fn draw(
        &self,
        tree: &cosmic::iced_core::widget::Tree,
        renderer: &mut Renderer,
        theme: &cosmic::Theme,
        style: &cosmic::iced_core::renderer::Style,
        layout: cosmic::iced_core::Layout<'_>,
        cursor: cosmic::iced_core::mouse::Cursor,
        viewport: &cosmic::iced::Rectangle,
    ) {
        // let cosmic_theme = theme.cosmic();

        // let corner_radius = cosmic_theme
        //     .radius_s()
        //     .map(|x| if x < 4.0 { x - 1.0 } else { x + 3.0 });
        //
        // TODO: add some padding later
        let view_position = layout.position();

        // TODO: add code that renders the default background, mainly used for cells that do not
        // have a background specified
        {}

        let mut previous_style: Option<DisplayStyle> = None;
        let mut bundle_text: Vec<char> = Vec::new();
        // TODO: try performance with just regular mutable index
        for (index, row) in self.cells.iter().enumerate() {
            let mut offset = 0;
            for cell in row {
                let mut char_width = 1;
                if cell.character.is_ascii() {
                    if previous_style == cell.style {
                        bundle_text.insert(bundle_text.len(), cell.character);
                        // bundle_text[bundle_text.len()] = cell.character;
                        continue;
                    } else {
                        previous_style = cell.style;
                    }
                } else {
                    char_width = cell.character.width().unwrap();
                    offset += char_width;
                    previous_style = None;
                }
                let position = Point {
                    x: view_position.x + self.glyph_size * (index + offset) as f32,
                    // x: view_position.x + offset,
                    y: view_position.y + ((self.line_height) * index as f32),
                }; // position

                // offset += glyph_size * (bundle.character_start) as f32;

                let size = Size::new(
                    self.glyph_size * (char_width + bundle_text.len()) as f32,
                    self.line_height,
                );
                renderer.fill_quad(
                    Quad {
                        bounds: Rectangle::new(position, size),
                        // ..Default::default()
                        border: Border {
                            color: Color::new(0.0, 1.0, 0.0, 0.2),
                            width: 1.0,
                            radius: Radius::new(0),
                        },
                        ..Default::default()
                    },
                    Color::new(1.0, 0.0, 0.0, 0.4),
                );
                // }
                renderer.fill_text(
                    Text {
                        // content: bundle.characters.iter().map(ToString::to_string).collect(),
                        content: if bundle_text.is_empty() {
                            cell.character.to_string()
                        } else {
                            // str::from_utf8(bun)
                            String::from_iter(bundle_text.iter())
                        },
                        size: Pixels(self.line_height),
                        line_height: LineHeight::Absolute(Pixels(self.line_height)),
                        // bounds: Size::new(layout.bounds().width, self.line_height),
                        bounds: size,
                        font: unsafe { Font::with_name(make_static_str(&self.font)) }, //"ProggyClean CE Nerd Font"),
                        horizontal_alignment: Horizontal::Left,
                        vertical_alignment: Vertical::Top,
                        wrapping: Wrapping::None,
                        shaping: Shaping::Advanced, // might need advanced?
                    },
                    position,
                    // Point {
                    //     x: view_position.x + (self.glyph_size * bundle.character_start as f32),
                    //     y: view_position.y + (self.line_height * index as f32  )
                    // }, // position
                    Color::new(1.0, 1.0, 1.0, 1.0), // TODO
                    Rectangle::new(position, size), // clip bounds
                );

                // renderer.fill_text(text, position, color, clip_bounds);
                // unicode_offset += offset;
            }
        }

        // let scrollbar_w = f32::from(cosmic_theme.spacing.space_xxs);
    }

    fn on_event(
        &mut self,
        _state: &mut cosmic::iced_core::widget::Tree,
        event: cosmic::iced::Event,
        _layout: layout::Layout<'_>,
        _cursor: cosmic::iced_core::mouse::Cursor,
        _renderer: &Renderer,
        _clipboard: &mut dyn cosmic::iced_core::Clipboard,
        shell: &mut cosmic::iced_core::Shell<'_, Message>,
        _viewport: &Rectangle,
    ) -> Status {
        match event {
            Event::Keyboard(KeyEvent::KeyPressed {
                key: Key::Named(named),
                modified_key: Key::Named(modified_name),
                modifiers,
                text,
                ..
            }) => match named {
                Named::Enter => {
                    shell.publish(self.on_input.clone()('\x0D'));
                    return Status::Captured;
                }
                Named::Space => {
                    shell.publish(self.on_input.clone()(
                        text.and_then(|c| c.chars().next()).unwrap_or_default(),
                    ));

                    return Status::Captured;
                }
                _ => (),
            },
            // }) if named == modified_name => match named {
            //     _ => Status::Ignored,
            // },
            Event::Keyboard(KeyEvent::KeyPressed {
                key,
                modified_key,
                physical_key,
                location,
                modifiers,
                text,
            }) => {
                let character = text.and_then(|c| c.chars().next()).unwrap_or_default();
                // if let Some(input) = &self.on_input {
                // shell.publish(self.on_input.clone());
                shell.publish(self.on_input.clone()(character));
                return Status::Captured;
                // }
            }
            _ => (),
        };

        Status::Ignored
    }
    fn size(&self) -> Size<Length> {
        Size::new(Length::Fill, Length::Fill)
    }
}

pub unsafe fn make_static_str<'a>(key: &'a str) -> &'static str {
    std::mem::transmute::<&'a str, &'static str>(key)
}
