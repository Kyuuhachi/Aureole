// Based on a disassembly of Zwei II's Packs.dll.

#[allow(unused)]
pub fn compress(input: &[u8], out: &mut Vec<u8>) {
	let mut input_pos = 0;
	let mut b = Bits::new(out);
	let mut dig = Digraphs::new(input);
	while input_pos < input.len() {
		let mut run_len = count_equal(&input[input_pos..], &input[input_pos+1..], 0xFFE) + 1;
		if run_len < 14 { run_len = 1; }
		let mut run_pos = input_pos;

		if run_len < 64 && input_pos + 3 < input.len() {
			if let Some((rep_len, rep_pos)) = dig.get() {
				if rep_len >= run_len {
					(run_len, run_pos) = (rep_len, rep_pos);
				}
			}
		}

		assert!(run_len > 0);
		if b.bit(run_len > 1) {
			if run_pos == input_pos {
				b.bit(true);
				b.bits(13, 1);
				let n = run_len - 14;
				if b.bit(n >= 16) {
					b.bits(12, n);
				} else {
					b.bits(4, n);
				}
				b.byte(input[input_pos]);
			} else {
				let n = input_pos - run_pos;
				if b.bit(n >= 256) {
					b.bits(13, n);
				} else {
					b.bits(8, n);
				}

				let m = run_len;
				if m >= 3 { b.bit(false); }
				if m >= 4 { b.bit(false); }
				if m >= 5 { b.bit(false); }
				if m >= 6 { b.bit(false); }
				if b.bit(m < 14) {
					if m >= 6 {
						b.bits(3, m-6);
					}
				} else {
					b.bits(8, m-14);
				}
			}
		} else {
			b.byte(input[input_pos]);
		}

		for _ in 0..run_len {
			input_pos += 1;
			dig.advance();
		}
	}
	b.bit(true);
	b.bit(true);
	b.bits(13, 0);
}

fn count_equal(a: &[u8], b: &[u8], limit: usize) -> usize {
	std::iter::zip(a, b)
		.take_while(|(a, b)| a == b)
		.take(limit)
		.count()
}

struct Digraphs<'a> {
	input: &'a [u8],
	pos: usize,
	head: [u16; 0x10000],
	next: [u16; 0x2000], // Falcom's is 0x8000, but that doesn't bring any benefits
	tail: [u16; 0x10000],
}

impl Digraphs<'_> {
	fn new(input: &[u8]) -> Digraphs {
		Digraphs {
			input,
			pos: 0,
			head: [0xFFFF; 0x10000],
			next: [0xFFFF; 0x2000],
			tail: [0xFFFF; 0x10000],
		}
	}

	#[inline(always)]
	fn digraph(&self, pos: usize) -> usize {
		let b1 = self.input[pos];
		let b2 = *self.input.get(pos+1).unwrap_or(&0);
		u16::from_le_bytes([b1, b2]) as usize
	}

	fn advance(&mut self) {
		if self.pos >= 0x1FFF {
			let prev_pos = self.pos - 0x1FFF;
			let dig = self.digraph(prev_pos);
			self.head[dig] = self.next[prev_pos % self.next.len()];
		}

		let dig = self.digraph(self.pos);

		if self.head[dig] == 0xFFFF {
			self.head[dig] = self.pos as u16;
		} else {
			self.next[self.tail[dig] as usize] = self.pos as u16;
		}
		self.tail[dig] = (self.pos % self.next.len()) as u16;
		self.next[self.pos % self.next.len()] = 0xFFFF;

		self.pos += 1;
	}

	fn get(&self) -> Option<(usize, usize)> {
		fn slot_to_pos(a: u16) -> Option<usize> {
			(a != 0xFFFF).then_some(a as usize)
		}
		std::iter::successors(
			slot_to_pos(self.head[self.digraph(self.pos)]),
			|a| slot_to_pos(self.next[*a % self.next.len()]),
		)
			.map(|pos| {
				let len = count_equal(&self.input[self.pos..], &self.input[pos..], 269);
				(len, pos)
			})
			.max_by_key(|a| a.0)
	}
}

struct Bits<'a> {
	out: &'a mut Vec<u8>,
	bit_mask: u16,
	bitpos: usize,
}

impl<'a> Bits<'a> {
	fn new(out: &'a mut Vec<u8>) -> Self {
		let bitpos = out.len();
		out.extend([0, 0]);
		Self { out, bit_mask: 0x0080, bitpos }
	}

	fn bit(&mut self, v: bool) -> bool {
		self.bit_mask <<= 1;
		if self.bit_mask == 0 {
			self.bitpos = self.out.len();
			self.out.extend([0, 0]);
			self.bit_mask = 0x0001;
		}
		if v {
			if self.bit_mask < 256 {
				self.out[self.bitpos] |= self.bit_mask as u8;
			} else {
				self.out[self.bitpos+1] |= (self.bit_mask>>8) as u8;
			}
		}
		v
	}

	fn bits(&mut self, n: usize, v: usize) {
		assert!(v < (1<<n), "{v} < (1<<{n})");
		for k in (n/8*8..n).rev() {
			self.bit((v>>k) & 1 != 0);
		}
		for k in (0..n/8).rev() {
			self.out.push((v>>(k*8)) as u8);
		}
	}

	fn byte(&mut self, v: u8) {
		self.out.push(v);
	}
}
