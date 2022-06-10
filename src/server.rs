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

use crate::boss::*;
use crate::bullet::*;
use crate::consts::*;
use crate::player::*;

/// New game session is created
#[derive(Message)]
#[rtype(usize)]
pub struct Connect {
    pub addr: Addr<WsGameSession>,
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct DecodedMessage {
    pub id: usize,
    pub m: ClientMessage,
}

/// Session is disconnected
#[derive(Message)]
#[rtype(result = "()")]
pub struct Disconnect {
    pub id: usize,
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct TransferClient(pub Addr<GameServer>);

#[derive(Message)]
#[rtype(result = "()")]
pub struct Message(pub String);

#[derive(Message)]
#[rtype(result = "()")]
struct Transfer(usize, Addr<WsGameSession>, Player);

#[derive(Message)]
#[rtype(result = "()")]
pub struct NewWormhole(pub Addr<GameServer>, pub u8);

#[derive(Deserialize)]
pub enum ClientMessage {
    Spawn(String, Classes),
    Target(Vector2<f32>),
    Click(bool),
    Split(bool),
    Join(bool),
    Escape(bool),
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
    color: u8,
}
#[derive(Serialize)]
struct ClientBoss {
    pos: Vector2<f32>,
    health: u8,
}
#[derive(Serialize)]
struct Playfield {
    players: Vec<ClientPlayer>,
    bullets: Vec<ClientBullet>,
    boss: Option<ClientBoss>,
}

struct Wormhole {
    pos: Vector2<f32>,
    color: u8,
    addr: Addr<GameServer>,
}

impl Wormhole {
    const RADIUS: f32 = 30.0;
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
    boss_dead: Instant,
    pvp_enabled: bool,
    home_server: Option<Addr<GameServer>>,
}

impl GameServer {
    pub fn new(boss: Option<BossType>, home_server: Option<Addr<GameServer>>) -> GameServer {
        let mut rng = rand::thread_rng();
        GameServer {
            sessions: HashMap::new(),
            players: HashMap::new(),
            bullets: Vec::new(),
            wormholes: Vec::new(),
            boss: if let Some(boss_type) = boss {
                Some(Boss {
                    pos: Vector2::new(rng.gen_range(0.0..WORLDSIZE), rng.gen_range(0.0..WORLDSIZE)),
                    vel: Vector2::new(0.0, 0.0),
                    health: 255,
                    shot_time: Instant::now(),
                    shot_time2: Instant::now(),
                    class: boss_type,
                })
            } else {
                None
            },
            pvp_enabled: true,
            boss_bullets: Vec::new(),
            rng,
            tick: Instant::now(),
            health_tick: Instant::now(),
            quickshot_mana: Instant::now(),
            sniper_mana: Instant::now(),
            boss_dead: Instant::now(),
            home_server,
        }
    }
    /// Send message to all players
    fn send_message(&self, message: &str) {
        for addr in self.sessions.values() {
            addr.do_send(Message(message.to_owned()));
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
        let ht = (self.health_tick.elapsed().as_millis() / 48) as u8;
        let qt = (self.quickshot_mana.elapsed().as_millis() / 24) as u8; // 2/3*1/16 of millis
        let st = (self.sniper_mana.elapsed().as_millis() / 16) as u8;
        if let Some(boss) = &mut self.boss {
            if boss.health > 0 {
                if !self.players.is_empty() {
                    boss.tick(
                        dt,
                        &mut self.rng,
                        &mut self.boss_bullets,
                        self.players.values().peekable(),
                    );
                }
            } else if self.boss_dead.elapsed() > Duration::from_millis(3000) {
                boss.pos = Vector2::new(
                    self.rng.gen_range(0.0..WORLDSIZE),
                    self.rng.gen_range(0.0..WORLDSIZE),
                );
                boss.vel = Vector2::new(0.0, 0.0);
                boss.health = 255;
                boss.shot_time = Instant::now();
                boss.shot_time2 = Instant::now();
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

        self.health_tick += Duration::from_millis(ht as u64 * 48);
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

        self.escape_players();

        self.bullets.retain(|b| {
            b.spawn.elapsed()
                < Duration::from_millis(match b.class {
                    Classes::Quickshot => 600,
                    _ => 1000,
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
                    angle: p.target.x.atan2(p.target.y),
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
            boss: self.boss.as_ref().and_then(|b| {
                if b.health > 0 {
                    Some(ClientBoss {
                        pos: b.pos,
                        health: b.health,
                    })
                } else {
                    None
                }
            }),
        };
        let serialized = ::serde_json::to_string(&playfield).unwrap();
        self.send_message(&serialized);
    }
    fn escape_players(&mut self) {
        let mut escapers = Vec::new();
        for (i, p) in self.players.iter() {
            if let Some(time) = p.escape_time {
                if time.elapsed() > Duration::from_secs(1) {
                    escapers.push(*i)
                }
            }
        }
        if let Some(hs) = &self.home_server {
            for escaper in escapers {
                if let Some(p) = self.players.remove(&escaper) {
                    if let Some(a) = self.sessions.remove(&escaper) {
                        a.do_send(TransferClient(hs.clone()));
                        self.send_message(
                            &json!({
                                "death": p.id,
                            })
                            .to_string(),
                        );
                        hs.do_send(Transfer(p.id, a, p));
                    }
                }
            }
        }
    }
    fn collision_trees(&mut self) {
        let pt = RTree::bulk_load(self.players.values().collect());

        let mut move_players = Vec::new();
        for w in &self.wormholes {
            let mut wv = Vec::new();
            let intersecting = pt.locate_in_envelope_intersecting(&(w).envelope());
            for intersect in intersecting {
                if intersect.join
                    && (intersect.pos - w.pos).magnitude()
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
        let dbt = RTree::bulk_load(self.boss_bullets.iter().map(|b| b).collect());

        let mut health_map = HashMap::new();
        let mut health_add = HashMap::new();
        let mut delete_bullets = HashSet::new();
        let mut delete_boss_bullets = HashSet::new();
        if let Some(boss) = &mut self.boss {
            if boss.health > 0 {
                let intersecting = dt.locate_in_envelope_intersecting(&(&*boss).envelope());
                for intersect in intersecting {
                    if (intersect.pos - boss.pos).magnitude()
                        <= (Boss::RADIUS + Bullet::RADIUS).powf(2.0)
                    {
                        boss.health = boss.health.saturating_sub(8);
                        *health_add.entry(intersect.owner).or_insert(0) += 4;

                        delete_bullets.insert(intersect.id);
                    }
                }
                if boss.health == 0 {
                    self.boss_dead = Instant::now();
                }
            }
        }
        for (i, p) in &self.players {
            if self.pvp_enabled {
                let intersecting = dt.locate_in_envelope_intersecting(&(p).envelope());
                for intersect in intersecting {
                    if intersect.owner != p.id
                        && (intersect.pos - p.pos).magnitude()
                            <= (Player::RADIUS + Bullet::RADIUS).powf(2.0)
                    {
                        *health_map.entry(*i).or_insert(0) += 8;
                        *health_add.entry(intersect.owner).or_insert(0) += 4;

                        delete_bullets.insert(intersect.id);
                    }
                }
            }
            let intersecting = dbt.locate_in_envelope_intersecting(&(p).envelope());
            for intersect in intersecting {
                if (intersect.pos - p.pos).magnitude()
                    <= (Player::RADIUS + BossBullet::RADIUS).powf(2.0)
                {
                    if let Some(boss) = &mut self.boss {
                        if boss.health > 0 {
                            boss.health = boss.health.saturating_add(20);
                        }
                    }
                    *health_map.entry(*i).or_insert(0) += 50;

                    delete_boss_bullets.insert(intersect.id);
                }
            }
        }

        for (i, h) in &health_map {
            self.players
                .entry(*i)
                .and_modify(|p| p.health = p.health.saturating_sub(*h));
        }
        for (i, h) in &health_add {
            self.players
                .entry(*i)
                .and_modify(|p| p.health = p.health.saturating_add(*h));
        }
        self.bullets.retain(|b| !delete_bullets.contains(&b.id));
        self.boss_bullets
            .retain(|b| !delete_boss_bullets.contains(&b.id));
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
                    + &::serde_json::to_string(&ClientWormhole {
                        pos: w.pos,
                        color: w.color,
                    })
                    .unwrap()
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
                "clear": true
            })
            .to_string(),
        ));
        for w in &self.wormholes {
            msg.1.do_send(Message(
                json!({
                    "wormhole": ClientWormhole {
                        pos: w.pos,
                        color: w.color,
                    }
                })
                .to_string(),
            ));
        }
        self.sessions.insert(msg.0, msg.1);
    }
}

impl Handler<NewWormhole> for GameServer {
    type Result = ();

    fn handle(&mut self, msg: NewWormhole, _: &mut Context<Self>) -> Self::Result {
        let b1 = if self.rng.gen::<bool>() {
            WORLDSIZE
        } else {
            0.0
        };
        let b2 = self.rng.gen::<bool>();
        let pos = self.rng.gen_range(0.0..WORLDSIZE);
        let pos = Vector2::new(if b2 { b1 } else { pos }, if b2 { pos } else { b1 });
        self.wormholes.push(Wormhole {
            pos,
            addr: msg.0,
            color: msg.1,
        });

        self.send_message(
            &("{\"wormhole\":".to_owned()
                + &::serde_json::to_string(&ClientWormhole { pos, color: msg.1 }).unwrap()
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
            let x = self.rng.gen_range(0.0..WORLDSIZE);
            let y = self.rng.gen_range(0.0..WORLDSIZE);

            let p = Player {
                id: msg.id,
                vel: Vector2::new(0.0, 0.0),
                pos: Vector2::new(x, y),
                shot_time: Instant::now() - Duration::from_secs(2),
                split_time: Instant::now() - Duration::from_secs(2),
                escape_time: None,
                target: Vector2::new(0.0, 0.0),
                health: 255,
                mana: 255,
                name: n,
                class: c,
                mouse: false,
                split: false,
                join: false,
            };
            self.players.insert(msg.id, p);
        } else if let Some(p) = self.players.get_mut(&msg.id) {
            match msg.m {
                ClientMessage::Click(b) => p.mouse = b,
                ClientMessage::Split(b) => p.split = b,
                ClientMessage::Target(v) => p.target = v,
                ClientMessage::Escape(b) => {
                    if !b {
                        p.escape_time = None
                    } else if p.escape_time.is_none() {
                        p.escape_time = Some(Instant::now())
                    }
                }
                ClientMessage::Join(b) => p.join = b,
                ClientMessage::Spawn(_, _) => unreachable!(),
            }
        }
    }
}
