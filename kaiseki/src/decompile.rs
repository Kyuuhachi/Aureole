use std::{ops::Range, collections::{BTreeMap, BTreeSet}, fmt::Debug};
use eyre::Result;
use color_eyre::{Section, SectionExt};
use crate::util;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FlowInsn<E, I> {
	If(E, usize),
	Goto(usize),
	Switch(E, Vec<(u16, usize)>, usize),
	Insn(I),
}

impl<E, I> FlowInsn<E, I> {
	pub fn labels(&self, mut f: impl FnMut(usize)) {
		match self {
			FlowInsn::If(_, target) => f(*target),
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

pub fn decompile<E: Clone + Debug, I: Clone + Debug>(asm: &[(usize, FlowInsn<E, I>)], end: usize) -> Result<Vec<Stmt<E, I>>> {
	decompile_inner(asm, end)
		.with_section(|| {
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
			out.join("\n").header("Code:")
		})
}

pub fn decompile_inner<E: Clone, I: Clone>(asm: &[(usize, FlowInsn<E, I>)], end: usize) -> Result<Vec<Stmt<E, I>>> {
	let asm = BTreeMap::from_iter(asm.iter().map(|(k, v)| (*k, v)));
	let start = asm.keys().next().copied().unwrap_or(end);
	Decompiler { asm }.block(start..end, None)
}

#[derive(Clone)]
struct Decompiler<'a, E, I> {
	asm: BTreeMap<usize, &'a FlowInsn<E, I>>,
}

impl<E: Clone, I: Clone> Decompiler<'_, E, I> {
	#[tracing::instrument(skip(self))]
	fn block(&self, mut range: Range<usize>, brk: Option<usize>) -> Result<Vec<Stmt<E, I>>> {
		let mut out = Vec::new();
		while range.start < range.end {
			out.push(self.stmt(&mut range, brk)?);
		}
		Ok(out)
	}

	#[tracing::instrument(skip(self))]
	fn stmt(&self, range: &mut Range<usize>, brk: Option<usize>) -> Result<Stmt<E, I>> {
		let start = range.start;
		*range = self.advance(range.clone());
		match &self.asm[&start] {
			&&FlowInsn::If(ref expr, jump) => {
				match self.find_jump_before(jump, brk) {
					Some((inner, outer)) if outer == start => {
						let body = self.block(range.start..inner, Some(jump))?;
						range.start = jump;
						Ok(Stmt::While(expr.clone(), body))
					}
					Some((inner, outer)) if outer > jump => {
						let body = self.block(range.start..inner, brk)?;
						let mut cases = vec![(Some(expr.clone()), body)];
						match &self.block(jump..outer, brk)?[..] {
							[Stmt::If(more_cases)] => cases.extend(more_cases.iter().cloned()),
							a => cases.push((None, a.to_owned())),
						}
						range.start = outer;
						Ok(Stmt::If(cases))
					}
					_ => {
						let body = self.block(range.start..jump, brk)?;
						let cases = vec![(Some(expr.clone()), body)];
						range.start = jump;
						Ok(Stmt::If(cases))
					}
				}
			}

			&&FlowInsn::Goto(jump) => {
				eyre::ensure!(Some(jump) == brk, "invalid goto {:?}", jump);
				Ok(Stmt::Break)
			}

			&&FlowInsn::Switch(ref expr, ref clauses, default) => {
				let mut groups = BTreeMap::new();
				clauses.iter()
					.map(|(a, b)| (Some(*a), *b))
					.chain(std::iter::once((None, default)))
					.for_each(|(k, addr)| groups.entry(addr).or_insert_with(Vec::new).push(k));

				let mut end = *groups.keys().next_back().unwrap();
				for jump in groups.keys() {
					if let Some((_, outer)) = self.find_jump_before(*jump, None) {
						end = end.max(outer);
					}
				}

				let mut branches = Vec::new();
				for (values, inner) in groups.values().zip(util::ranges(groups.keys().copied(), end)) {
					branches.push((values.clone(), self.block(inner, Some(end))?));
				}

				range.start = end;
				Ok(Stmt::Switch(expr.clone(), branches))
			}

			FlowInsn::Insn(i) => Ok(Stmt::Insn(i.clone())),
		}
	}

	#[tracing::instrument(skip(self))]
	fn advance(&self, range: Range<usize>) -> Range<usize> {
		match self.asm.range(range.clone()).nth(1) {
			Some((addr, _)) => *addr..range.end,
			None => range.end..range.end,
		}
	}

	#[tracing::instrument(skip(self))]
	fn find_jump_before(&self, addr: usize, brk: Option<usize>) -> Option<(usize, usize)> {
		if let Some((addr, FlowInsn::Goto(target))) = self.asm.range(..addr).next_back() {
			if Some(*target) != brk {
				return Some((*addr, *target))
			}
		}
		None
	}
}
