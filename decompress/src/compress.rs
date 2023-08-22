pub(crate) mod suffix;
pub(crate) mod common;

mod py;
mod naive;
mod optimal;
mod canon;

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

pub use py::compress_chunk as compress;

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
		let mut d2 = Vec::with_capacity(d.len());
		crate::decompress::decompress(c, &mut d2).unwrap();
		assert!(d2 == d, "\n[[[\n\n{}\n\n===\n\n{}\n\n]]]", String::from_utf8_lossy(d), String::from_utf8_lossy(&d2));
		println!("{name}: {}", c.len() as f32 / d.len() as f32);
	}

	#[test]
	fn should_roundtrip_font64() {
		use gospel::read::{Reader, Le as _};

		let data = std::fs::read("../data/fc.extract2/00/font64._da").unwrap();
		let mut f = Reader::new(&data);
		let start = std::time::Instant::now();
		let mut d1 = std::time::Duration::ZERO;
		let mut d2 = std::time::Duration::ZERO;

		loop {
			let chunklen = f.u16().unwrap() as usize - 2;
			let inchunk = f.slice(chunklen).unwrap();
			if inchunk[0] != 0 {
				println!("skip");
				continue
			}
			println!("{} / {}", f.pos(), f.len());

			let mut chunk = Vec::new();
			let start = std::time::Instant::now();
			crate::decompress::decompress(inchunk, &mut chunk).unwrap();
			let end = std::time::Instant::now();
			d1 += end - start;

			let mut outchunk = Vec::new();
			let start = std::time::Instant::now();
			super::canon::compress(&chunk, &mut outchunk);
			let end = std::time::Instant::now();
			d2 += end - start;

			assert!(inchunk == outchunk);

			if f.u8().unwrap() == 0 {
				break
			}
		}
		let end = std::time::Instant::now();

		println!("Decompress {}, compress {}, total {}", d1.as_secs_f64(), d2.as_secs_f64(), (end-start).as_secs_f64());
	}
}
