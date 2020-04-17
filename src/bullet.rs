use crate::player::Classes;
use na::Vector2;
use nalgebra as na;
use rstar::{RTreeObject, AABB};
use std::time::Instant;

pub struct Bullet {
    pub vel: Vector2<f32>,
    pub pos: Vector2<f32>,
    pub spawn: Instant,
    pub class: Classes,
    pub id: usize,
    pub owner: usize,
}

impl Bullet {
    pub const RADIUS: f32 = 10.0;
}

impl PartialEq for Bullet {
    fn eq(&self, other: &Bullet) -> bool {
        self.id == other.id
    }
}

impl<'a> RTreeObject for &'a Bullet {
    type Envelope = AABB<[f32; 2]>;

    fn envelope(&self) -> Self::Envelope {
        let size = Bullet::RADIUS;
        AABB::from_corners(
            [self.pos.x - size, self.pos.y - size],
            [self.pos.x + size, self.pos.y + size],
        )
    }
}
