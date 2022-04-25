use std::{ops::Range, collections::BTreeMap};
use eyre::Result;
use crate::util;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FlowInsn<E, I> {
	If(E, usize),
	Goto(usize),
	Switch(E, Vec<(u16, usize)>, usize),
	Insn(I),
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

pub fn decompile<E: Clone, I: Clone>(asm: &[(usize, FlowInsn<E, I>)], end: usize) -> Result<Vec<Stmt<E, I>>> {
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
				let (inner, target) = self.find_target(range.start..jump, brk);
				range.start = jump;
				if target == Some(start) {
					return Ok(Stmt::While(expr.clone(), self.block(inner, Some(jump))?))
				}

				let mut cases = vec![(Some(expr.clone()), self.block(inner, brk)?)];
				if let Some(target) = target {
					range.start = target;
					match &self.block(jump..target, brk)?[..] {
						[Stmt::If(more_cases)] => cases.extend(more_cases.iter().cloned()),
						a => cases.push((None, a.to_owned())),
					}
				};
				Ok(Stmt::If(cases))
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

				eyre::ensure!(groups.values().next_back().unwrap() == &[None], "this switch statement is not supported");
				let mut vals = groups.keys().copied();
				let end = vals.next_back().unwrap();
				let mut branches = Vec::new();
				for (values, inner) in groups.values().zip(util::ranges(vals, end)) {
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
	fn find_target(&self, range: Range<usize>, brk: Option<usize>) -> (Range<usize>, Option<usize>) {
		if let Some((addr, FlowInsn::Goto(target))) = self.asm.range(range.clone()).next_back() {
			if Some(*target) != brk {
				return (range.start..*addr, Some(*target))
			}
		}
		(range, None)
	}
}
