use serde::{Serialize, Deserialize};

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub struct Location {
    x: f32,
    y: f32,
    z: f32,
    yaw: f32,
    pitch: f32
}

impl Into<String> for Location {
    fn into(self) -> String {
        format!("{} {} {}", self.x, self.y, self.z)
    }
}

impl Location {
    pub fn simple<F: Into<f32>>(x: F, y: F, z: F) -> Self {
        Self {
            x: x.into(),
            y: y.into(),
            z: z.into(),
            yaw: 0.0,
            pitch: 0.0
        }
    }

    pub fn full<F: Into<f32>>(x: F, y: F, z: F, yaw: F, pitch: F) -> Self {
        Self {
            x: x.into(),
            y: y.into(),
            z: z.into(),
            yaw: yaw.into(),
            pitch: pitch.into()
        }
    }

    pub fn x(&self) -> f32 {
        self.x.clone()
    }

    pub fn y(&self) -> f32 {
        self.y.clone()
    }

    pub fn z(&self) -> f32 {
        self.z.clone()
    }

    pub fn yaw(&self) -> f32 {
        self.yaw.clone()
    }

    pub fn pitch(&self) -> f32 {
        self.pitch.clone()
    }

}