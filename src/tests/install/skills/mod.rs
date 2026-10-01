mod config_dir_default;
mod config_dir_env;
mod describe_skipped;
mod describe_written;
mod embedded_skills;
mod remove_absent;
mod remove_deletes;
mod remove_keeps_other_files;
mod render_plain_path;
mod render_spaced_path;
mod write_installs;
mod write_skips_missing;

use super::*;

const EXE: &str = "C:\\tools\\relay\\relay.exe";
