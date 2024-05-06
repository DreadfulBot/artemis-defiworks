use common::{ReportError, Reporter};
use teloxide::{requests::Requester, types::ChatId, Bot};

pub struct TelegramBot {
    bot: Bot,
}

impl TelegramBot {
    pub fn new(token: &str) -> Self {
        let bot = Bot::new(token);
        TelegramBot { bot }
    }
}

impl Reporter for TelegramBot {
    async fn report(&self, target: i64, text: String) -> Result<(), ReportError> {
        let chat_id = ChatId(target);
        self.bot
            .send_message(chat_id, text)
            .await
            .map(|_| ())
            .map_err(ReportError::Telegram)
    }
}
