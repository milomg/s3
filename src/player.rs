use crate::bullet::Bullet;
use na::Vector2;
use nalgebra as na;
use rand::prelude::*;
use rstar::{RTreeObject, AABB};
use serde_derive::{Deserialize, Serialize};
use std::time::{Duration, Instant};

#[derive(Deserialize, Serialize, Copy, Clone)]
pub enum Classes {
    Sniper,
    Quickshot,
}

pub struct Player {
    pub id: usize,
    pub vel: Vector2<f32>,
    pub pos: Vector2<f32>,
    pub angle: f32,
    pub health: u8,
    pub mana: u8,
    pub mouse: bool,
    pub split: bool,
    pub join: bool,
    pub shot_time: Instant,
    pub split_time: Instant,
    pub escape_time: Option<Instant>,
    pub class: Classes,
    pub name: String,
}

impl<'a> RTreeObject for &'a Player {
    type Envelope = AABB<[f32; 2]>;

    fn envelope(&self) -> Self::Envelope {
        let size = Player::RADIUS;
        AABB::from_corners(
            [self.pos.x - size, self.pos.y - size],
            [self.pos.x + size, self.pos.y + size],
        )
    }
}

impl Player {
    pub const RADIUS: f32 = 35.0;
    pub fn tick(&mut self, dt: f32, rng: &mut ThreadRng, bullets: &mut Vec<Bullet>) {
        let acc = Vector2::new(self.angle.sin(), self.angle.cos());
        if self.split && (self.split_time.elapsed() > Duration::from_millis(600)) && self.mana > 100
        {
            self.split_time = Instant::now();
            self.mana -= 100;
        }
        if self.split_time.elapsed() < Duration::from_millis(600) {
            self.vel += 1. * acc * dt;
            self.vel *= (0.95_f32).powf(dt);
        } else {
            self.vel += 0.6 * acc * dt;
            self.vel *= (0.9_f32).powf(dt);
        }
        self.pos += self.vel;
        self.pos.x = self.pos.x.max(0.0).min(800.0);
        self.pos.y = self.pos.y.max(0.0).min(800.0);

        if self.mouse
            && self.shot_time.elapsed()
                > Duration::from_millis(match self.class {
                    Classes::Quickshot => 750,
                    Classes::Sniper => 1000,
                })
        {
            let acopy = Vector2::new(acc.y, -acc.x);

            match self.class {
                Classes::Quickshot => {
                    for i in (10..=12).step_by(1) {
                        let f = i as f32;
                        bullets.push(Bullet {
                            pos: self.pos - acopy * 35.0,
                            vel: acc * (f) - acopy * (11.0 - f),
                            spawn: Instant::now(),
                            id: rng.gen::<usize>(),
                            owner: self.id,
                            class: self.class,
                        });
                        bullets.push(Bullet {
                            pos: self.pos + acopy * 35.0,
                            vel: acc * (f) + acopy * (11.0 - f),
                            spawn: Instant::now(),
                            id: rng.gen::<usize>(),
                            owner: self.id,
                            class: self.class,
                        });
                    }
                }
                _ => {
                    bullets.push(Bullet {
                        pos: self.pos + acopy * 35.0,
                        vel: acc * 15.0 - acopy,
                        spawn: Instant::now(),
                        id: rng.gen::<usize>(),
                        owner: self.id,
                        class: self.class,
                    });
                    bullets.push(Bullet {
                        pos: self.pos + acc * 35.0,
                        vel: acc * 15.0 - acc,
                        spawn: Instant::now(),
                        id: rng.gen::<usize>(),
                        owner: self.id,
                        class: self.class,
                    });
                    bullets.push(Bullet {
                        pos: self.pos - acopy * 35.0,
                        vel: acc * 15.0 + acopy,
                        spawn: Instant::now(),
                        id: rng.gen::<usize>(),
                        owner: self.id,
                        class: self.class,
                    });
                }
            }

            self.shot_time = Instant::now();
        }
    }
}
