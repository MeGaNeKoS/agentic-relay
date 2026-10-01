use super::*;
use crate::install::windows::ElevationRequired;

#[test]
fn a_refusal_for_elevation_leaves_the_skills_in_place() {
    let skills_touched = Cell::new(false);

    let result = finish_uninstall(Err(ElevationRequired.into()), || {
        skills_touched.set(true);
        Ok(())
    });

    assert!(result.unwrap_err().is::<ElevationRequired>());
    assert!(!skills_touched.get());
}
