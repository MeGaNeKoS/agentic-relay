pub mod claude;
pub mod codex;

pub fn norm_cwd(s: &str) -> String {
    s.replace('\\', "/").trim_end_matches('/').to_lowercase()
}

#[cfg(test)]
#[path = "../tests/transport/mod.rs"]
mod tests;
