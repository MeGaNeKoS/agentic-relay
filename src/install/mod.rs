#[cfg(windows)]
mod windows;
#[cfg(target_os = "linux")]
mod systemd;
#[cfg(target_os = "macos")]
mod launchd;
mod skills;

use anyhow::Result;

fn install_service() -> Result<()> {
    #[cfg(windows)]
    return windows::install();
    #[cfg(target_os = "linux")]
    return systemd::install();
    #[cfg(target_os = "macos")]
    return launchd::install();
    #[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
    anyhow::bail!("no install support for this platform");
}

fn uninstall_service() -> Result<()> {
    #[cfg(windows)]
    return windows::uninstall();
    #[cfg(target_os = "linux")]
    return systemd::uninstall();
    #[cfg(target_os = "macos")]
    return launchd::uninstall();
    #[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
    anyhow::bail!("no install support for this platform");
}

fn is_elevation_refusal(error: &anyhow::Error) -> bool {
    #[cfg(windows)]
    return error.is::<windows::ElevationRequired>();
    #[cfg(not(windows))]
    return {
        let _ = error;
        false
    };
}

fn finish_install(service: Result<()>, install_skills: impl FnOnce() -> Result<()>) -> Result<()> {
    service?;
    install_skills()
}

fn finish_uninstall(service: Result<()>, remove_skills: impl FnOnce() -> Result<()>) -> Result<()> {
    if let Err(error) = &service
        && is_elevation_refusal(error)
    {
        return service;
    }
    let skills = remove_skills();
    service.and(skills)
}

pub fn install() -> Result<()> {
    finish_install(install_service(), skills::install)
}

pub fn uninstall() -> Result<()> {
    finish_uninstall(uninstall_service(), skills::uninstall)
}

#[cfg(test)]
#[path = "../tests/install/order/mod.rs"]
mod tests;
