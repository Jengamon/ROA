use specs;
use nalgebra::{Vector2, Point2, Rotation2};
use graphics::{Vertex, Index};
use image::DynamicImage;
use palette::{Colora, Rgb};

mod movement;
pub use self::movement::*;
mod physics;
pub use self::physics::*;

// TODO Put Spatial and friends into submodule

#[derive(Clone)]
// This object physically exists at a point.
pub struct Spatial {
    pub pos: Point2<f32>,
    pub origin: Point2<f32>, // Relative to the bottom left corner
    pub rotation: Rotation2<f32>, // NOTE NEVER CHANGE disp! The render system already sets it.
}

// For shape support
pub trait VerticesProvider: Send + Sync {
    fn provide(&self) -> (Vec<Vertex>, Option<Vec<Index>>);
}

#[derive(Clone, Copy, Debug)]
pub struct RectangleProvider {
    half_extants: Vector2<f32>,
    color: Colora
}

impl RectangleProvider {
    pub fn new_from_size_components(x: f32, y: f32, color: Colora) -> Box<RectangleProvider> {
        let prov = RectangleProvider {
            half_extants: Vector2::new(x / 2.0, y / 2.0),
            color: color
        };
        Box::new(prov)
    }
}

impl VerticesProvider for RectangleProvider {
    fn provide(&self) -> (Vec<Vertex>, Option<Vec<Index>>) {
        let colort: Rgb = self.color.color.into();
        let color = [colort.red, colort.green, colort.blue, self.color.alpha];
        let points = vec![
            Vertex { position: [0.0, 0.0], color: color, tex_coords: [0.0, 0.0] },
            Vertex { position: [self.half_extants.x * 2.0, 0.0], color: color, tex_coords: [1.0, 0.0] },
            Vertex { position: [self.half_extants.x * 2.0, self.half_extants.y * 2.0], color: color, tex_coords: [1.0, 1.0] },
            Vertex { position: [0.0, self.half_extants.y * 2.0], color: color, tex_coords: [0.0, 1.0] }
        ];
        (points, Some(vec![0, 1, 2, 2, 0, 3]))
    }
}

impl specs::Component for Spatial {
    type Storage = specs::VecStorage<Spatial>;
}

// The Render system stores what "type_index" means, and the other numbers
// are used to support animation.
pub enum VisualType {
    Sprite {
        // TODO Make sure the common interface supports Sprites (animated stuff), too.
    },
    // TODO Hide using a common interface
    // VProv Texture ZLayer
    Still(Box<VerticesProvider>, Option<DynamicImage>, f32)
}

impl specs::Component for VisualType {
    type Storage = specs::VecStorage<VisualType>;
}
