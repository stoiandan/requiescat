use iced::{Point, Size};

use crate::models::{Cemetery, GraveRectangle};

const A0_LANDSCAPE_WIDTH: f32 = 3370.39;
const A0_LANDSCAPE_HEIGHT: f32 = 2383.94;

#[derive(Debug, Clone, Copy)]
pub(super) struct PageLayout {
    pub(super) width: f32,
    pub(super) height: f32,
    pub(super) margin: f32,
    pub(super) title_size: f32,
    pub(super) subtitle_size: f32,
    pub(super) footer_size: f32,
    pub(super) map_left: f32,
    pub(super) map_right: f32,
    pub(super) map_top: f32,
    pub(super) map_bottom: f32,
}

impl PageLayout {
    pub(super) const A0_LANDSCAPE: Self = Self {
        width: A0_LANDSCAPE_WIDTH,
        height: A0_LANDSCAPE_HEIGHT,
        margin: 96.0,
        title_size: 54.0,
        subtitle_size: 24.0,
        footer_size: 18.0,
        map_left: 96.0,
        map_right: A0_LANDSCAPE_WIDTH - 96.0,
        map_top: A0_LANDSCAPE_HEIGHT - 240.0,
        map_bottom: 150.0,
    };

    pub(super) fn map_width(&self) -> f32 {
        self.map_right - self.map_left
    }

    pub(super) fn map_height(&self) -> f32 {
        self.map_top - self.map_bottom
    }
}

#[derive(Debug, Clone, Copy)]
pub(super) struct CemeteryBounds {
    pub(super) min: Point,
    pub(super) max: Point,
}

impl CemeteryBounds {
    pub(super) fn from_cemetery(cemetery: &Cemetery) -> Option<Self> {
        let mut rectangles = cemetery
            .graves()
            .iter()
            .map(|grave| grave.rectangle())
            .chain(
                cemetery
                    .delimiters()
                    .iter()
                    .map(|delimiter| delimiter.rectangle()),
            );
        let first = rectangles.next()?;
        let mut bounds = Self {
            min: first.top_left(),
            max: Point::new(
                first.top_left().x + first.size().width,
                first.top_left().y + first.size().height,
            ),
        };

        for rectangle in rectangles {
            bounds.include(rectangle);
        }

        Some(bounds.with_padding())
    }

    fn include(&mut self, rectangle: GraveRectangle) {
        self.min.x = self.min.x.min(rectangle.top_left().x);
        self.min.y = self.min.y.min(rectangle.top_left().y);
        self.max.x = self
            .max
            .x
            .max(rectangle.top_left().x + rectangle.size().width);
        self.max.y = self
            .max
            .y
            .max(rectangle.top_left().y + rectangle.size().height);
    }

    fn with_padding(mut self) -> Self {
        let padding = self.width().max(self.height()).max(1.0) * 0.06;
        self.min.x -= padding;
        self.min.y -= padding;
        self.max.x += padding;
        self.max.y += padding;
        self
    }

    pub(super) fn width(&self) -> f32 {
        (self.max.x - self.min.x).max(1.0)
    }

    pub(super) fn height(&self) -> f32 {
        (self.max.y - self.min.y).max(1.0)
    }
}

#[derive(Debug, Clone, Copy)]
pub(super) struct MapTransform {
    bounds: CemeteryBounds,
    scale: f32,
    left: f32,
    top: f32,
}

impl MapTransform {
    pub(super) fn new(bounds: CemeteryBounds, layout: PageLayout) -> Self {
        let frame_width = layout.map_width();
        let frame_height = layout.map_height();
        let scale = (frame_width / bounds.width()).min(frame_height / bounds.height());
        let rendered_width = bounds.width() * scale;
        let rendered_height = bounds.height() * scale;

        Self {
            bounds,
            scale,
            left: layout.map_left + (frame_width - rendered_width) / 2.0,
            top: layout.map_bottom + (frame_height + rendered_height) / 2.0,
        }
    }

    pub(super) fn point(&self, point: Point) -> Point {
        Point::new(
            self.left + (point.x - self.bounds.min.x) * self.scale,
            self.top - (point.y - self.bounds.min.y) * self.scale,
        )
    }

    pub(super) fn size(&self, size: Size) -> Size {
        Size::new(size.width * self.scale, size.height * self.scale)
    }
}

pub(super) fn grid_interval(span: f32) -> f32 {
    let target = (span / 8.0).max(1.0);
    let magnitude = 10.0_f32.powf(target.log10().floor());
    for multiplier in [1.0, 2.0, 5.0, 10.0] {
        let interval = magnitude * multiplier;
        if interval >= target {
            return interval;
        }
    }
    magnitude * 10.0
}
