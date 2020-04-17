use crate::player::Player;
use na::Vector2;
use nalgebra as na;
use rand::prelude::*;
use rstar::{RTreeObject, AABB};
use serde_repr::*;
use std::iter::Peekable;
use std::time::{Duration, Instant};

#[derive(Serialize_repr, Clone, Copy)]
#[repr(u8)]
pub enum BossType {
    NormalBoss,
    HardcoreBoss,
}

impl<'a> RTreeObject for &'a Boss {
    type Envelope = AABB<[f32; 2]>;

    fn envelope(&self) -> Self::Envelope {
        let size = Boss::RADIUS;
        AABB::from_corners(
            [self.pos.x - size, self.pos.y - size],
            [self.pos.x + size, self.pos.y + size],
        )
    }
}
impl Boss {
    pub const RADIUS: f32 = 30.0;
}

impl<'a> RTreeObject for &'a BossBullet {
    type Envelope = AABB<[f32; 2]>;

    fn envelope(&self) -> Self::Envelope {
        let size = BossBullet::RADIUS;
        AABB::from_corners(
            [self.pos.x - size, self.pos.y - size],
            [self.pos.x + size, self.pos.y + size],
        )
    }
}

impl BossBullet {
    pub const RADIUS: f32 = 10.0;
}
pub struct Boss {
    pub pos: Vector2<f32>,
    pub health: u8,
    pub shot_time: Instant,
    pub shot_time2: Instant,
    pub class: BossType,
}
impl Boss {
    pub fn tick<'a>(
        &mut self,
        dt: f32,
        rng: &mut ThreadRng,
        boss_bullets: &mut Vec<BossBullet>,
        mut players: Peekable<impl Iterator<Item = &'a Player>>,
    ) {
        let mut nearest_player = *players.peek().unwrap();
        let mut nearest_dist = std::f32::MAX;
        for p in players {
            let dist = (self.pos - p.pos).magnitude();
            if dist < nearest_dist {
                nearest_dist = dist;
                nearest_player = p;
            }
        }
        let vel = (nearest_player.pos - self.pos).normalize() * 3.0;
        self.pos += vel * dt;

        if self.shot_time.elapsed() > Duration::from_millis(500) {
            let velp = Vector2::new(vel.y, -vel.x);
            boss_bullets.push(BossBullet {
                pos: self.pos - velp * 10.0,
                id: rng.gen::<usize>(),
                spawn: Instant::now(),
                vel: vel * 3.0 + velp * 0.5,
            });
            boss_bullets.push(BossBullet {
                pos: self.pos + velp * 10.0,
                id: rng.gen::<usize>(),
                spawn: Instant::now(),
                vel: vel * 3.0 - velp * 0.5,
            });
            self.shot_time = Instant::now();
        }
        if let BossType::HardcoreBoss = self.class {
            if self.shot_time2.elapsed() > Duration::from_millis(250) {
                let velp = Vector2::new(vel.y, -vel.x);
                boss_bullets.push(BossBullet {
                    pos: self.pos - velp * 20.0,
                    id: rng.gen::<usize>(),
                    spawn: Instant::now(),
                    vel: vel * 0.1,
                });
                boss_bullets.push(BossBullet {
                    pos: self.pos + velp * 20.0,
                    id: rng.gen::<usize>(),
                    spawn: Instant::now(),
                    vel: vel * 0.1,
                });
                self.shot_time2 = Instant::now();
            }
        }
    }
}
pub struct BossBullet {
    pub vel: Vector2<f32>,
    pub pos: Vector2<f32>,
    pub spawn: Instant,
    pub id: usize,
}
