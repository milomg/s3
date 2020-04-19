use crate::bullet::Bullet;
use crate::consts::WORLDSIZE;
use na::Vector2;
use nalgebra as na;
use rand::prelude::*;
use rstar::{RTreeObject, AABB};
use serde_derive::{Deserialize, Serialize};
use std::f32::consts::PI;
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
    pub target: Vector2<f32>,
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

fn sunflower(n: u8) -> impl Iterator<Item = Vector2<f32>> {
    let golden = PI * (3.0 - (5.0f32).sqrt());
    (0..n).map(move |k| {
        let r = (k as f32).sqrt() / (n as f32).sqrt();
        let theta = (k as f32) * golden;
        Vector2::new(r * theta.cos(), r * theta.sin())
    })
}

impl Player {
    pub const RADIUS: f32 = 35.0;
    pub fn tick(&mut self, dt: f32, rng: &mut ThreadRng, bullets: &mut Vec<Bullet>) {
        let acc = self.target.try_normalize(1.0e-6).unwrap_or_else(Vector2::y);
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
        self.pos.x = self.pos.x.max(0.0).min(WORLDSIZE);
        self.pos.y = self.pos.y.max(0.0).min(WORLDSIZE);

        if self.mouse
            && self.shot_time.elapsed()
                > Duration::from_millis(match self.class {
                    Classes::Quickshot => 750,
                    Classes::Sniper => 1000,
                })
        {
            match self.class {
                Classes::Quickshot => {
                    let btarget = self.pos + acc * self.target.magnitude().max(100.0);
                    for pos in sunflower(self.health / 10) {
                        let bpos = self.pos + 50.0 * pos;
                        let rvec = Vector2::new(rng.gen_range(-8.0, 8.0), rng.gen_range(-8.0, 8.0));
                        bullets.push(Bullet {
                            pos: bpos,
                            vel: (btarget - bpos + rvec).normalize() * 15.0,
                            spawn: Instant::now(),
                            id: rng.gen::<usize>(),
                            owner: self.id,
                            class: self.class,
                        });
                    }
                }
                _ => {
                    let wide = self.target.magnitude().max(100.0).min(600.0);
                    let totalbullets = self.health as i16 / 51 + 5;
                    for i in (-totalbullets)..=(totalbullets) {
                        let angle = self.target.y.atan2(self.target.x) + (i as f32 / totalbullets as f32) * PI / 2.0;
                        let circle = Vector2::new(angle.cos(), angle.sin());
                        bullets.push(Bullet {
                            pos: self.pos + circle * 50.0,
                            vel: (acc * (wide - 100.0) + circle * (600.0 - wide) / 20.0).normalize() * 15.0,
                            spawn: Instant::now(),
                            id: rng.gen::<usize>(),
                            owner: self.id,
                            class: self.class,
                        });
                    }
                }
            }

            self.shot_time = Instant::now();
        }
    }
}
