pub fn parse_header(content: &str) -> (Option<u32>, Option<(u32, u32)>) {
    let mut tempo = None;
    let mut time = None;

    for line in content.lines() {
        let line = line.trim();
        let lower = line.to_ascii_lowercase();

        if let Some(value) = lower.strip_prefix("tempo:") {
            tempo = value.split_whitespace().next().and_then(|s| s.parse().ok());
        } else if let Some(value) = lower.strip_prefix("time:") {
            time = value
                .split_once('/')
                .and_then(|(n, d)| Some((n.trim().parse().ok()?, d.trim().parse().ok()?)));
        }
    }

    (tempo, time)
}
