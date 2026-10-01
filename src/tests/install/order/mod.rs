mod install_stops_at_a_failed_service;
mod uninstall_elevation_refused;
mod uninstall_other_outcomes;

use std::cell::Cell;

use anyhow::anyhow;

use super::*;
