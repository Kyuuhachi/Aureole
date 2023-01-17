// Adapted from uyjulian's https://gist.github.com/uyjulian/ba631cbba7025806c5e356daeb3c9507
// May not give exactly the same byte sequence.

use super::common::*;

const WINDOW_SIZE: usize = (1<<13)-1;
const MIN_MATCH: usize = 2;
const MAX_MATCH: usize = 0xFF + 0xE;

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
	memchr::memmem::rfind(haystack, needle)
}

fn find_match(data: &[u8], pos: usize) -> Option<(usize, usize)> {
	let window = &data[pos.saturating_sub(WINDOW_SIZE)..pos];
	let max_match = MAX_MATCH.min(data.len() - pos);

	let Some((size, match_pos)) = (MIN_MATCH..=max_match)
		.map_while(|size| find(window, &data[pos..pos+size]).map(|pos| (size, pos)))
		.last()
	else { return None };

	let extra = if window.len() - match_pos == size {
		data[pos..].iter()
			.zip(window[match_pos+size..].iter().cycle())
			.take(max_match - size)
			.take_while(|a| a.0 == a.1)
			.count()
	} else {
		0
	};
	let size = size + extra;
	let match_pos = window.len() - match_pos;
	Some((size, match_pos))
}

fn find_repeat(data: &[u8], pos: usize) -> usize {
	data[pos..].iter()
		.take(MAX_MATCH)
		.take_while(|a| **a == data[pos])
		.count()
}

pub fn compress_chunk(data: &[u8]) -> Vec<u8> {
	let mut pos = 0;
	let mut b = BitW::new();
	while pos < data.len() {
		let cmd = match (find_match(data, pos), find_repeat(data, pos)) {
			(Some((match_size, match_pos)), repeat_size) if match_size >= repeat_size
				=> CompressCommand::Match { offset: match_pos, count: match_size },
			(_, repeat_size) if repeat_size >= 14
				=> CompressCommand::Fill { val: data[pos], count: repeat_size },
			(_, _) => CompressCommand::Byte { val: data[pos] },
		};
		pos += cmd.write_to(&mut b);
	}
	b.bit(true);
	b.bit(true);
	b.bits(13, 0);
	b.finish()
}
