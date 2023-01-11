// Ported from uyjulian's https://gist.github.com/uyjulian/ba631cbba7025806c5e356daeb3c9507

const WINDOW_SIZE: usize = 0x1FFF;
const MIN_MATCH: usize = 2;
const MAX_MATCH: usize = 0xFF + 0xE;

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
	memchr::memmem::rfind(haystack, needle)
}

fn find_match(data: &[u8], pos: usize) -> Option<(usize, usize)> {
	let window = &data[pos.saturating_sub(WINDOW_SIZE)..pos];
	let max_match = MAX_MATCH.min(data.len() - pos);
	if max_match < MIN_MATCH {
		return None;
	}
	if find(window, &data[pos..pos+MIN_MATCH]).is_none() {
		return None;
	}

	let size = 'a: {
		for size in MIN_MATCH..=max_match {
			if find(window, &data[pos..pos+size]).is_none() {
				break 'a size - 1
			}
		}
		max_match
	};

	let match_pos = find(window, &data[pos..pos+size]).unwrap();
	if size == match_pos {
		let match_pos = window.len() - match_pos;
		return Some((size, match_pos))
	}

	if window.len() - match_pos == size {
		let mut extra_match = 0;
		let mut extra_match1 = 0;
		while data[pos + size + extra_match] == window[match_pos + extra_match1] {
			extra_match += 1;
			extra_match1 += 1;
			if match_pos + extra_match1 == window.len() {
				extra_match1 = 0;
			}
			if size + extra_match == max_match {
				break
			}
		}
		let size = size + extra_match;
		let match_pos = window.len() - match_pos;
		return Some((size, match_pos));
	} else {
		let match_pos = window.len() - match_pos;
		return Some((size, match_pos));
	}
}

fn find_repeat(data: &[u8], pos: usize) -> Option<usize> {
	let max_match = MAX_MATCH.min(data.len() - pos);
	let mut i = 0;
	while i < max_match && data[pos+i] == data[i] {
		i += 1
	}
	if i < 3 {
		None
	} else {
		Some(i)
	}
}

struct Ctx {
	flags: u16,
	flag_write: u16,
	flag_pos: usize,
	out: Vec<u8>,
	buf: Vec<u8>,
}

impl Ctx {
	fn new() -> Self {
		Self {
			flags: 0,
			flag_write: 0x8000,
			flag_pos: 8,
			out: Vec::new(),
			buf: Vec::new(),
		}
	}

	fn bit(&mut self, b: bool) {
		if b {
			self.flags |= self.flag_write
		}
		self.flag_pos -= 1;
		if self.flag_pos == 0 {
			self.out.extend(u16::to_le_bytes(self.flags));
			self.out.extend(self.buf.iter());
			self.buf.clear();
			self.flag_pos = 16;
			self.flags = 0;
		} else {
			self.flags >>= 1;
		}
	}

	fn byte(&mut self, b: u8) {
		self.buf.push(b)
	}
}

fn encode_repeat(repeat_byte: u8, repeat_size: usize, ctx: &mut Ctx) {
	if repeat_size < 14 {
		ctx.byte(repeat_byte);
		ctx.bit(false);
		encode_match(repeat_size - 1, 1, ctx);
	} else {
		let repeat_size = repeat_size - 14;
		for _ in 0..2 {
			ctx.bit(false);
		}
		for _ in 0..4 {
			ctx.bit(true);
		}
		ctx.byte(1);
		ctx.bit(false);
		if repeat_size < 16 {
			ctx.bit(false);
			for i in (0..4).rev() {
				ctx.bit((repeat_size >> i) & 1 != 0);
				if i == 1 {
					ctx.byte(repeat_byte);
				}
			}
		} else {
			let high_order = (repeat_size >> 8) as u8;
			let low_order = (repeat_size & 0xFF) as u8;
			ctx.bit(true);
			for i in (0..4).rev() {
				ctx.bit((high_order >> i) & 1 != 0);
				if i == 1 {
					ctx.byte(low_order);
					ctx.byte(repeat_byte);
				}
			}
		}
	}
}

fn encode_match(match_size: usize, match_pos: usize, ctx: &mut Ctx) {
	if match_pos < 0x100 {
		ctx.bit(true);
		ctx.byte(match_pos as u8);
		ctx.bit(false);
	} else {
		let high_order = (match_pos >> 8) as u8;
		let low_order = (match_pos & 0xFF) as u8;
		for _ in 0..2 {
			ctx.bit(true);
		}
		for i in (0..5).rev() {
			ctx.bit((high_order >> i) & 1 != 0);
			if i == 1 {
				ctx.byte(low_order);
			}
		}
	}

	for i in 2..5 {
		if i >= match_size {
			break
		}
		ctx.bit(false);
	}
	if match_size >= 6 {
		ctx.bit(false);
		if match_size >= 14 {
			let match_size = match_size - 14;
			ctx.byte(match_size as u8);
			ctx.bit(false);
		} else {
			ctx.bit(true);
			let match_size = match_size - 6;
			for i in (0..3).rev() {
				ctx.bit((match_size>>i) & 1 != 0);
			}
		}
	} else {
		ctx.bit(true);
	}
}

pub fn compress_chunk(data: &[u8]) -> Vec<u8> {
	let mut pos = 0;
	let mut ctx = Ctx::new();
	while pos < data.len() {
		match (find_match(data, pos), find_repeat(data, pos)) {
			(Some((match_size, _)), Some(repeat_size)) if repeat_size > match_size => {
				encode_repeat(data[pos], repeat_size, &mut ctx);
				pos += repeat_size;
			}
			(None, Some(repeat_size)) => {
				encode_repeat(data[pos], repeat_size, &mut ctx);
				pos += repeat_size;
			}
			(Some((match_size, match_pos)), _) => {
				encode_match(match_size, match_pos, &mut ctx);
				pos += match_size;
			}
			(_, _) => {
				ctx.byte(data[pos]);
				pos += 1;
				ctx.bit(false);
			}
		}
	}
	for _ in 0..2 {
		ctx.bit(true);
	}
	for i in 0..5 {
		if i == 4 {
			ctx.byte(0);
		}
		ctx.bit(false);
	}
	if ctx.flag_pos != 16 {
		for _ in 0..ctx.flag_pos {
			ctx.bit(false);
		}
	}
	ctx.out
}
