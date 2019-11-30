//! `GameServer` is an actor. It maintains list of connection client session.
//!  Peers send messages to other peers through `GameServer`.
use crate::WsGameSession;
use actix::prelude::*;
use na::Vector2;
use nalgebra as na;
use rand::prelude::*;
use rstar::{RTree, RTreeObject, AABB};
use serde_derive::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::collections::HashSet;
use std::time::{Duration, Instant};

/// New game session is created
#[derive(Message)]
#[rtype(usize)]
pub struct Connect {
    pub addr: Addr<WsGameSession>,
}

#[derive(Message)]
pub struct DecodedMessage {
    pub id: usize,
    pub m: ClientMessage,
}

/// Session is disconnected
#[derive(Message)]
pub struct Disconnect {
    pub id: usize,
}

#[derive(Message)]
pub struct TransferClient(pub Addr<GameServer>);

#[derive(Message)]
pub struct Message(pub String);

#[derive(Message)]
struct Transfer(usize, Addr<WsGameSession>, Player);

#[derive(Message)]
pub struct NewWormhole(pub Addr<GameServer>);

#[derive(Deserialize, Serialize, Copy, Clone)]
pub enum Classes {
    Sniper,
    Quickshot,
}

#[derive(Deserialize)]
pub enum ClientMessage {
    Spawn(String, Classes),
    Angle(f32),
    Click(bool),
    Split(bool),
}

#[derive(Serialize)]
struct ClientPlayer {
    id: usize,
    pos: Vector2<f32>,
    name: String,
    angle: f32,
    health: u8,
    mana: u8,
    class: Classes,
    shot_time: u128,
}
#[derive(Serialize)]
struct ClientBullet {
    vel: Vector2<f32>,
    pos: Vector2<f32>,
    id: usize,
}
#[derive(Serialize)]
struct ClientWormhole {
    pos: Vector2<f32>,
}
#[derive(Serialize)]
struct Playfield {
    players: Vec<ClientPlayer>,
    bullets: Vec<ClientBullet>,
}

struct Player {
    id: usize,
    vel: Vector2<f32>,
    pos: Vector2<f32>,
    angle: f32,
    health: u8,
    mana: u8,
    mouse: bool,
    split: bool,
    shot_time: Instant,
    split_time: Instant,
    class: Classes,
    name: String,
}
struct Bullet {
    vel: Vector2<f32>,
    pos: Vector2<f32>,
    spawn: Instant,
    class: Classes,
    id: usize,
    owner: usize,
}
struct BossBullet {
    vel: Vector2<f32>,
    pos: Vector2<f32>,
    spawn: Instant,
    id: usize,
}
struct Wormhole {
    pos: Vector2<f32>,
    addr: Addr<GameServer>,
}
struct Boss {
    pos: Vector2<f32>,
    pub shot_time: Instant,
}

impl Player {
    const RADIUS: f32 = 30.0;
    fn tick(&mut self, dt: f32, rng: &mut ThreadRng, bullets: &mut Vec<Bullet>) {
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
                            pos: self.pos.clone() - acopy * 35.0,
                            vel: acc * (f) - acopy * (11.0 - f),
                            spawn: Instant::now(),
                            id: rng.gen::<usize>(),
                            owner: self.id,
                            class: self.class,
                        });
                        bullets.push(Bullet {
                            pos: self.pos.clone() + acopy * 35.0,
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
                        pos: self.pos.clone() + acopy * 35.0,
                        vel: acc * 15.0 - acopy,
                        spawn: Instant::now(),
                        id: rng.gen::<usize>(),
                        owner: self.id,
                        class: self.class,
                    });
                    bullets.push(Bullet {
                        pos: self.pos.clone() + acc * 35.0,
                        vel: acc * 15.0 - acc,
                        spawn: Instant::now(),
                        id: rng.gen::<usize>(),
                        owner: self.id,
                        class: self.class,
                    });
                    bullets.push(Bullet {
                        pos: self.pos.clone() - acopy * 35.0,
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
impl Bullet {
    const RADIUS: f32 = 10.0;
}
impl Wormhole {
    const RADIUS: f32 = 30.0;
}

impl PartialEq for Bullet {
    fn eq(&self, other: &Bullet) -> bool {
        self.id == other.id
    }
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
impl<'a> RTreeObject for &'a Wormhole {
    type Envelope = AABB<[f32; 2]>;

    fn envelope(&self) -> Self::Envelope {
        let size = Wormhole::RADIUS;
        AABB::from_corners(
            [self.pos.x - size, self.pos.y - size],
            [self.pos.x + size, self.pos.y + size],
        )
    }
}

/// `GameServer` responsible for coordinating game sessions.
/// implementation is super primitive
pub struct GameServer {
    sessions: HashMap<usize, Addr<WsGameSession>>,
    players: HashMap<usize, Player>,
    bullets: Vec<Bullet>,
    boss: Option<Boss>,
    boss_bullets: Vec<BossBullet>,
    rng: ThreadRng,
    wormholes: Vec<Wormhole>,
    tick: Instant,
    health_tick: Instant,
    quickshot_mana: Instant,
    sniper_mana: Instant,
}

impl GameServer {
    pub fn new() -> GameServer {
        let rng = rand::thread_rng();
        GameServer {
            sessions: HashMap::new(),
            players: HashMap::new(),
            bullets: Vec::new(),
            wormholes: Vec::new(),
            boss: None,
            boss_bullets: Vec::new(),
            rng: rng,
            tick: Instant::now(),
            health_tick: Instant::now(),
            quickshot_mana: Instant::now(),
            sniper_mana: Instant::now(),
        }
    }
    /// Send message to all players
    fn send_message(&self, message: &str) {
        for addr in self.sessions.values() {
            let _ = addr.do_send(Message(message.to_owned()));
        }
    }
    fn tick(&self, ctx: &mut Context<Self>) {
        ctx.run_later(Duration::from_millis(16), |act, ctx| {
            act.move_and_things();

            act.send_to_players();

            act.tick(ctx);
        });
    }
    fn move_and_things(&mut self) {
        let dt = self.tick.elapsed().as_millis() as f32 / 16.0;
        let ht = (self.health_tick.elapsed().as_millis() / 24) as u8;
        let qt = (self.quickshot_mana.elapsed().as_millis() / 24) as u8; // 2/3*1/16 of millis
        let st = (self.sniper_mana.elapsed().as_millis() / 16) as u8;
        if let Some(boss) = &mut self.boss {
            if self.players.len() > 0 {
                let mut nearest_player = self.players.values().next().unwrap();
                let mut nearest_dist = std::f32::MAX;
                for p in self.players.values() {
                    let dist = (boss.pos - p.pos).magnitude();
                    if dist < nearest_dist {
                        nearest_dist = dist;
                        nearest_player = p;
                    }
                }
                boss.pos += (nearest_player.pos - boss.pos).normalize();

                if boss.shot_time.elapsed() > Duration::from_millis(500) {
                    self.boss_bullets.push(BossBullet {
                        pos: boss.pos,
                        id: self.rng.gen::<usize>(),
                        spawn: Instant::now(),
                        vel: (nearest_player.pos - boss.pos).normalize() * 10.0,
                    });
                    boss.shot_time = Instant::now();
                }
            }
        }
        for p in self.players.values_mut() {
            p.tick(dt, &mut self.rng, &mut self.bullets);

            p.health = p.health.saturating_add(ht);

            p.mana = p.mana.saturating_add(match p.class {
                Classes::Quickshot => qt,
                Classes::Sniper => st,
            });
        }

        self.health_tick += Duration::from_millis(ht as u64 * 24);
        self.quickshot_mana += Duration::from_millis(qt as u64 * 24);
        self.sniper_mana += Duration::from_millis(st as u64 * 16);

        for b in self.bullets.iter_mut() {
            b.pos += b.vel * dt;
        }
        for b in self.boss_bullets.iter_mut() {
            b.pos += b.vel * dt;
        }

        self.collision_trees();

        self.reap_players();

        self.bullets.retain(|b| {
            b.spawn.elapsed()
                < Duration::from_millis(match b.class {
                    Classes::Quickshot => 600,
                    _ => 1250,
                })
        });

        self.boss_bullets
            .retain(|b| b.spawn.elapsed() < Duration::from_millis(1250));

        self.tick = Instant::now();
    }
    fn send_to_players(&self) {
        let playfield = Playfield {
            players: self
                .players
                .iter()
                .map(|(_, p)| ClientPlayer {
                    id: p.id,
                    pos: p.pos,
                    angle: p.angle,
                    health: p.health,
                    mana: p.mana,
                    class: p.class,
                    name: (*p.name).to_string(),
                    shot_time: p.shot_time.elapsed().as_millis(),
                })
                .collect(),
            bullets: self
                .bullets
                .iter()
                .map(|b| ClientBullet {
                    pos: b.pos,
                    vel: b.vel,
                    id: b.id,
                })
                .chain(self.boss_bullets.iter().map(|b| ClientBullet {
                    pos: b.pos,
                    vel: b.vel,
                    id: b.id,
                }))
                .collect(),
        };
        let serialized = ::serde_json::to_string(&playfield).unwrap();
        self.send_message(&serialized);
    }
    fn collision_trees(&mut self) {
        let pt = RTree::bulk_load(self.players.values().collect());

        let mut move_players = Vec::new();
        for w in &self.wormholes {
            let mut wv = Vec::new();
            let intersecting = pt.locate_in_envelope_intersecting(&(w).envelope());
            for intersect in intersecting {
                if (intersect.pos - w.pos).magnitude()
                    <= (Player::RADIUS + Wormhole::RADIUS).powf(2.0)
                {
                    wv.push(intersect.id);
                }
            }
            move_players.push(wv);
        }
        for (i, pl) in move_players.iter().enumerate() {
            for pi in pl {
                if let Some(p) = self.players.remove(pi) {
                    if let Some(a) = self.sessions.remove(pi) {
                        a.do_send(TransferClient(self.wormholes[i].addr.clone()));
                        self.send_message(
                            &json!({
                                "death": p.id,
                            })
                            .to_string(),
                        );
                        self.wormholes[i].addr.do_send(Transfer(p.id, a, p));
                    }
                }
            }
        }

        let dt = RTree::bulk_load(self.bullets.iter().map(|b| b).collect());

        let mut health_map = HashMap::new();
        let mut delete_bullets = HashSet::new();
        for (i, p) in &self.players {
            let intersecting = dt.locate_in_envelope_intersecting(&(p).envelope());
            for intersect in intersecting {
                if intersect.owner != p.id
                    && (intersect.pos - p.pos).magnitude()
                        <= (Player::RADIUS + Bullet::RADIUS).powf(2.0)
                {
                    *health_map.entry(*i).or_insert(0) += match intersect.class {
                        Classes::Sniper => 60,
                        Classes::Quickshot => 30,
                    };

                    delete_bullets.insert(intersect.id);
                }
            }
        }

        for (i, h) in &health_map {
            self.players
                .entry(*i)
                .and_modify(|p| p.health = p.health.saturating_sub(*h));
        }
        self.bullets.retain(|b| !delete_bullets.contains(&b.id));
    }
    fn reap_players(&mut self) {
        let mut delete = Vec::new();
        self.players.retain(|i, p| {
            if p.health == 0 {
                delete.push(*i);
                false
            } else {
                true
            }
        });
        for p in &delete {
            self.send_message(
                &json!({
                    "death": p,
                })
                .to_string(),
            )
        }
    }
}

/// Make actor from `GameServer`
impl Actor for GameServer {
    /// We are going to use simple Context, we just need ability to communicate
    /// with other actors.
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        self.tick(ctx);
    }
}

/// Handler for Connect message.
///
/// Register new session and assign unique id to this session
impl Handler<Connect> for GameServer {
    type Result = usize;

    fn handle(&mut self, msg: Connect, _: &mut Context<Self>) -> Self::Result {
        // register session with random id
        let id = self.rng.gen::<usize>();
        for w in &self.wormholes {
            msg.addr.do_send(Message(
                "{\"wormhole\":".to_owned()
                    + &::serde_json::to_string(&ClientWormhole { pos: w.pos }).unwrap()
                    + "}",
            ));
        }
        self.sessions.insert(id, msg.addr);

        // send id back
        id
    }
}

impl Handler<Transfer> for GameServer {
    type Result = ();

    fn handle(&mut self, msg: Transfer, _: &mut Context<Self>) -> Self::Result {
        // msg.2.mana = 255;
        // msg.2.health = 255;
        // msg.2.shot_time = Instant::now() - Duration::from_secs(2);
        // msg.2.split_time = Instant::now() - Duration::from_secs(2);

        self.players.insert(msg.0, msg.2);
        msg.1.do_send(Message(
            json!({
                "clear":true
            })
            .to_string(),
        ));
        for w in &self.wormholes {
            msg.1.do_send(Message(
                "{\"wormhole\":".to_owned()
                    + &::serde_json::to_string(&ClientWormhole { pos: w.pos }).unwrap()
                    + "}",
            ));
        }
        self.sessions.insert(msg.0, msg.1);
    }
}

impl Handler<NewWormhole> for GameServer {
    type Result = ();

    fn handle(&mut self, msg: NewWormhole, _: &mut Context<Self>) -> Self::Result {
        let b1 = if self.rng.gen::<bool>() { 800.0 } else { 0.0 };
        let b2 = self.rng.gen::<bool>();
        let pos = self.rng.gen_range(0.0, 800.0);
        let pos = Vector2::new(if b2 { b1 } else { pos }, if b2 { pos } else { b1 });
        self.wormholes.push(Wormhole {
            pos: pos,
            addr: msg.0,
        });

        self.send_message(
            &("{\"wormhole\":".to_owned()
                + &::serde_json::to_string(&ClientWormhole { pos: pos }).unwrap()
                + "}"),
        )
    }
}

/// Handler for Disconnect message.
impl Handler<Disconnect> for GameServer {
    type Result = ();

    fn handle(&mut self, msg: Disconnect, _: &mut Context<Self>) {
        // remove address
        self.sessions.remove(&msg.id);
        self.players.remove(&msg.id);
        self.send_message(
            &json!({
                "death": msg.id
            })
            .to_string(),
        );
    }
}

impl Handler<DecodedMessage> for GameServer {
    type Result = ();

    fn handle(&mut self, msg: DecodedMessage, _: &mut Context<Self>) {
        if let ClientMessage::Spawn(n, c) = msg.m {
            let x = self.rng.gen_range(0.0, 800.0);
            let y = self.rng.gen_range(0.0, 800.0);

            let p = Player {
                id: msg.id,
                vel: Vector2::new(0.0, 0.0),
                pos: Vector2::new(x, y),
                shot_time: Instant::now() - Duration::from_secs(2),
                split_time: Instant::now() - Duration::from_secs(2),
                angle: 0.0,
                health: 255,
                mana: 255,
                name: n,
                class: c,
                mouse: false,
                split: false,
            };
            self.players.insert(msg.id, p);
        } else {
            if let Some(p) = self.players.get_mut(&msg.id) {
                match msg.m {
                    ClientMessage::Click(b) => p.mouse = b,
                    ClientMessage::Split(b) => p.split = b,
                    ClientMessage::Angle(a) => p.angle = a,
                    ClientMessage::Spawn(_, _) => unreachable!(),
                }
            }
        }
    }
}
