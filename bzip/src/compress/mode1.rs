#![allow(clippy::unusual_byte_groupings)]

// This compressor was reversed by hand. It's way simpler than the mode 2 one.

use std::collections::{HashMap, VecDeque};

use super::count_equal;

pub fn compress(input: &[u8], out: &mut Vec<u8>) {
	let mut input_pos = 0;
	let mut last = 0;
	let mut cache = HashMap::<[u8; 7], VecDeque<usize>>::new();
	let mut w = 0;
	while input_pos < input.len() {
		let mut run_len = count_equal(&input[input_pos..], &input[input_pos+1..], (1<<12)+3) + 1;
		let mut run_pos = input_pos;
		if let Some(input_slice) = input.get(input_pos..input_pos+7) {
			let input_slice = <[u8; 7]>::try_from(input_slice).unwrap();
			for rep_pos in cache.entry(input_slice).or_default() {
				let rep_len = count_equal(&input[input_pos+7..], &input[*rep_pos+7..], usize::MAX) + 7;
				if rep_len > run_len {
					(run_len, run_pos) = (rep_len, *rep_pos);
				}
			}
		}

		if run_len >= 7 {
			write_verb(out, &input[last..input_pos]);
			if run_pos == input_pos {
				write_const(out, input[input_pos], run_len);
			} else {
				write_repeat(out, input_pos - run_pos, run_len);
			}
			input_pos += run_len;
			last = input_pos;
		} else {
			run_len = 1;
			input_pos += run_len;
		}

		while w < input_pos {
			if let Some(input_slice) = input.get(w..w+7) {
				let input_slice = input_slice.try_into().unwrap();
				cache.entry(input_slice).or_default().push_back(w);
			}
			if let Some(prev_pos) = w.checked_sub(0x1FFF) {
				let prev_slice = &input[prev_pos..prev_pos+7];
				let prev_slice = prev_slice.try_into().unwrap();
				cache.entry(prev_slice).or_default().pop_front();
			}
			w += 1;
		}
	}
	write_verb(out, &input[last..input_pos]);
}

fn write_verb(out: &mut Vec<u8>, input: &[u8]) {
	for w in input.chunks(0x1FFF) {
		write_head(out, 0b00_000000, 5, w.len());
		out.extend_from_slice(w);
	}
}

fn write_const(out: &mut Vec<u8>, b: u8, len: usize) {
	write_head(out, 0b010_00000, 4, len - 4);
	out.push(b);
}

fn write_repeat(out: &mut Vec<u8>, off: usize, mut len: usize) {
	assert!(len >= 7); // technically supports 4, but not used
	assert!(off < (1<<13));
	out.push(0b1_11_00000 | (off >> 8) as u8);
	out.push(off as u8);
	len -= 7;
	while len > 0 {
		out.push(0b011_00000 | len.min(0x1F) as u8);
		len = len.saturating_sub(0x1F);
	}
}

fn write_head(out: &mut Vec<u8>, mask: u8, bits: usize, len: usize) {
	assert!(mask & ((1<<(bits+1))-1) == 0);
	assert!(len < (1<<(8+bits)), "{len} < (1<<{})", 8+bits);
	if len >= (1<<bits) {
		out.push(mask | (1<<bits) | (len >> 8) as u8);
		out.push(len as u8);
	} else {
		out.push(mask | len as u8);
	}
}
