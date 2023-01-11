// Made according to this guide https://zork.net/~st/jottings/sais.html
use std::cmp::Ordering;

trait Value: Ord + Clone + Into<usize> {
	fn i(&self) -> usize {
		self.clone().into()
	}
}
impl<T: Ord + Clone + Into<usize>> Value for T {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Type {
	S, // smaller
	L, // larger
}

fn types(t: &[impl Ord]) -> Vec<Type> { // TODO use bitvec?
	let mut res = vec![Type::S; t.len() + 1];
	if res.len() > 1 {
		res[t.len()-1] = Type::L;
		for i in (0..res.len()-2).rev() {
			res[i] = match t[i].cmp(&t[i+1]) {
				Ordering::Less => Type::S,
				Ordering::Equal => res[i+1],
				Ordering::Greater => Type::L,
			}
		}
	}
	res
}

#[inline]
fn is_lms(types: &[Type], pos: usize) -> bool {
	pos > 0 && types[pos] == Type::S && types[pos-1] == Type::L
}

#[inline]
fn buckets<'a, const TAIL: bool>(t: &[impl Value], buckets: &'a mut [usize]) -> &'a mut [usize] {
	buckets.fill(0);
	for b in t {
		buckets[b.i()] += 1;
	}
	let mut o = 1;
	for b in buckets.iter_mut() {
		o += *b;
		*b = o - if TAIL { 0 } else { *b };
	}
	buckets
}

pub fn make_suffix_array(t: &[u8]) -> Vec<usize> {
	let mut sa = vec![0; t.len()+1];
	make_array(t, &mut [0; 256], &mut sa);
	sa
}

fn make_array(t: &[impl Value], scratch: &mut [usize], sa: &mut [usize]) {

	let types = &types(t);

	lms_sort(t, types, scratch, sa,
		(0..t.len()).filter(|&i| is_lms(types, i))
	);

	let mut names = vec![usize::MAX; t.len()+1];
	let mut cur_name = 0;
	names[sa[0]] = cur_name;
	let mut last_offset = sa[0];
	for &offset in &sa[1..] {
		if is_lms(types, offset) {
			if !lms_equal(t, types, last_offset, offset) {
				cur_name += 1
			}
			last_offset = offset;
			names[offset] = cur_name;
		}
	}

	let (summary_offsets, t1): (Vec<_>, Vec<_>) = names.into_iter()
		.enumerate()
		.filter(|a| a.1 != usize::MAX)
		.unzip();
	let (t1, k1, summary_offsets) = (t1, cur_name + 1, summary_offsets);

	let mut sa1 = vec![usize::MAX; t1.len()+1];
	if t1.len() == k1 {
		sa1[0] = t1.len();
		for (i, &c) in t1.iter().enumerate() {
			sa1[c+1] = i
		}
	} else {
		make_array(&t1, &mut vec![0; k1], &mut sa1);
	};

	lms_sort(t, types, scratch, sa,
		sa1[2..].iter().map(|&i| summary_offsets[i])
	);
}

fn lms_sort(
	t: &[impl Value],
	types: &[Type],
	scratch: &mut [usize],
	sa: &mut [usize],
	ix: impl DoubleEndedIterator<Item=usize>,
) {
	sa.fill(usize::MAX);
	sa[0] = t.len();

	let tails = buckets::<true>(t, scratch);
	for i in ix.rev() {
		debug_assert!(i > 0);
		let v = &mut tails[t[i].i()];
		*v -= 1;
		sa[*v] = i;
	}

	let heads = buckets::<false>(t, scratch);
	for i in 0..sa.len() {
		if sa[i] != usize::MAX && sa[i] != 0 && types[sa[i]-1] == Type::L {
			let v = &mut heads[t[sa[i]-1].i()];
			debug_assert!(*v > i);
			sa[*v] = sa[i] - 1;
			*v += 1;
		}
	}

	let tails = buckets::<true>(t, scratch);
	for i in (0..sa.len()).rev() {
		if sa[i] != usize::MAX && sa[i] != 0 && types[sa[i]-1] == Type::S {
			let v = &mut tails[t[sa[i]-1].i()];
			debug_assert!(*v <= i);
			*v -= 1;
			sa[*v] = sa[i] - 1;
		}
	}
}

fn lms_equal(t: &[impl Value], types: &[Type], a: usize, b: usize) -> bool {
	if a == t.len() || b == t.len() {
		return a == b
	}
	let mut i = 0;
	loop {
		let al = is_lms(types, a+i);
		let bl = is_lms(types, b+i);
		if i > 0 && al && bl {
			return true
		}
		if al != bl {
			return false
		}
		if t[a+i] != t[b+i] {
			return false
		}
		i += 1
	}
}

#[test]
fn a() {
	assert_eq!(make_suffix_array("cabbage".as_bytes()), [7, 1, 4, 3, 2, 0, 6, 5]);
	assert_eq!(make_suffix_array("baabaabac".as_bytes()), [9, 1, 4, 2, 5, 7, 0, 3, 6, 8]);
}
