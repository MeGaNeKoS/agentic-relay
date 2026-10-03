use crate::identity::Identity;

pub fn render(message_id: &str, sender: &Identity, from_name: Option<&str>, text: &str) -> String {
    let from_name = from_name.map(|name| format!(" from-name=\"{name}\"")).unwrap_or_default();
    format!("<cross-session-message id=\"{message_id}\" from=\"{}\"{from_name}>\n{text}\n</cross-session-message>", sender.address())
}

#[cfg(test)]
#[path = "tests/frame/mod.rs"]
mod tests;
