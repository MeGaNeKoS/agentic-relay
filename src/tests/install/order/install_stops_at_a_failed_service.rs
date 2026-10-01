use super::*;

#[test]
fn a_failed_service_registration_writes_no_skills() {
    let skills_touched = Cell::new(false);

    let result = finish_install(Err(anyhow!("registering failed")), || {
        skills_touched.set(true);
        Ok(())
    });

    assert!(result.is_err());
    assert!(!skills_touched.get());
}
