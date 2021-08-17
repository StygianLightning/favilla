//use crate::aabb::AABB;
use cgmath::{point2, vec4, Matrix4, Point2, Vector2};

pub const OPENGL_TO_VULKAN_MATRIX: cgmath::Matrix4<f32> = cgmath::Matrix4::new(
    1.0, 0.0, 0.0, 0.0, // first row
    0.0, 1.0, 0.0, 0.0, // second row
    0.0, 0.0, 0.5, 0.0, // third row
    0.0, 0.0, 0.5, 1.0, // fourth row
);

#[derive(Debug)]
pub struct Camera {
    min: Point2<f32>,
    extent: Vector2<f32>,
    near: f32,
    far: f32,
}

impl Camera {
    pub fn new(extent: Vector2<f32>, near: f32, far: f32) -> Self {
        Self {
            min: point2(0., 0.),
            extent,
            near,
            far,
        }
    }

    pub fn min(&self) -> Point2<f32> {
        self.min
    }

    pub fn set_min(&mut self, min: Point2<f32>) {
        self.min = min;
    }

    pub fn centre(&self) -> Point2<f32> {
        self.min + 0.5 * self.extent
    }

    pub fn max(&self) -> Point2<f32> {
        self.min + self.extent
    }

    pub fn extent(&self) -> Vector2<f32> {
        self.extent
    }

    pub fn near(&self) -> f32 {
        self.near
    }

    pub fn far(&self) -> f32 {
        self.far
    }

    pub fn centre_on(&mut self, centre: Point2<f32>) {
        self.min = centre - 0.5 * self.extent;
    }

    pub fn set_extent(&mut self, extent: Vector2<f32>) {
        self.extent = extent;
    }

    pub fn view_projection_matrix(&self) -> Matrix4<f32> {
        OPENGL_TO_VULKAN_MATRIX
            * Matrix4::from_cols(
                vec4(2. / self.extent.x, 0., 0., 0.),
                vec4(0., 2. / self.extent.y, 0., 0.),
                vec4(0., 0., 2. / (self.far - self.near), 0.),
                vec4(
                    -(self.extent.x + 2. * self.min.x) / self.extent.x,
                    -(self.extent.y + 2. * self.min.y) / (self.extent.y),
                    -(self.far + self.near) / (self.far - self.near),
                    1.,
                ),
            )
            * self.invert_viewport()
    }

    fn invert_viewport(&self) -> Matrix4<f32> {
        Matrix4::from_cols(
            vec4(1., 0., 0., 0.),
            vec4(0., -1., 0., 0.),
            vec4(0., 0., 1., 0.),
            vec4(0., self.extent.y, 0., 1.),
        )
    }

    pub fn projection_matrix(&self) -> Matrix4<f32> {
        OPENGL_TO_VULKAN_MATRIX
            * Matrix4::from_cols(
                vec4(2. / self.extent.x, 0., 0., 0.),
                vec4(0., 2. / self.extent.y, 0., 0.),
                vec4(0., 0., 2. / (self.far - self.near), 0.),
                vec4(
                    -1., //-(right + 0.) / (right - 0.),
                    -1., //-(top + bottom) / (top - bottom),
                    -(self.far + self.near) / (self.far - self.near),
                    1.,
                ),
            )
            * self.invert_viewport()
    }
}
