---
name: iced-custom-widget
description: Author a custom iced 0.14 widget in Rust — the two-file split (the widget vs. a style-only Catalog module), making the widget generic over Theme, builder setters for layout, and a walkthrough of the advanced::Widget trait impl. Use when implementing a new custom Widget rather than composing existing ones.
---

# /iced-custom-widget

How to write a custom iced 0.14 `Widget` that is generic over `Theme`, configured
through builder setters, and styled through a separate style-only module.

Reach for a custom `Widget` only when composing existing widgets can't express the
layout, drawing, or interaction you need. The running example below is `Panel`, a
minimal widget that draws a styled rectangle around optional content — substitute
your own widget's name and fields.

---

## 1. File layout

Split the widget from its styling into two modules:

- `widget/<name>.rs` — the widget struct, its builder methods, the `impl Widget`,
  and `From<W> for Element`.
- `style/<name>.rs` — **only** styling: `StyleFn`, `Status`, `Style`, the `Catalog`
  trait, `impl Catalog` for your theme type, and named style functions. No layout,
  event, or draw logic here; no widget struct here.

This mirrors iced's own crates and keeps appearance swappable without touching
behavior. Keep one widget per module.

---

## 2. The struct — generic over Theme

```rust
pub struct Panel<'a, Message, Theme = iced::Theme, Renderer = iced::Renderer>
where
    Theme: Catalog,
{
    content: Option<Element<'a, Message, Theme, Renderer>>,
    width: Length,
    height: Length,
    class: Theme::Class<'a>,
}
```

- `Message` — the message type the widget can emit.
- `Theme` — defaulted to `iced::Theme`, **bounded by your own `Catalog`**. This bound
  is what makes the widget theme-generic: any `Theme` that implements your `Catalog`
  can style it. The widget itself never hardcodes colors.
- `Renderer` — defaulted to `iced::Renderer`; bound to `advanced::Renderer` (or
  `text::Renderer`, etc.) on the impl block, depending on what you draw.
- `class: Theme::Class<'a>` stores the chosen style — a boxed closure by default.

---

## 3. Constructor + builder setters

`new()` sets sensible defaults; the style class starts at `Theme::default()`:

```rust
pub fn new(content: Option<impl Into<Element<'a, Message, Theme, Renderer>>>) -> Self {
    Self { content: content.map(Into::into),
           width: Length::Shrink, height: Length::Shrink,
           class: Theme::default() }
}
```

Expose a setter for every layout knob a caller commonly configures. Convention —
take `mut self`, accept `impl Into<…>`, return `Self`:

```rust
pub fn width(mut self, width: impl Into<Length>) -> Self { self.width = width.into(); self }
pub fn height(mut self, height: impl Into<Length>) -> Self { self.height = height.into(); self }
```

Typical knobs: `width`, `height`, and where relevant `padding`/`spacing`/`size`
(use `impl Into<Pixels>` for those). The `style()` setter boxes a closure into
`Theme::Class<'a>` — this is what lets callers pass a `|theme, status| Style {…}`:

```rust
pub fn style(mut self, style: impl Fn(&Theme, Status) -> Style + 'a) -> Self
where
    Theme::Class<'a>: From<StyleFn<'a, Theme>>,
{
    self.class = (Box::new(style) as StyleFn<'a, Theme>).into();
    self
}
```

Add a `Default` impl if a content-less default makes sense.

---

## 4. The `advanced::Widget` impl

Implement `iced::advanced::Widget<Message, Theme, Renderer>`. The methods that
matter, with their contracts:

| Method | Job | Default if omitted |
|---|---|---|
| `size` | Return `Size<Length>` from configured `width`/`height`. | — (required) |
| `layout` | Respect `limits`; use `layout::padded`/`contained`; lay out children. | — (required) |
| `draw` | Derive `Status`, fetch `Style` from `Catalog`, `fill_quad`, draw children. | — (required) |
| `tag` / `state` | Internal state type + initial value. Stateless widgets skip these. | stateless / `None` |
| `children` / `diff` | Child state-tree plumbing. | empty / clear |
| `update` | Handle events; `shell.publish(msg)` to emit; mutate state. | no-op |
| `operate` / `mouse_interaction` / `overlay` | Tree queries / cursor / popups. | no-op / `None` |

`draw` is the **only** place `Theme` is consulted — derive a `Status`, ask the
`Catalog` for the resolved `Style`, then render:

```rust
fn draw(&self, tree, renderer, theme: &Theme, style, layout, cursor, viewport) {
    let bounds = layout.bounds();
    let status = if cursor.is_over(bounds) { Status::Hovered } else { Status::Active };
    let s = <Theme as Catalog>::style(theme, &self.class, status);
    renderer.fill_quad(
        renderer::Quad { bounds, border: s.border, shadow: s.shadow, snap: false },
        s.background.unwrap_or(Color::TRANSPARENT.into()),
    );
    // then draw any children at their child layout
}
```

`layout` produces a `Node`, sizing children under the configured limits:

```rust
fn layout(&mut self, tree, renderer, limits) -> Node {
    let limits = limits.width(self.width).height(self.height);
    match self.content.as_mut() {
        Some(el) => layout::contained(&limits, self.width, self.height,
            |l| el.as_widget_mut().layout(&mut tree.children[0], renderer, l)),
        None => Node::new(limits.max()),
    }
}
```

A wrapper widget forwards `tag`/`children`/`diff`/`operate`/`update`/
`mouse_interaction`/`overlay` to its wrapped child; a leaf widget uses the trait
defaults for most of them. Finish with the mandatory conversion so the widget can
be used as an `Element`:

```rust
impl<'a, Message, Theme, Renderer> From<Panel<'a, Message, Theme, Renderer>>
    for Element<'a, Message, Theme, Renderer>
where Message: 'a, Theme: 'a + Catalog, Renderer: 'a + advanced::Renderer,
{ fn from(p: Panel<'a, Message, Theme, Renderer>) -> Self { Element::new(p) } }
```

---

## 5. The style-only module (`style/<name>.rs`)

Exactly these items, in this order — appearance only, no logic. This is the same
`Catalog` pattern every built-in iced widget uses:

```rust
use iced::{Background, Border, Color, Shadow, Theme};

pub type StyleFn<'a, Theme> = Box<dyn Fn(&Theme, Status) -> Style + 'a>;

#[derive(Clone, Copy)]
pub enum Status { Active, Hovered }      // the visual states you distinguish

#[derive(Default)]
pub struct Style {                       // appearance fields only
    pub background: Option<Background>,
    pub border: Border,
    pub shadow: Shadow,
}

pub trait Catalog {
    type Class<'a>;
    fn default<'a>() -> Self::Class<'a>;
    fn style(&self, class: &Self::Class<'_>, status: Status) -> Style;
}

impl Catalog for Theme {
    type Class<'a> = StyleFn<'a, Self>;
    fn default<'a>() -> Self::Class<'a> { Box::new(default) }
    fn style(&self, class: &Self::Class<'_>, status: Status) -> Style { class(self, status) }
}

// named style functions — derive colors from the theme, never hardcode them
pub fn default(theme: &Theme, status: Status) -> Style {
    let palette = theme.extended_palette();
    let background = match status {
        Status::Active => palette.background.weak.color,
        Status::Hovered => palette.background.strong.color,
    };
    Style { background: Some(background.into()), border: Border::default(), ..Style::default() }
}
```

Rule: pull colors from the theme (`theme.extended_palette()`, or your project's own
palette accessor) so the widget follows the active theme. Never bake a literal
`Color` into a style function.

---

## 6. Use it

```rust
Panel::new(Some(content)).width(Length::Fill).style(style::panel::default)
```

## Checklist

- [ ] Two modules: `widget/<name>.rs` + `style/<name>.rs`.
- [ ] Struct generic `<…, Theme = iced::Theme, Renderer = iced::Renderer> where Theme: Catalog`.
- [ ] `class: Theme::Class<'a>`; builder setters take `mut self`, return `Self`.
- [ ] Full `advanced::Widget` impl + `From<W> for Element`.
- [ ] `style.rs` holds only StyleFn/Status/Style/Catalog/impl/style-fns; colors from the theme.

## Going further

### State

A widget struct is rebuilt every frame, so anything that must persist — a drag in
progress, focus, a text cursor position, hover that can't be re-derived from the
cursor — lives in the **widget tree**, not in the struct. Declare a `State` type,
register it with `tag()`, seed it with `state()`, then read/write it through the
tree:

```rust
#[derive(Default)]
struct State { hovered: bool }

fn tag(&self) -> tree::Tag { tree::Tag::of::<State>() }
fn state(&self) -> tree::State { tree::State::new(State::default()) }

// in update():                    let s = tree.state.downcast_mut::<State>();
// in draw() / mouse_interaction(): let s = tree.state.downcast_ref::<State>();
```

Stateless widgets omit `tag`/`state` (the trait defaults make them no-ops). Mutate
state in `update`; the runtime redraws on events.

### Overlays

Dropdowns, menus, and tooltips must render **above** the rest of the tree, escaping
parent clipping and bounds. Return `Some` from `Widget::overlay()`, wrapping a small
struct that implements `overlay::Overlay` in an `overlay::Element`:

```rust
fn overlay<'a>(&'a mut self, tree, layout, renderer, _viewport, translation)
    -> Option<overlay::Element<'a, Message, Theme, Renderer>>
{
    self.open.then(|| overlay::Element::new(Box::new(
        Menu { position: layout.position() + translation, /* … */ }
    )))
}
```

The `Overlay` trait (0.14 — note `update`, mirroring `Widget`; only `layout` and
`draw` are required, the rest default):

```rust
impl<Message, Theme, Renderer> overlay::Overlay<Message, Theme, Renderer> for Menu<…> {
    fn layout(&mut self, renderer: &Renderer, bounds: Size) -> Node { /* node.move_to(self.position) */ }
    fn draw(&self, renderer, theme: &Theme, style, layout, cursor) { /* … */ }
    fn update(&mut self, event, layout, cursor, renderer, clipboard, shell) {}         // default no-op
    fn mouse_interaction(&self, layout, cursor, renderer) -> mouse::Interaction { … }   // default None
}
```

Fold `translation` into the position so the overlay tracks scrolled or moved parents.

### Composition

When your widget internally builds other widgets (`button`, `text_input`, …), the
`Theme` must satisfy *their* `Catalog`s too. Store the built children as `Element`,
delegate the trait methods to `tree.children[i]`, and widen the bound:

```rust
where
    Theme: Catalog + button::Catalog + text_input::Catalog,
    Renderer: advanced::Renderer + text::Renderer,
```

If you build those children with inline `.style(|theme, status| …)` closures, also
bind each child's class to its `StyleFn` so the closure can be stored:

```rust
where for<'b> <Theme as button::Catalog>::Class<'b>: From<button::StyleFn<'b, Theme>>,
```

Simplest alternative: accept already-styled `Element`s from the caller and skip the
extra bounds — the widget just lays them out.
