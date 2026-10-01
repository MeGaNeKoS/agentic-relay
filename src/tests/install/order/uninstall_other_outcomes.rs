use super::*;

#[test]
fn any_other_service_failure_still_removes_the_skills_and_is_reported() {
    let skills_touched = Cell::new(false);

    let result = finish_uninstall(Err(anyhow!("no such task")), || {
        skills_touched.set(true);
        Ok(())
    });

    assert_eq!(result.unwrap_err().to_string(), "no such task");
    assert!(skills_touched.get());
}

#[test]
fn a_removed_service_is_followed_by_removing_the_skills() {
    let skills_touched = Cell::new(false);

    let result = finish_uninstall(Ok(()), || {
        skills_touched.set(true);
        Ok(())
    });

    assert!(result.is_ok());
    assert!(skills_touched.get());
}
