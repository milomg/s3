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
use std::cell::RefCell;
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
pub struct Transfer(pub usize, pub Addr<WsGameSession>, pub Player);

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
pub struct ClientBullet {
    pub vel: Vector2<f32>,
    pub pos: Vector2<f32>,
    pub id: usize,
}
#[derive(Serialize)]
pub struct ClientWormhole {
    pub pos: Vector2<f32>,
}
#[derive(Serialize)]
struct Playfield {
    players: Vec<ClientPlayer>,
    bullets: Vec<ClientBullet>,
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
    pub shot_time: Instant,
    pub split_time: Instant,
    pub class: Classes,
    pub name: String,
}
pub struct Bullet {
    pub vel: Vector2<f32>,
    pub pos: Vector2<f32>,
    pub spawn: Instant,
    pub class: Classes,
    pub id: usize,
    pub owner: usize,
}
struct Wormhole {
    pos: Vector2<f32>,
    addr: Addr<GameServer>,
}

impl Player {
    const RADIUS: f32 = 30.0;
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
    rng: RefCell<ThreadRng>,
    wormholes: Vec<Wormhole>,
    tick: usize,
}

impl GameServer {
    pub fn new() -> GameServer {
        let rng = rand::thread_rng();
        GameServer {
            sessions: HashMap::new(),
            players: HashMap::new(),
            bullets: Vec::new(),
            wormholes: Vec::new(),
            rng: RefCell::new(rng),
            tick: 0,
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
        for p in self.players.values_mut() {
            let acc = Vector2::new(p.angle.sin(), p.angle.cos());
            if p.split && (p.split_time.elapsed() > Duration::from_millis(600)) && p.mana > 100 {
                p.split_time = Instant::now();
                p.mana -= 100;
            }
            if p.split_time.elapsed() < Duration::from_millis(600) {
                p.vel += 1. * acc;
                p.vel *= 0.95;
            } else {
                p.vel += 0.6 * acc;
                p.vel *= 0.9;
            }
            p.pos += p.vel;
            p.pos.x = p.pos.x.max(0.0).min(800.0);
            p.pos.y = p.pos.y.max(0.0).min(800.0);
            if self.tick % 3 != 2 {
                p.health = p.health.saturating_add(1);
            }
            match (self.tick % 3, p.class) {
                (2, Classes::Quickshot) => (),
                (_, Classes::Quickshot) => p.mana = p.mana.saturating_add(1),
                (_, Classes::Sniper) => p.mana = p.mana.saturating_add(1),
            }

            if p.mouse
                && p.shot_time.elapsed()
                    > Duration::from_millis(match p.class {
                        Classes::Quickshot => 750,
                        Classes::Sniper => 1000,
                    })
            {
                let acopy = Vector2::new(acc.y, -acc.x);

                match p.class {
                    Classes::Quickshot => {
                        for i in (10..=12).step_by(1) {
                            let f = i as f32;
                            self.bullets.push(Bullet {
                                pos: p.pos.clone() - acopy * 35.0,
                                vel: acc * (f) - acopy * (11.0 - f),
                                spawn: Instant::now(),
                                id: self.rng.borrow_mut().gen::<usize>(),
                                owner: p.id,
                                class: p.class,
                            });
                            self.bullets.push(Bullet {
                                pos: p.pos.clone() + acopy * 35.0,
                                vel: acc * (f) + acopy * (11.0 - f),
                                spawn: Instant::now(),
                                id: self.rng.borrow_mut().gen::<usize>(),
                                owner: p.id,
                                class: p.class,
                            });
                        }
                    }
                    _ => {
                        self.bullets.push(Bullet {
                            pos: p.pos.clone() + acopy * 35.0,
                            vel: acc * 15.0 - acopy,
                            spawn: Instant::now(),
                            id: self.rng.borrow_mut().gen::<usize>(),
                            owner: p.id,
                            class: p.class,
                        });
                        self.bullets.push(Bullet {
                            pos: p.pos.clone() + acc * 35.0,
                            vel: acc * 15.0 - acc,
                            spawn: Instant::now(),
                            id: self.rng.borrow_mut().gen::<usize>(),
                            owner: p.id,
                            class: p.class,
                        });
                        self.bullets.push(Bullet {
                            pos: p.pos.clone() - acopy * 35.0,
                            vel: acc * 15.0 + acopy,
                            spawn: Instant::now(),
                            id: self.rng.borrow_mut().gen::<usize>(),
                            owner: p.id,
                            class: p.class,
                        });
                    }
                }

                p.shot_time = Instant::now();
            }
        }
        for b in self.bullets.iter_mut() {
            b.pos += b.vel;
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

        self.tick += 1;
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
        let id = self.rng.borrow_mut().gen::<usize>();
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

/// Handler for Connect message.
///
/// Register new session and assign unique id to this session
impl Handler<Transfer> for GameServer {
    type Result = ();

    fn handle(&mut self, msg: Transfer, _: &mut Context<Self>) -> Self::Result {
        self.players.insert(msg.0, msg.2);
        msg.1.do_send(Message(
            json!({
                "clear":true
            })
            .to_string(),
        ));
        self.sessions.insert(msg.0, msg.1);
    }
}

/// Handler for Connect message.
///
/// Register new session and assign unique id to this session
impl Handler<NewWormhole> for GameServer {
    type Result = ();

    fn handle(&mut self, msg: NewWormhole, _: &mut Context<Self>) -> Self::Result {
        let pos = Vector2::new(0.0, 0.0);
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
            let p = Player {
                id: msg.id,
                vel: Vector2::new(0.0, 0.0),
                pos: Vector2::new(400.0, 400.0),
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
