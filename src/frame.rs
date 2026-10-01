use crate::identity::Identity;

pub fn render(message_id: &str, sender: &Identity, text: &str) -> String {
    format!("<cross-session-message id=\"{message_id}\" from=\"{}\">\n{text}\n</cross-session-message>", sender.address())
}

#[cfg(test)]
#[path = "tests/frame/mod.rs"]
mod tests;
