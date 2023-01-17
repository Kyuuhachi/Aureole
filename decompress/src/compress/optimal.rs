use std::ops::Range;

use super::common::*;
use super::suffix;

#[derive(Debug)]
struct SplitSuffixArray<'a> {
	chunk_len: usize,
	chunks: Vec<suffix::ByteSuffixArray<'a>>,
}

impl<'a> SplitSuffixArray<'a> {
	fn new(text: &'a [u8], chunk_len: usize, overlap: usize) -> Self {
		let mut chunks = Vec::with_capacity((text.len() + chunk_len - 1) / chunk_len);
		let mut i = 0;
		while i < text.len() {
			let chunk = &text[i..(i+chunk_len+overlap).min(text.len())];
			chunks.push(suffix::SuffixArray::new(chunk));
			i += chunk_len;
		}
		SplitSuffixArray {
			chunk_len,
			chunks,
		}
	}

	fn find(&self, window: Range<usize>, needle: &[u8]) -> Vec<(usize, usize)> {
		if window.is_empty() {
			return Vec::new()
		}
		let mut out = Vec::new();
		let mut len = 0;
		let first_chunk = window.start / self.chunk_len;
		let last_chunk = (window.end-1) / self.chunk_len;
		for chunk_idx in (first_chunk..=last_chunk).rev() {
			let chunk_off = chunk_idx * self.chunk_len;
			let mut chunk = self.chunks[chunk_idx].as_ref();
			chunk = chunk.find(&needle[chunk.offset()..len]);

			while chunk.offset() < needle.len() {
				chunk = chunk.find(&needle[chunk.offset()..chunk.offset()+1]);
				// chunk = chunk.advance_on(&needle[chunk.offset()..]);
				let max = chunk.full().indices()
					.filter(|&a| window.contains(&(chunk_off+a)))
					.max();
				match max {
					Some(max) => {
						out.push((chunk_off+max, chunk.offset()));
						len = chunk.offset();
					},
					None => break,
				}
			}
		}
		out
	}
}

pub fn compress_chunk(data: &[u8]) -> Vec<u8> {
	const LOOKAHEAD: usize = 269;
	let ssa = SplitSuffixArray::new(data, 1<<13, LOOKAHEAD);
	let mut dp = vec![(usize::MAX, CompressCommand::NO_COMMAND); data.len()+1];
	dp[0] = (0, CompressCommand::NO_COMMAND);
	for i in 0..data.len() {
		let mut cmds = Vec::new();
		cmds.push(CompressCommand::Byte { val: data[i] });

		for (o, n) in ssa.find(i.saturating_sub((1<<13)-1)..i, &data[i..(i+LOOKAHEAD).min(data.len())]) {
			if n >= 2 {
				cmds.push(CompressCommand::Match { offset: i - o, count: n });
			}
		}

		let fill = data[i..].iter().take_while(|a| **a == data[i]).count();
		for n in 14..=fill { // I think we can safely exclude a couple of these
			cmds.push(CompressCommand::Fill { val: data[i], count: n });
		}

		for cmd in cmds {
			let (bytes, bits) = cmd.size();
			if dp[i].0 + bits < dp[i+bytes].0 {
				dp[i+bytes] = (dp[i].0 + bits, cmd)
			}
		}
	}

	let mut commands = Vec::new();
	let mut i = data.len();
	while i > 0 {
		let cmd = dp[i].1;
		let (bytes, _) = cmd.size();
		commands.push(cmd);
		i -= bytes;
	}

	let mut b = BitW::new();
	for cmd in commands.into_iter().rev() {
		cmd.write_to(&mut b);
	}
	b.bit(true);
	b.bit(true);
	b.bits(13, 0);
	b.finish()
}
