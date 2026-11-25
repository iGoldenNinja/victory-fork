//! Optional upstream compute captures. The stages share a tile list and an output
//! texture. Capture both inputs and the output around the actual dispatches.
use std::sync::OnceLock;
use windows::core::Interface;
use crate::{descriptor::{View, host::{WatchedSlot, buffer_copy}}, host::HostList};

pub(super) struct Config {
    pub table: u8,
    pub list: u32,
    pub output: u32,
    pub constant: u32,
    pub downstream: u32,
    pub downstream_constant: Option<u32>,
    pub rect: [u32; 4],
    pub shaders: Vec<String>,
}

fn parse(text: &str) -> Option<Config> {
    let mut words = text.split(',').map(str::trim);
    let table = words.next()?.parse().ok()?;
    let [list, output, constant] =
        std::array::from_fn(|_| words.next()?.parse::<u32>().ok());
    let (list, output, constant) = (list?, output?, constant?);
    let mut feed = words.next()?.split(':');
    let downstream = feed.next()?.parse().ok()?;
    let downstream_constant = feed.next().map(str::parse::<u32>).transpose().ok()?;
    if feed.next().is_some() || downstream_constant.is_some_and(|d| !d.is_multiple_of(8) || d == downstream) {
        return None;
    }
    let [x, y, w, h] = std::array::from_fn(|_| words.next()?.parse::<u32>().ok());
    let rect = [x?, y?, w?, h?];
    if [list, output, constant, downstream].iter().any(|d| !d.is_multiple_of(8))
        || list == output || list == constant || output == constant || rect[2] == 0 || rect[3] == 0
        || u64::from(rect[2]) * u64::from(rect[3]) > 1_048_576
        || rect[0].checked_add(rect[2]).is_none() || rect[1].checked_add(rect[3]).is_none() { return None; }
    let mut shaders = Vec::new();
    for word in words {
        let hash = word.to_ascii_lowercase();
        if hash.len() != 16 || !hash.bytes().all(|b| b.is_ascii_hexdigit())
            || shaders.len() == 8 || shaders.contains(&hash) { return None; }
        shaders.push(hash);
    }
    (!shaders.is_empty()).then_some(Config { table, list, output, constant, downstream, downstream_constant, rect, shaders })
}

pub(super) fn config() -> Option<&'static Config> {
    static SETTING: OnceLock<Option<Config>> = OnceLock::new();
    SETTING.get_or_init(|| {
        let text = std::env::var("VICTORY_CAPTURE_PRODUCERS").ok()?;
        let selection = parse(&text);
        if selection.is_none() { tracing::warn!("invalid producer capture settings"); }
        selection
    }).as_ref()
}

/// Accept only a shader-visible slot that still retains the exact buffer whose
/// GPU address and range the descriptor uses, including placed-buffer offsets.
pub(super) fn sample_slot(list: &HostList, slot: &WatchedSlot, view: Option<&View>,
                          frame: u64, name: String) -> bool {
    let Some(view) = view else { return false };
    if let View::Constant(_, bytes) = view
        && let Some((current, at)) = slot.scratch_range() {
        return super::pass::sample_buffer(list, &current, at, u64::from(*bytes), None, frame, name);
    }
    let binding = match view {
        View::Constant(address, size) => crate::shadow::find(*address, u64::from(*size))
            .map(|shadow| (shadow, *address, u64::from(*size))),
        _ => buffer_copy(view),
    };
    let Some((shadow, address, bytes)) = binding else { return false };
    let Some(current) = slot.resource() else { return false };
    if current.as_raw() != shadow.buffer.as_raw() || shadow.offset_in(address).is_none()
        || bytes > shadow.left(address) { return false; }
    super::pass::sample_buffer(list, &current, shadow.offset(address), bytes, None, frame, name)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn validates_upstream_capture_offsets_and_area() {
        let valid = "1,16,152,184,24,0,320,600,260,1234567890abcdef";
        assert_eq!(parse(valid).unwrap().rect, [0,320,600,260]);
        assert!(parse(&valid.replace(",16,", ",15,")).is_none());
        assert!(parse(&valid.replace("600,260", "4294967295,260")).is_none());
        assert!(parse(&format!("{valid},1234567890ABCDEF")).is_none());
        assert_eq!(parse(&valid.replace(",24,", ",24:80,")).unwrap().downstream_constant, Some(80));
        assert!(parse(&valid.replace(",24,", ",24:24,")).is_none());
        assert!(parse(&valid.replace(",24,", ",24:81,")).is_none());
        assert!(parse(&valid.replace(",24,", ",24:80:88,")).is_none());
    }
}
