use actix::{Actor, ActorContext, AsyncContext, Running, StreamHandler};
use actix_web_actors::ws;
use actix_web_actors::ws::{CloseCode, CloseReason};
use serenity::async_trait;
use tokio::sync::{RwLock, RwLockWriteGuard};

use crate::http::http_server::UserInfo;
use crate::system::game::GameManager;

use super::proto::{Handler, ProcessingError};

use std::sync::Arc;
use std::time::{Duration, Instant};

const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);
const CLIENT_TIMEOUT: Duration = Duration::from_secs(10);

pub struct WebSocketSession {
    pub game_manager: Arc<RwLock<GameManager>>,
    pub info: UserInfo,
    pub heartbeat: Instant,
}

impl WebSocketSession {
    pub fn new(info: UserInfo, game_manager: Arc<RwLock<GameManager>>) -> Self {
        Self {
            game_manager,
            info,
            heartbeat: Instant::now(),
        }
    }

    pub fn do_heartbeat(&self, ctx: &mut ws::WebsocketContext<Self>) {
        ctx.run_interval(HEARTBEAT_INTERVAL, |act, ctx| {
            if Instant::now().duration_since(act.heartbeat) > CLIENT_TIMEOUT {
                ctx.stop();
                return;
            }

            ctx.ping(b"");
        });
    }

    pub async fn handle_packet(&mut self, text: String) -> Result<String, ProcessingError> {
        <Self as Handler>::handle(self, text).await
    }
}

#[async_trait]
impl Handler for WebSocketSession {
    async fn fetch_user_info(&mut self) -> UserInfo {
        self.info.clone()
    }

    async fn get_game_manager<'a>(&'a mut self) -> RwLockWriteGuard<'a, GameManager> {
        self.game_manager.write().await
    }
}

impl Actor for WebSocketSession {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        self.do_heartbeat(ctx);
    }

    fn stopping(&mut self, _: &mut Self::Context) -> Running {
        Running::Stop
    }
}

/// WebSocket message handler
impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for WebSocketSession {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        let msg = match msg {
            Err(_) => {
                ctx.stop();
                return;
            }
            Ok(msg) => msg,
        };

        match msg {
            ws::Message::Ping(msg) => {
                self.heartbeat = Instant::now();
                ctx.pong(&msg);
            }
            ws::Message::Pong(_) => {
                self.heartbeat = Instant::now();
            }
            ws::Message::Text(text) => match futures::executor::block_on(self.handle_packet(text)) {
                Ok(str) => {
                    ctx.text(str);
                }
                Err(e) => match e {
                    ProcessingError::InvalidProtocol => {
                        ctx.close(Some(CloseReason::from(CloseCode::Unsupported)));
                    }
                    ProcessingError::NoOutput => {}
                },
            },
            ws::Message::Binary(_) => {
                ctx.close(Some(CloseReason::from(CloseCode::Unsupported)));
            }
            ws::Message::Close(reason) => {
                ctx.close(reason);
                ctx.stop();
            }
            ws::Message::Continuation(_) => {
                ctx.stop();
            }
            ws::Message::Nop => (),
        }
    }
}
