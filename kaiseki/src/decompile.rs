use std::{ops::Range, collections::{BTreeMap, BTreeSet}, fmt::Debug};
use crate::util;

#[derive(Debug)]
pub enum Trace {
	Goto {
		addr: usize,
		target: usize,
		brk: Option<usize>,
	},
	Statement {
		range: Range<usize>,
		brk: Option<usize>,
		next: Box<Trace>,
	},
}

impl std::fmt::Display for Trace {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		fn write_brk(f: &mut std::fmt::Formatter, brk: &Option<usize>) -> std::fmt::Result {
			if let Some(brk) = brk {
				write!(f, " [{}]", brk)?;
			}
			Ok(())
		}

		match self {
			Trace::Goto { addr, target, brk } => {
				write!(f, "({addr}) {target}")?;
				write_brk(f, brk)?;
			},
			Trace::Statement { range, brk, next } => {
				write!(f, "({range:?})")?;
				write_brk(f, brk)?;
				write!(f, " → ")?;
				std::fmt::Display::fmt(&next, f)?;
			},
		}
		Ok(())
	}
}

#[derive(Debug, thiserror::Error)]
pub struct Error {
	pub code: String,
	pub trace: Box<Trace>,
}

impl std::fmt::Display for Error {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(f, "Decompilation error: ")?;
		std::fmt::Display::fmt(&self.trace, f)?;
		if f.alternate() {
			write!(f, "\nCode:\n")?;
			for l in self.code.lines() {
				writeln!(f, "  {l}")?;
			}
		}
		Ok(())
	}
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FlowInsn<E, I> {
	Unless(E, usize),
	Goto(usize),
	Switch(E, Vec<(u16, usize)>, usize),
	Insn(I),
}

impl<E, I> FlowInsn<E, I> {
	pub fn labels(&self, mut f: impl FnMut(usize)) {
		match self {
			FlowInsn::Unless(_, target) => f(*target),
			FlowInsn::Goto(target) => f(*target),
			FlowInsn::Switch(_, branches, default) => {
				for (_, target) in branches {
					f(*target);
				}
				f(*default);
			}
			FlowInsn::Insn(_) => {}
		}
	}
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Stmt<E, I> {
	#[allow(clippy::type_complexity)]
	If(Vec<(Option<E>, Vec<Stmt<E, I>>)>),
	#[allow(clippy::type_complexity)]
	Switch(E, Vec<(Vec<Option<u16>>, Vec<Stmt<E, I>>)>),
	While(E, Vec<Stmt<E, I>>),
	Break,
	Insn(I),
}

#[tracing::instrument(skip(asm, end))]
pub fn decompile<E: Clone + Debug, I: Clone + Debug>(asm: &[(usize, FlowInsn<E, I>)], end: usize) -> Result<Vec<Stmt<E, I>>, Error> {
	let asm_map = BTreeMap::from_iter(asm.iter().map(|(k, v)| (*k, v)));
	let start = asm_map.keys().next().copied().unwrap_or(end);
	Decompiler { asm: asm_map }.block(start..end, None)
	.map_err(|e| {
		let mut labels = BTreeSet::<usize>::new();
		for (_, insn) in asm {
			insn.labels(|a| { labels.insert(a); });
		}
		let mut out = Vec::new();
		for (addr, insn) in asm {
			if labels.contains(addr) {
				out.push(format!("{addr}:"));
			}
			out.push(format!("  {insn:?}"));
		}
		Error { code: out.join("\n"), trace: Box::new(e) }
	})
}

#[derive(Clone)]
struct Decompiler<'a, E, I> {
	asm: BTreeMap<usize, &'a FlowInsn<E, I>>,
}

impl<E: Clone, I: Clone> Decompiler<'_, E, I> {
	fn block(&self, mut range: Range<usize>, brk: Option<usize>) -> Result<Vec<Stmt<E, I>>, Trace> {
		let mut out = Vec::new();
		while range.start < range.end {
			out.push(self.stmt(&mut range, brk)?);
		}
		Ok(out)
	}

	fn stmt(&self, range: &mut Range<usize>, brk: Option<usize>) -> Result<Stmt<E, I>, Trace> {
		let range_ = range.clone();
		self.stmt_inner(range, brk)
		.map_err(|e| Trace::Statement { range: range_, brk, next: Box::new(e) })
	}

	#[inline]
	fn stmt_inner(&self, range: &mut Range<usize>, brk: Option<usize>) -> Result<Stmt<E, I>, Trace> {
		let start = range.start;
		*range = self.advance(range.clone());
		Ok(match *self.asm[&start] {
			FlowInsn::Unless(ref expr, l1) => {
				match self.find_jump_before(l1, brk) {
					// While
					// =====
					// L0:
					//   UNLESS expr GOTO L1
					//   body (brk=L1)
					//   GOTO L0
					// L1:
					Some((inner, l0)) if l0 == start => {
						let body = self.block(range.start..inner, Some(l1))?;
						range.start = l1;
						Stmt::While(expr.clone(), body)
					}

					// If/else (flattened for convenience)
					// =======
					//   UNLESS expr GOTO L1
					//   body1 (brk=inherit)
					//   GOTO L2
					// L1:
					//   body2 (brk=inherit)
					// L2:
					Some((inner, l2)) if l2 >= l1 => {
						let body1 = self.block(range.start..inner, brk)?;
						let body2 = self.block(l1..l2, brk)?;
						range.start = l2;

						let mut cases = vec![(Some(expr.clone()), body1)];
						match &body2[..] {
							[Stmt::If(more_cases)] => cases.extend(more_cases.iter().cloned()),
							a => cases.push((None, a.to_owned())),
						}
						Stmt::If(cases)
					}

					// If
					// ==
					//   UNLESS expr GOTO L1
					//   then (brk=inherit)
					// L1:
					_ => {
						let body = self.block(range.start..l1, brk)?;
						range.start = l1;

						let cases = vec![(Some(expr.clone()), body)];
						Stmt::If(cases)
					}
				}
			}

			FlowInsn::Goto(l1) => {
				if Some(l1) == brk {
					Stmt::Break
				} else {
					return Err(Trace::Goto { addr: start, target: l1, brk });
				}
			}

			FlowInsn::Switch(ref expr, ref clauses, default) => {
				let mut groups = BTreeMap::new();
				clauses.iter()
					.map(|(a, b)| (Some(*a), *b))
					.chain(std::iter::once((None, default)))
					.for_each(|(k, addr)| groups.entry(addr).or_insert_with(Vec::new).push(k));

				// It's tricky to know when a switch ends.
				// First, check if there exists a break.
				// There exist no labeled break, so any jump from [here..last_case] must be a break, so let's try to find that.
				let last_case = *groups.keys().next_back().unwrap();
				let mut end = None;
				for (_, insn) in self.asm.range(range.start..last_case) {
					insn.labels(|a| if a >= last_case { end = end.max(Some(a)) });
				}

				let mut branches = Vec::new();

				// Skip empty trailing default case.
				let ranges = util::ranges(groups.keys().copied(), end.unwrap_or(last_case));
				for (values, inner) in groups.values().zip(ranges) {
					let body = self.block(inner, end)?;
					branches.push((values.clone(), body));
				}

				// If we found a break, no problem, we're done.
				if let Some(end) = end {
					// But let's remove empty trailing default blocks.
					if let Some((lastk, lastv)) = branches.last() {
						if lastk == &[None] && lastv.is_empty() {
							branches.pop();
						}
					}
					range.start = end;
					return Ok(Stmt::Switch(expr.clone(), branches))
				}

				// No luck, we'll need less precise heuristics.
				// There are cases where the last case contains a break. No, I don't know why.
				// But we need to look forward through the whole range to find that.

				let mut range2 = range.clone();
				while range2.start < range2.end {
					let mut iter = self.asm.range(range2.clone());
					if let Some((_, FlowInsn::Goto(a))) = iter.next() {
						if let Some((b, _)) = iter.next() {
							if *a == *b {
								end = Some(*a);
								break;
							}
						}
					}
					if self.stmt(&mut range2, None).is_err() {
						break;
					}
				}

				if let Some(end) = end {
					// We did find a break; parse until there. Shame that we need to parse twice, but oh well.
					let body = self.block(last_case..end, Some(end))?;
					branches.last_mut().unwrap().1 = body;
					range.start = end;
					return Ok(Stmt::Switch(expr.clone(), branches))
				} else {
					// No break, last case is empty.
					// Remove it if it's default.
					if let Some((lastk, lastv)) = branches.last() {
						if lastk == &[None] && lastv.is_empty() {
							branches.pop();
						}
					}
					range.start = last_case;
					return Ok(Stmt::Switch(expr.clone(), branches))
				}
			}

			FlowInsn::Insn(ref insn) => Stmt::Insn(insn.clone()),
		})
	}

	fn advance(&self, range: Range<usize>) -> Range<usize> {
		match self.asm.range(range.clone()).nth(1) {
			Some((addr, _)) => *addr..range.end,
			None => range.end..range.end,
		}
	}

	fn find_jump_before(&self, addr: usize, brk: Option<usize>) -> Option<(usize, usize)> {
		if let Some((addr, FlowInsn::Goto(target))) = self.asm.range(..addr).next_back() {
			if Some(*target) != brk {
				return Some((*addr, *target))
			}
		}
		None
	}
}
