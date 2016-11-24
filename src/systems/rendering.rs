use specs;
use graphics::{Vertex, Index};
use time::Duration;
use std::sync::mpsc::{Sender, Receiver, channel};
use image::DynamicImage;
use components::{Spatial, VisualType};
use nalgebra::{Isometry2, OrthographicMatrix3, Matrix4, Point2, Vector2};

pub type RenderPipeIn = Sender<RenderInstruction>;
pub type RenderPipeOut = Receiver<RenderInstruction>;

pub fn create_render_channel() -> (RenderPipeIn, RenderPipeOut) {
    channel()
}

#[allow(dead_code)]
#[derive(Clone)]
pub enum RenderInstruction {
    // ClearScreen(f32, f32, f32, f32), // DEPRECATED XXX Handle this outside
    // Vertices Indices Texture ShaderID Modelmatrix
    Draw(Vec<Vertex>, Option<Vec<Index>>, Option<DynamicImage>, String, Isometry2<f32>),
    Zoom(f32), // We don't support separate x and y zooms...yet.
    Translate(f32, f32),
    SetOrigin(f32, f32),
}

pub struct RenderSystem {
    pipeline: Sender<RenderInstruction>,
}

impl RenderSystem {
    pub fn new(p: Sender<RenderInstruction>) -> RenderSystem {
        RenderSystem {
            pipeline: p,
        }
    }
}

impl specs::System<Duration> for RenderSystem {
    fn run(&mut self, arg: specs::RunArg, _: Duration) {
        use specs::Join;

        let (mut spat, vtype, ents) = arg.fetch(|w| {
            (w.write::<Spatial>(), w.read::<VisualType>(), w.entities())
        });
        self.pipeline.send(RenderInstruction::Translate(-1.0, -0.5)).unwrap();
        for (s, v, _) in (&mut spat, &vtype, &ents).iter() {
            // Here we kind of change it up!
            use nalgebra::{RotationWithTranslation, Rotation};
            use nalgebra as na;

            // let iso = Isometry2::from_rotation_matrix(-s.pos.clone().to_vector(), na::one());
            let iso = Isometry2::from_rotation_matrix(s.pos.clone().to_vector(), na::one()).append_rotation_wrt_point(&s.rotation.rotation(), &(s.pos.as_vector() + s.origin.as_vector()));
            // let iso = Isometry2::new(s.pos.clone().to_vector(), na::one());
            match v {
                &VisualType::Sprite { .. } => (),
                &VisualType::Still(ref verts_gen, ref tex) => {
                    let (verts, indx) = verts_gen.provide();
                    self.pipeline.send(RenderInstruction::Draw(verts, indx, tex.clone(), "basic".into(), iso)).unwrap();
                }
            }
        }
    }
}

use std::convert::AsRef;
use std::fs::File;
use std::io::Error as IoError;

// This function is made of LOVE, CARING, and KINDNESS
// It loads the pair of shader files required (vertex, fragment),
// puts them into strings, and automatically determines the names too.
//
// Shaders should be present in a "shaders" folder separate(?) from
// the "data" folder. The name given (ex. "basic") is the name of the pair
// of files making up the shader program, with fn.vert and fn.frag being loaded.
// (ex. "basic.vert" and "basic.frag")
fn load_shaders<P: AsRef<str>>(s: P) -> Result<(String, String), IoError> {
    use std::path::Path;
    let base = Path::new("shaders").join(s.as_ref());
    Ok((match File::open(&base.with_extension("vert")) {
        Ok(mut file) => {
            use std::io::Read;

            let mut s = String::new();
            file.read_to_string(&mut s).unwrap();
            s
        },
        Err(e) => return Err(e)
    },
    match File::open(&base.with_extension("frag")) {
        Ok(mut file) => {
            use std::io::Read;

            let mut s = String::new();
            file.read_to_string(&mut s).unwrap();
            s
        },
        Err(e) => return Err(e)
    }))
}

// This keeps an origin and a Decomposed together, mainly for Renderer purposes, simulating SFML
#[derive(Clone, Debug)]
pub struct View {
    pub origin: Point2<f32>,
    pub viewport_size: (f32, f32),
    pub transform: Isometry2<f32>,
}

impl View {
}

// This struct runs on the other side, and trys to organize and realize the commands of the rendering system.
// NOTE FUTURE: May implement GUI system using this
pub struct Renderer {
    receiver: Receiver<RenderInstruction>,
    // TODO Move projection to view to complete it.
    projection: OrthographicMatrix3<f32>,
    view: View,
    default_view: View, // A 1 to 1 mapping of the screen
}

use glium::Surface;
use glium::backend::Facade;
use nalgebra as na;
impl Renderer {
    pub fn new(r: Receiver<RenderInstruction>, wsize: (u32, u32)) -> Renderer {
        let default_view = View {
            origin: Point2::new(wsize.0 as f32 / 2.0, wsize.1 as f32 / 2.0),
            viewport_size: (wsize.0 as f32, wsize.1 as f32),
            transform: na::one()
        };

        Renderer {
            receiver: r,
            projection: OrthographicMatrix3::new(0.0, wsize.0 as f32, 0.0, wsize.1 as f32, 0.0, 1.0),
            view: default_view.clone(),
            default_view: default_view
        }
    }

    pub fn size_and_center(&mut self, x: u32, y: u32) {
        self.size(x, y);
        self.center(x as f32, y as f32);
    }

    pub fn center(&mut self, x: f32, y: f32) {
        self.view = View {
            origin: Point2::new(x / 2.0, y / 2.0),
            viewport_size: (x, y),
            transform: na::one()
        };
    }

    // Sets screen size
    pub fn size(&mut self, w: u32, h: u32) {
        self.projection =  OrthographicMatrix3::new(0.0, w as f32, 0.0, h as f32, 0.0, 1.0);
    }

    pub fn draw<F: Facade, S: Surface>(&mut self, f: &F, surface: &mut S) {
        use nalgebra::{Translation, ToHomogeneous};

        // Check if there are any instructions
        while let Ok(inst) = self.receiver.try_recv() {
            match inst {
                // RenderInstruction::ClearScreen(r, g, b, a) => surface.clear_color(r, g, b, a),
                // RenderInstruction::Zoom(by) => self.view.transform.scale = by,
                // Translate everything drawn by amount
                RenderInstruction::Translate(x, y) => self.view.transform.append_translation_mut(&-Vector2::new(x,y)),
                RenderInstruction::SetOrigin(x, y) => self.view.origin = Point2::new(x, y),
                RenderInstruction::Draw(vb, ib, tex, shd, model_iso) => {
                    use glium::{IndexBuffer, index, VertexBuffer, Program};

                    // TODO: Support view scaling (zoom)
                    let view_iso = self.view.transform;
                    let proj_m = self.projection.to_matrix();

                    let uniforms = match tex {
                        Some(_) => unimplemented!(),
                        None => uniform!{
                            projection: *proj_m.as_ref(),
                            view: *view_iso.to_homogeneous().as_ref(),
                            model: *na::to_homogeneous(&model_iso).as_ref()
                        }
                    };
                    let index_buffer = match ib {
                        Some(indx) => Some(IndexBuffer::new(f, index::PrimitiveType::TrianglesList, &indx).unwrap()),
                        None => None
                    };
                    let indsource: index::IndicesSource = match index_buffer {
                        Some(ref ib) =>ib.into(),
                        None => index::NoIndices(index::PrimitiveType::TrianglesList).into()
                    };
                    let vertsource = VertexBuffer::new(f, &vb).unwrap();
                    let (vert_shd_src, frag_shd_src) = load_shaders(shd).unwrap();// TODO: Log this.
                    let program = Program::from_source(f, &vert_shd_src, &frag_shd_src, None).unwrap();

                    surface.draw(&vertsource, indsource, &program, &uniforms, &Default::default()).unwrap();
                },
                _ => ()
            }
        }
    }
}
