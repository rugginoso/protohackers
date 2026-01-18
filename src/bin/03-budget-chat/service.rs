use std::{
    convert::Infallible,
    future::{Ready, ready},
    task::Poll,
};

use tower::Service;

use crate::username::Username;

#[derive(Debug, Clone)]
pub enum BudgetChatEvent {
    Join { username: Username },
    Leave { username: Username },
    Message { username: Username, content: String },
}

impl BudgetChatEvent {
    pub fn is_from(&self, username: &Username) -> bool {
        let msg_username = match self {
            Self::Join { username } => username,
            Self::Leave { username } => username,
            Self::Message {
                username,
                content: _,
            } => username,
        };

        msg_username == username
    }
}

pub struct BudgetChatService {
    username: Username,
}

impl BudgetChatService {
    pub fn new(username: Username) -> Self {
        Self { username }
    }
}

impl Service<String> for BudgetChatService {
    type Response = BudgetChatEvent;
    type Error = Infallible;
    type Future = Ready<Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, _cx: &mut std::task::Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: String) -> Self::Future {
        ready(Ok(BudgetChatEvent::Message {
            username: self.username.clone(),
            content: req,
        }))
    }
}

#[derive(Clone)]
pub struct BudgetChatServiceFactory;

impl Service<Username> for BudgetChatServiceFactory {
    type Response = BudgetChatService;
    type Error = Infallible;
    type Future = Ready<Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, _cx: &mut std::task::Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, username: Username) -> Self::Future {
        ready(Ok(BudgetChatService::new(username)))
    }
}

#[cfg(test)]
mod tests {}
