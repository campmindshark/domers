/// Unit quaternion used for orientation device input.
#[derive(Clone, Copy, Debug)]
pub struct Quaternion {
    pub(crate) x: f64,
    pub(crate) y: f64,
    pub(crate) z: f64,
    pub(crate) w: f64,
}

impl Quaternion {
    /// Build from Spectrum's `(x, y, z, w)` float components.
    #[must_use]
    pub fn from_spectrum_components(x: f32, y: f32, z: f32, w: f32) -> Self {
        Self {
            x: f64::from(x),
            y: f64::from(y),
            z: f64::from(z),
            w: f64::from(w),
        }
        .normalize()
    }

    #[allow(
        clippy::cast_possible_truncation,
        reason = "Paintbrush idle orientation mirrors C# CreateFromYawPitchRoll float angles"
    )]
    pub(crate) fn from_unitless_yaw_pitch_roll(yaw: f64, pitch: f64, roll: f64) -> Self {
        Self::from_yaw_pitch_roll_f32(
            (std::f64::consts::TAU * yaw) as f32,
            (std::f64::consts::TAU * pitch) as f32,
            (std::f64::consts::TAU * roll) as f32,
        )
    }

    #[allow(
        clippy::cast_possible_truncation,
        reason = "Orientation overrides are radians passed through C#-compatible float trig"
    )]
    pub(crate) fn from_yaw_pitch_roll(yaw: f64, pitch: f64, roll: f64) -> Self {
        Self::from_yaw_pitch_roll_f32(yaw as f32, pitch as f32, roll as f32)
    }

    pub(crate) fn from_yaw_pitch_roll_f32(yaw: f32, pitch: f32, roll: f32) -> Self {
        let (sy, cy) = (yaw * 0.5).sin_cos();
        let (sp, cp) = (pitch * 0.5).sin_cos();
        let (sr, cr) = (roll * 0.5).sin_cos();
        Self {
            x: f64::from(cy.mul_add(sp * cr, sy * cp * sr)),
            y: f64::from(sy.mul_add(cp * cr, -cy * sp * sr)),
            z: f64::from(cy.mul_add(cp * sr, -sy * sp * cr)),
            w: f64::from(cy.mul_add(cp * cr, sy * sp * sr)),
        }
        .normalize()
    }

    pub(crate) fn normalize(self) -> Self {
        let length = (self.x * self.x + self.y * self.y + self.z * self.z + self.w * self.w).sqrt();
        if length <= f64::EPSILON {
            return Self {
                x: 0.0,
                y: 0.0,
                z: 0.0,
                w: 1.0,
            };
        }
        Self {
            x: self.x / length,
            y: self.y / length,
            z: self.z / length,
            w: self.w / length,
        }
    }

    pub(crate) fn transform_vector(self, x: f64, y: f64, z: f64) -> (f64, f64, f64) {
        let qx2 = self.x + self.x;
        let qy2 = self.y + self.y;
        let qz2 = self.z + self.z;
        let wx2 = self.w * qx2;
        let wy2 = self.w * qy2;
        let wz2 = self.w * qz2;
        let xx2 = self.x * qx2;
        let xy2 = self.x * qy2;
        let xz2 = self.x * qz2;
        let yy2 = self.y * qy2;
        let yz2 = self.y * qz2;
        let zz2 = self.z * qz2;
        (
            (1.0 - yy2 - zz2).mul_add(x, (xy2 - wz2).mul_add(y, (xz2 + wy2) * z)),
            (xy2 + wz2).mul_add(x, (1.0 - xx2 - zz2).mul_add(y, (yz2 - wx2) * z)),
            (xz2 - wy2).mul_add(x, (yz2 + wx2).mul_add(y, (1.0 - xx2 - yy2) * z)),
        )
    }
}
