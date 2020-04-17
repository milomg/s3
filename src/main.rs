use ::actix::*;
use actix_files as fs;
use actix_web::*;
use actix_web_actors::ws;
use na::Vector2;
use nalgebra as na;
use serde_json::json;

mod boss;
mod bullet;
mod player;
mod server;

use server::{
    ClientMessage, Connect, DecodedMessage, Disconnect, GameServer, Message, TransferClient,
};

/// Entry point for our route
async fn game_route(
    req: HttpRequest,
    stream: web::Payload,
    srv: web::Data<Addr<server::GameServer>>,
) -> Result<HttpResponse> {
    ws::start(
        WsGameSession {
            id: 0,
            addr: srv.get_ref().clone(),
        },
        &req,
        stream,
    )
}

pub struct WsGameSession {
    /// unique session id
    id: usize,

    addr: Addr<server::GameServer>,
}

impl Actor for WsGameSession {
    type Context = ws::WebsocketContext<Self>;

    /// Method is called on actor start.
    /// We register ws session with GameServer
    fn started(&mut self, ctx: &mut Self::Context) {
        // register self in game server. `AsyncContext::wait` register
        // future within context, but context waits until this future resolves
        // before processing any other events.
        // HttpContext::state() is instance of WsGameSessionState, state is shared across all
        // routes within application
        let addr: Addr<_> = ctx.address();
        self.addr
            .send(Connect { addr })
            .into_actor(self)
            .then(|res, act, ctx| {
                match res {
                    Ok(res) => {
                        act.id = res;
                        ctx.text(
                            json!({
                                "you": act.id,
                                "pos": Vector2::new(400.0, 400.0),
                            })
                            .to_string(),
                        );
                    }
                    // something is wrong with game server
                    _ => ctx.stop(),
                }
                fut::ready(())
            })
            .wait(ctx);
    }

    fn stopping(&mut self, _: &mut Self::Context) -> Running {
        // notify game server
        self.addr.do_send(Disconnect { id: self.id });
        Running::Stop
    }
}

/// Handle messages from game server, we simply send it to peer WebSocket
impl Handler<Message> for WsGameSession {
    type Result = ();

    fn handle(&mut self, msg: Message, ctx: &mut Self::Context) {
        ctx.text(msg.0);
    }
}

impl Handler<TransferClient> for WsGameSession {
    type Result = ();

    fn handle(&mut self, msg: TransferClient, _: &mut Self::Context) {
        self.addr = msg.0;
    }
}

/// WebSocket message handler
impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for WsGameSession {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match msg {
            Ok(ws::Message::Ping(msg)) => ctx.pong(&msg),
            Ok(ws::Message::Pong(_)) => println!("Ping"),
            Ok(ws::Message::Text(text)) => {
                // All the client sends are key messages so we assume that the message is a key message
                if let Ok(m) = serde_json::from_str::<ClientMessage>(text.trim()) {
                    self.addr.do_send(DecodedMessage { id: self.id, m });
                }

                // send message to game server
            }
            Ok(ws::Message::Binary(_)) => println!("Unexpected binary"),
            Ok(ws::Message::Close(_)) => {
                ctx.stop();
            }
            _ => (),
        }
    }
}

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    // let sys = actix::System::new("ghist");

    // Start game server actor in separate thread
    let homeserver = GameServer::new(None, None).start();

    let bossserver =
        GameServer::new(Some(boss::BossType::NormalBoss), Some(homeserver.clone())).start();
    let bossserver2 =
        GameServer::new(Some(boss::BossType::HardcoreBoss), Some(homeserver.clone())).start();
    // Create a wormhole to the new server
    homeserver.do_send(server::NewWormhole(bossserver.clone(), 1));
    homeserver.do_send(server::NewWormhole(bossserver2.clone(), 2));

    let port = std::env::var("PORT").unwrap_or_else(|_| "8080".into());
    println!("Starting a server on http://localhost:{}", port);
    // Create Http server with WebSocket support
    HttpServer::new(move || {
        App::new()
            .data(homeserver.clone())
            .service(web::resource("/ws/").to(game_route))
            .service(fs::Files::new("/", "client/dist/").index_file("index.html"))
    })
    .bind(format!("0.0.0.0:{}", port))?
    .run()
    .await
}
