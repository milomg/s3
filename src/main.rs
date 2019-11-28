use ::actix::*;
use actix_files as fs;
use actix_web::*;
use actix_web_actors::ws;
use na::Vector2;
use nalgebra as na;
use serde_json::json;

mod server;

use server::{
    ClientMessage, Connect, DecodedMessage, Disconnect, GameServer, Message, TransferClient,
};

/// Entry point for our route
fn game_route(
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
            .send(Connect { addr: addr })
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
                fut::ok(())
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
impl StreamHandler<ws::Message, ws::ProtocolError> for WsGameSession {
    fn handle(&mut self, msg: ws::Message, ctx: &mut Self::Context) {
        match msg {
            ws::Message::Ping(msg) => ctx.pong(&msg),
            ws::Message::Pong(_) => println!("Ping"),
            ws::Message::Text(text) => {
                // All the client sends are key messages so we assume that the message is a key message
                if let Ok(m) = serde_json::from_str::<ClientMessage>(text.trim()) {
                    self.addr.do_send(DecodedMessage { id: self.id, m });
                }

                // send message to game server
            }
            ws::Message::Binary(_) => println!("Unexpected binary"),
            ws::Message::Close(_) => {
                ctx.stop();
            }
            ws::Message::Nop => (),
        }
    }
}

fn main() -> Result<(), failure::Error> {
    let sys = actix::System::new("ghist");

    // Start game server actor in separate thread
    let server = GameServer::new().start();

    let server2 = GameServer::new().start();
    server.do_send(server::NewWormhole(server2));

    // Create Http server with WebSocket support
    HttpServer::new(move || {
        App::new()
            .data(server.clone())
            .service(web::resource("/ws/").to(game_route))
            // static resources
            .service(fs::Files::new("/", "client/dist/").index_file("index.html"))
    })
    .bind("0.0.0.0:8080")
    .unwrap()
    .start();

    println!("Started http server: http://localhost:8080");
    let _ = sys.run();
    Ok(())
}
