use super::*;

fn write_line(writer: &RotatingWriter, line: &str) {
    let mut guard = writer.make_writer();
    guard.write_all(line.as_bytes()).unwrap();
}

fn names(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().into_owned()).collect();
    names.sort();
    names
}

#[test]
fn a_write_that_would_pass_the_limit_starts_a_fresh_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("relay.log");
    let writer = RotatingWriter(Arc::new(Mutex::new(RotatorState::open(path.clone(), 10).unwrap())));

    write_line(&writer, "123456");
    write_line(&writer, "abcdef");

    assert_eq!(std::fs::read_to_string(&path).unwrap(), "abcdef");
    assert_eq!(std::fs::read_to_string(rotated_path(&path, 1)).unwrap(), "123456");
}

#[test]
fn only_three_rotated_files_are_kept_and_the_oldest_is_dropped() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("relay.log");
    let writer = RotatingWriter(Arc::new(Mutex::new(RotatorState::open(path.clone(), 4).unwrap())));

    for line in ["aaaa", "bbbb", "cccc", "dddd", "eeee"] {
        write_line(&writer, line);
    }

    assert_eq!(names(dir.path()), ["relay.log", "relay.log.1", "relay.log.2", "relay.log.3"]);
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "eeee");
    assert_eq!(std::fs::read_to_string(rotated_path(&path, 1)).unwrap(), "dddd");
    assert_eq!(std::fs::read_to_string(rotated_path(&path, 2)).unwrap(), "cccc");
    assert_eq!(std::fs::read_to_string(rotated_path(&path, 3)).unwrap(), "bbbb");
}

#[test]
fn an_existing_file_counts_toward_the_limit_after_a_restart() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("relay.log");
    std::fs::write(&path, "123456").unwrap();
    let writer = RotatingWriter(Arc::new(Mutex::new(RotatorState::open(path.clone(), 10).unwrap())));

    write_line(&writer, "abcdef");

    assert_eq!(std::fs::read_to_string(rotated_path(&path, 1)).unwrap(), "123456");
}
