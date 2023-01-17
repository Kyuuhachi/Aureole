pub(crate) mod suffix;
pub(crate) mod common;

mod py;
mod naive;
mod optimal;

#[allow(unused)]
pub fn compress_chunk_noop(data: &[u8]) -> Vec<u8> {
	let mut f = Vec::new();
	for c in data.chunks(1<<13) {
		f.push(0x20 | (c.len() >> 8) as u8);
		f.push(c.len() as u8);
		f.extend(c);
	}
	f
}

pub use py::compress_chunk;

#[allow(unused)]
pub(crate) use py::compress_chunk as compress_chunk_py;

#[allow(unused)]
pub(crate) use naive::compress_chunk as compress_chunk_naive;

#[allow(unused)]
pub(crate) use optimal::compress_chunk as compress_chunk_optimal;

#[cfg(test)]
mod test {
	extern crate test;
	const DATA: &[u8] = include_bytes!("compress.rs");

	#[bench]
	fn bench_py(b: &mut test::Bencher) {
		use super::compress_chunk_py as compress;
		b.iter(|| compress(DATA));
		check_compress("py", DATA, &compress(DATA));
	}

	#[bench]
	fn bench_naive(b: &mut test::Bencher) {
		use super::compress_chunk_naive as compress;
		b.iter(|| compress(DATA));
		check_compress("naive", DATA, &compress(DATA));
	}

	#[bench]
	fn bench_optimal(b: &mut test::Bencher) {
		use super::compress_chunk_optimal as compress;
		b.iter(|| compress(DATA));
		check_compress("optimal", DATA, &compress(DATA));
	}

	fn check_compress(name: &'static str, d: &[u8], c: &[u8]) {
		use super::super::decompress::decompress_chunk as decompress;
		let mut d2 = Vec::with_capacity(d.len());
		decompress(c, &mut d2).unwrap();
		assert!(d2 == d, "\n[[[\n\n{}\n\n===\n\n{}\n\n]]]", String::from_utf8_lossy(d), String::from_utf8_lossy(&d2));
		println!("{name}: {}", c.len() as f32 / d.len() as f32);
	}
}
