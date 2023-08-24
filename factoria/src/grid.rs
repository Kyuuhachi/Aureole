// Adapted from https://github.com/nushell/nushell/blob/79000aa/crates/nu-term-grid/src/grid.rs

use std::fmt::{self, Display};
use unicode_width::UnicodeWidthChar;

fn strwidth(text: &str) -> usize {
	let mut keep = true;
	let mut width = 0;
	for c in text.chars() {
		match c {
			'\x1B' => keep = false,
			'm' if !keep => keep = true,
			c if keep => width += c.width().unwrap_or(0),
			_ => {}
		}
	}
	width
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Alignment { Left, Right }

#[derive(Clone, PartialEq, Eq)]
pub struct Cell {
	text: String,
	width: usize,
	alignment: Alignment,
}

impl Cell {
	pub fn left(text: String) -> Cell {
		Self {
			width: strwidth(&text),
			text,
			alignment: Alignment::Left,
		}
	}

	pub fn right(text: String) -> Cell {
		Self {
			width: strwidth(&text),
			text,
			alignment: Alignment::Right,
		}
	}
}

impl fmt::Debug for Cell {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match self.alignment {
			Alignment::Left  => write!(f, "Cell::left({:?})", &self.text),
			Alignment::Right => write!(f, "Cell::right({:?})", &self.text),
		}
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Orientation { Horizontal, Vertical }

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Columns {
	pub rows: usize,
	pub cols: Vec<usize>,
}

impl Columns {
	pub fn total_width(&self) -> usize {
		self.cols.iter().sum()
	}

	pub fn on_columns<T>(
		ncols: usize,
		cells: &[T],
		measure: impl Fn(&T) -> usize,
	) -> Columns {
		let mut cols = vec![0; ncols];
		for (i, c) in cells.iter().enumerate() {
			let a = &mut cols[i % ncols];
			*a = measure(c).max(*a);
		}
		Columns {
			rows: cells.len().div_ceil(cols.len()),
			cols,
		}
	}

	pub fn on_rows<T: fmt::Debug>(
		nrows: usize,
		group: usize,
		cells: &[T],
		measure: impl Fn(&T) -> usize,
	) -> Columns {
		assert_eq!(cells.len() % group, 0);
		let mut cols = Vec::new();
		for (i, g) in cells.chunks_exact(group).enumerate() {
			if i % nrows == 0 {
				for _ in g {
					cols.push(0)
				}
			}
			let j = cols.len() - group;
			for (a, c) in cols[j..].iter_mut().zip(g) {
				*a = measure(c).max(*a);
			}
		}
		Columns {
			rows: nrows,
			cols,
		}
	}

	pub fn fit_width_vertical<T: fmt::Debug>(
		width: usize,
		group: usize,
		cells: &[T],
		measure: impl Fn(&T) -> usize,
	) -> Option<Columns> {
		(1..cells.len())
			.map(|nrows| Self::on_rows(nrows, group, cells, &measure))
			.find(|c| c.total_width() <= width)
	}

	pub fn fit_width_horizontal<T>(
		width: usize,
		group: usize,
		cells: &[T],
		measure: impl Fn(&T) -> usize,
	) -> Option<Columns> {
		let mut prev = None;
		let iter = (group..cells.len()).step_by(group)
			.map(|ncols| Self::on_columns(ncols, cells, &measure));
		for c in iter {
			if c.total_width() > width {
				return prev
			}
			prev = Some(c)
		}
		None
	}
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Grid<'a> {
	columns: Columns,
	cells: &'a [Cell],
	sep: &'a str,
	orientation: Orientation,
	group: usize,
}

impl<'a> Grid<'a> {
	pub fn best_fit(
		width: usize,
		orientation: Orientation,
		group: usize,
		cells: &'a [Cell],
		sep: &'a str,
	) -> Grid<'a> {
		let w = strwidth(sep);
		let mut columns = match orientation {
			Orientation::Horizontal => Columns::fit_width_horizontal(width+w, group, cells, |c| c.width + w),
			Orientation::Vertical   => Columns::fit_width_vertical  (width+w, group, cells, |c| c.width + w),
		}.unwrap_or_else(|| Columns::on_columns(group, cells, |c| c.width + w));
		columns.cols.iter_mut().for_each(|a| *a -= w);
		Grid {
			columns,
			cells,
			sep,
			orientation,
			group,
		}
	}
}

impl fmt::Display for Grid<'_> {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		let cols = self.columns.cols.len();
		let rows = self.columns.rows;
		for y in 0..rows {
			for x in 0..cols {
				let x0 = x/self.group*self.group;
				let x1 = x%self.group;
				let n = match self.orientation {
					Orientation::Horizontal => y * cols + x0 + x1,
					Orientation::Vertical   => y*self.group + rows * x0 + x1,
				};
				let Some(cell) = self.cells.get(n) else { continue };

				let x0 = (x+1)/self.group*self.group;
				let x1 = (x+1)%self.group;
				let n = match self.orientation {
					Orientation::Horizontal => y * cols + x0 + x1,
					Orientation::Vertical   => y*self.group + rows * x0 + x1,
				};
				let has_next = n < self.cells.len() && (x+1) < cols;

				let width = self.columns.cols[x];
				let surplus = width.saturating_sub(cell.width);
				match cell.alignment {
					Alignment::Left if !has_next  => write!(f, "{}", &cell.text)?,
					Alignment::Left  => write!(f, "{}{:surplus$}", &cell.text, "")?,
					Alignment::Right => write!(f, "{:surplus$}{}", "", &cell.text)?,
				};

				if has_next {
					write!(f, "{}", self.sep)?;
				}
			}

			writeln!(f)?;
		}

		Ok(())
	}
}
