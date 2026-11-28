use super::line::Line;

pub(super) fn least_recently_used(lines: &[Line]) -> usize {
    if let Some(index) = lines.iter().position(|line| !line.state.valid) {
        return index;
    }
    lines
        .iter()
        .enumerate()
        .min_by_key(|(_, line)| line.last_used)
        .map(|(index, _)| index)
        .expect("validated associativity")
}
