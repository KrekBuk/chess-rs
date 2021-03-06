use actix::{Actor, ActorContext, AsyncContext, Handler as ActixHandler, Message, Running, StreamHandler};
use actix_web_actors::ws;
use actix_web_actors::ws::{CloseCode, CloseReason};
use serenity::async_trait;
use tokio::sync::{RwLock, RwLockWriteGuard};

use crate::http::http_server::UserInfo;
use crate::system::game::GameManager;

use super::proto::{Handler, ProcessingError};

use crate::http::proto::make_state;
use serenity::model::id::UserId;
use std::sync::Arc;
use std::time::{Duration, Instant};

pub struct UnauthorizedWebSocketSession {}

impl Actor for UnauthorizedWebSocketSession {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        ctx.close(Some(CloseReason::from(CloseCode::from(4000))));
    }

    fn stopping(&mut self, _: &mut Self::Context) -> Running {
        Running::Stop
    }
}

impl Default for UnauthorizedWebSocketSession {
    fn default() -> Self {
        Self {}
    }
}

const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);
const CLIENT_TIMEOUT: Duration = Duration::from_secs(10);

pub struct WebSocketSession {
    pub game_manager: Arc<RwLock<GameManager>>,
    pub info: Option<UserInfo>,
    pub heartbeat: Instant,
}

impl WebSocketSession {
    pub fn new(info: Option<UserInfo>, game_manager: Arc<RwLock<GameManager>>) -> Self {
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

    pub async fn handle_packet(&mut self, text: String) -> Result<Option<String>, ProcessingError> {
        <Self as Handler>::handle(self, text).await
    }

    fn block_for_manager(&self) -> RwLockWriteGuard<'_, GameManager> {
        futures::executor::block_on(self.game_manager.write())
    }

    fn do_handle_packet(&mut self, text: String, ctx: &mut <WebSocketSession as Actor>::Context) {
        match futures::executor::block_on(self.handle_packet(text)) {
            Ok(str) => {
                if let Some(str) = str {
                    ctx.text(str);
                }
            }
            Err(e) => match e {
                ProcessingError::InvalidProtocol => {
                    ctx.close(Some(CloseReason::from(CloseCode::Unsupported)));
                }
                ProcessingError::OldState => {}
                ProcessingError::NoOutput => {}
            },
        }
    }
}

#[async_trait]
impl Handler for WebSocketSession {
    async fn fetch_user_info(&mut self) -> UserInfo {
        self.info.as_ref().unwrap().clone()
    }

    #[allow(clippy::needless_lifetimes)] // clippy bug ?
    async fn get_game_manager<'a>(&'a mut self) -> RwLockWriteGuard<'a, GameManager> {
        self.game_manager.write().await
    }
}

impl Actor for WebSocketSession {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        self.do_heartbeat(ctx);

        if self.info.is_none() {
            ctx.close(Some(CloseReason::from(CloseCode::from(4000))));
            return;
        };

        self.block_for_manager().register_socket(ctx.address());
    }

    fn stopping(&mut self, ctx: &mut Self::Context) -> Running {
        self.block_for_manager().unregister_socket(ctx.address());

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
            ws::Message::Text(text) => {
                if self.info.is_none() {
                    ctx.close(Some(CloseReason::from(CloseCode::from(4000))));
                    return;
                }

                self.do_handle_packet(text, ctx);
            }
            ws::Message::Binary(_) => {
                ctx.close(Some(CloseReason::from(CloseCode::Unsupported)));
            }
            ws::Message::Close(_) => {
                ctx.stop();
            }
            ws::Message::Continuation(_) => {
                ctx.stop();
            }
            ws::Message::Nop => (),
        }
    }
}

#[derive(Clone)]
pub struct UpdateGameStateMessage {
    pub viewer_list: Vec<UserId>,
}

impl Message for UpdateGameStateMessage {
    type Result = ();
}

impl ActixHandler<UpdateGameStateMessage> for WebSocketSession {
    type Result = ();

    fn handle(&mut self, msg: UpdateGameStateMessage, ctx: &mut Self::Context) -> Self::Result {
        match &self.info {
            Some(info) => {
                if !msg.viewer_list.contains(&info.id) {
                    return;
                }
            }
            None => {
                return;
            }
        }

        match &self.info {
            Some(info) => {
                let mut game_manager = self.block_for_manager();
                ctx.text(make_state(&info, &game_manager.get_game(info.id)));
            }
            None => {
                ctx.close(Some(CloseReason::from(CloseCode::from(4000))));
            }
        }
    }
}
