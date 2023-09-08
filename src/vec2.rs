#[derive(Default, Clone, Copy, Debug, PartialEq)]
pub struct Vec2(pub f32, pub f32);

pub fn lerp(start: f32, end: f32, t: f32) -> f32 {
    start * (1.0 - t) + end * t
}

impl Vec2 {
    pub const ZERO: Vec2 = Vec2(0.0, 0.0);
    const MAX_LINE_MIDPOINTS: usize = 5000;

    pub fn lerp(v1: &Self, v2: &Self, t: f32) -> Self {
        Vec2(lerp(v1.0, v2.0, t), lerp(v1.1, v2.1, t))
    }

    pub fn dist(&self, r: &Self) -> f32 {
        ((r.0 - self.0).powf(2.0) + (r.1 - self.1).powf(2.0)).sqrt() 
    }

    pub fn round(&self) -> (isize, isize) {
        (self.0.round() as isize, self.1.round() as isize)
    }

    pub fn signum(self) -> Self {
        Self(self.0.signum(), self.1.signum())
    }

    pub fn line_substeps(p1: Vec2, p2: Vec2, n: usize) -> Vec<(isize, isize)> {
        let mut pts = vec![];

        for s in 0..n {
            let t = if n == 0 { 0.0 } else { s as f32 / n as f32 };
            pts.push(Vec2::lerp(&p1, &p2, t).round());
        }
        pts
    }

    pub fn linef32(p1: Vec2, p2: Vec2) -> Vec<Vec2> {
        let mut pts = vec![];

        let n = ((p1.dist(&p2) as usize).clamp(1, Self::MAX_LINE_MIDPOINTS) as f32 * 1.0) as usize;
        for s in 0..n {
            let t = if n == 0 { 0.0 } else { s as f32 / n as f32 };
            pts.push(Vec2::lerp(&p1, &p2, t));
        }
        pts
    }

    pub fn line(p1: Vec2, p2: Vec2) -> Vec<(isize, isize)> {
        let mut pts = vec![];

        let n = ((p1.dist(&p2) as usize).clamp(1, Self::MAX_LINE_MIDPOINTS) as f32 * 1.0) as usize;
        for s in 0..n {
            let t = if n == 0 { 0.0 } else { s as f32 / n as f32 };
            pts.push(Vec2::lerp(&p1, &p2, t).round());
        }
        pts
    }
}

impl std::ops::Add<Vec2> for Vec2 {
    type Output = Vec2;
    fn add(self, rhs: Vec2) -> Self::Output {
        Self (
            self.0 + rhs.0,
            self.1 + rhs.1
        )
    }
}

impl std::ops::Sub<Vec2> for Vec2 {
    type Output = Vec2;
    fn sub(self, rhs: Vec2) -> Self::Output {
        Self (
            self.0 - rhs.0,
            self.1 - rhs.1
        )
    }
}

impl std::ops::Mul<f32> for Vec2 {
    type Output = Vec2;
    fn mul(self, rhs: f32) -> Self::Output {
        Self (
            self.0 * rhs,
            self.1 * rhs
        )
    }
}

impl std::ops::Mul<Vec2> for Vec2 {
    type Output = Vec2;
    fn mul(self, rhs: Vec2) -> Self::Output {
        Self (
            self.0 * rhs.0,
            self.1 * rhs.1
        )
    }
}

impl From<sdl2::rect::Point> for Vec2 {
    fn from(t: sdl2::rect::Point) -> Vec2 {
        Vec2(t.x() as f32, t.y() as f32)
    }
}
