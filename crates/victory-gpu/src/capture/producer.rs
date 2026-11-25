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

// A producer can classify tiles in a small mask before emitting an indirect list.
// Use the same crop parser as other dispatch captures, followed by list and CBV offsets.
pub(super) struct Mask {
    pub pass: super::pass::Config,
    pub list: u32,
    pub constant: u32,
    pub counters: Option<u32>,
}

fn parse_mask(text: &str) -> Option<Mask> {
    let (fields, constant) = text.rsplit_once(',')?;
    let (fields, list) = fields.rsplit_once(',')?;
    let (list, counters) = match list.split_once(':') {
        Some((a,b)) => (a, Some(b.trim().parse::<u32>().ok()?)),
        None => (list, None),
    };
    let (list, constant) = (list.trim().parse::<u32>().ok()?, constant.trim().parse::<u32>().ok()?);
    let pass = super::pass::parse(fields)?;
    let slots = [pass.srv, pass.uav, list, constant];
    if slots.iter().enumerate().any(|(i, d)| !d.is_multiple_of(8) || slots[..i].contains(d)) {
        return None;
    }
    if counters.is_some_and(|d| !d.is_multiple_of(8) || slots.contains(&d)) { return None; }
    Some(Mask { pass, list, constant, counters })
}

pub(super) fn mask() -> Option<&'static Mask> {
    static MASK: OnceLock<Option<Mask>> = OnceLock::new();
    MASK.get_or_init(|| {
        let text = std::env::var("VICTORY_CAPTURE_MASK_PASS").ok()?;
        let selection = parse_mask(&text);
        if selection.is_none() { tracing::warn!("invalid tile-mask capture settings"); }
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

/// Retain the exact resource and byte range behind a writable shader descriptor.
pub(super) fn output_range(slot: &WatchedSlot, view: Option<&View>)
    -> Option<(windows::Win32::Graphics::Direct3D12::ID3D12Resource, u64, u64)> {
    let view = view?;
    if !matches!(view, View::Unordered(..) | View::Placed(true, ..)) { return None; }
    let (shadow, address, bytes) = buffer_copy(view)?;
    let actual = slot.resource()?;
    if actual.as_raw() != shadow.buffer.as_raw() || bytes > shadow.left(address) { return None; }
    let at = shadow.offset_in(address)?;
    Some((actual, at, bytes))
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

    #[test]
    fn parses_mask_textures_and_distinct_list_and_constant_offsets() {
        let fields = "1234567890abcdef,1,8,40,0,0,20,23";
        let mask = parse_mask(&format!("{fields},32,48")).unwrap();
        assert_eq!((mask.pass.rect, mask.list, mask.constant, mask.counters), ([0,0,20,23], 32, 48, None));
        assert_eq!(parse_mask(&format!("{fields},32:24,48")).unwrap().counters, Some(24));
        for invalid in ["32:8,48", "32:32,48", "32:48,48", "32:21,48", "32:24:16,48"] {
            assert!(parse_mask(&format!("{fields},{invalid}")).is_none());
        }
        assert!(parse_mask(&format!("{fields},32,32")).is_none());
        assert!(parse_mask(&format!("{fields},8,48")).is_none());
        assert!(parse_mask(&format!("{fields},33,48")).is_none());
    }
}
