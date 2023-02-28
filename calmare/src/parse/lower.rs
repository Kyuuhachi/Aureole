use super::diag::*;
use super::ast::*;
use Spanned as S;

struct Error;

macro f {
	($p:pat $(if $e:expr)? => $v:expr) => { |a| {
		match a {
			$p $(if $e)? => Some($v),
			_ => None
		}
	} },
	($p:pat $(if $e:expr)? ) => { |a| {
		match a {
			$p $(if $e)? => true,
			_ => false
		}
	} },
}

macro t($k:ident) {
	f!(Term::$k(v) => v.clone())
}

pub fn lower(decls: &[S<Decl>]) {
	let mut ty = None;
	let mut funs = Vec::new();
	let mut datas = Vec::new();
	for d in decls {
		match d {
			S(s, Decl::FileType(g, t)) => {
				if ty.is_some() {
					Diag::error(*s, "duplicate type declaration").emit();
				}
				ty = Some((g, t));
			},
			S(s, Decl::Function(f)) => funs.push(S(*s, f)),
			S(s, Decl::Data(d)) => datas.push(S(*s, d)),
		}
	}

	let Some((g, t)) = ty else {
		Diag::error(Span::new_at(0), "missing type declaration").emit();
		return;
	};

	match t {
		FileType::Scena => {
			lower_ed6_scena(&datas);
		}
	}
}

macro parse_block(
	$e:ident => {
		$($k:ident : $v:expr),* $(,)?
	} $(else {
		$($k2:ident => $v2:expr),* $(,)?
	})?
) {
	// $(let mut $k = None;)*
	let Some(body) = &$e.body else {
		Diag::error($e.eol, "a body is required here").emit();
		continue
	};
	for l in body {
		match l.head.key.1.as_str() {
			$(stringify!($k) => {
			})*
			$($(stringify!($k2) => {
				let () = (($v2))(l);
			})*)?
			_ => {
				Diag::error(l.head.key.0, "unknown field")
					.note(l.head.key.0, format_args!("expected {}", [
						$(concat!("'", stringify!($k), "'"),)*
						$($(concat!("'", stringify!($k2), "'"),)*)?
					].join(", ")))
					.emit();
				continue
			}
		}
	}
}

fn lower_ed6_scena(datas: &[S<&Data>]) {
	for S(s, d) in datas {
		match d.head.key.1.as_str() {
			"scena" => {
				// let mut scp = Vec::new();
				parse_block!(d => {
					name: |p| Ok((p(t!(String))?, p(t!(String))?)),
					town: |p| p(t!(Town)),
					bgm:  |p| p(t!(Bgm)),
					item: |p| p(t!(Fn)),
				} else {
					scp => |l| {

					}
				});
			}
			"entry" => {
				parse_block!(d => {
					pos:       |p| p(pos3),
					chr:       |p| p(int!(None)),
					angle:     |p| p(int!(Deg)),
					cam_from:  |p| p(pos3),
					cam_at:    |p| p(pos3),
					cam_zoom:  |p| p(int!(None)),
					cam_pers:  |p| p(int!(None)),
					cam_deg:   |p| p(int!(Deg)),
					cam_limit: |p| Ok((p(int!(Deg))?, p(int!(Deg))?)),
					north:     |p| p(int!(Deg)),
					flags:     |p| p(int!(None)),
					town:      |p| p(t!(Town)),
					init:      |p| p(t!(Fn)),
					reinit:    |p| p(t!(Fn)),
				});
			}
			"chcp" => {

			}
			"npc" => {
				parse_block!(d => {
					name:  |p| p(t!(String)),
					pos:   |p| p(pos3),
					angle: |p| p(int!(Deg)),
					x:     |p| p(int!(None)),
					pt:    |p| p(t!(Chcp)),
					no:    |p| p(int!(None)),
					bs:    |p| p(t!(Chcp)),
					flags: |p| p(int!(None)),
					init:  |p| p(t!(Fn)),
					talk:  |p| p(t!(Fn)),
				});
			}
			"monster" => {
				parse_block!(d => {
					name:   |p| p(t!(String)),
					pos:    |p| p(pos3),
					angle:  |p| p(int!(Deg)),
					unk1:   |p| p(int!(None)),
					flags:  |p| p(int!(None)),
					unk2:   |p| p(int!(None)),
					battle: |p| p(t!(Battle)),
					flag:   |p| p(t!(Flag)),
					unk3:   |p| p(int!(None)),
				});
			}
			"trigger" => {
				parse_block!(d => {
					pos1: |p| p(pos3),
					pos2: |p| p(pos3),
					func: |p| p(t!(Fn)),
					unk1: |p| p(int!(None)),
				});
			}
			"look_point" => {
				parse_block!(d => {
					pos:        |p| p(pos3),
					radius:     |p| p(int!(Mm)),
					bubble_pos: |p| p(pos3),
					flags:      |p| p(int!(None)),
					func:       |p| p(t!(Fn)),
					unk1:       |p| p(int!(None)),
				});
			}
			_ => {
				Diag::error(d.head.key.0, "unknown declaration")
					.note(d.head.key.0, "expected 'scena', 'entry', 'chcp', 'npc', 'monster', 'trigger', 'look_point'")
					.emit();
			}
		}
	}

}


fn find<T, Y>(xs: &mut Vec<&T>, mut f: impl FnMut(&T) -> Option<Y>) -> Option<Y> {
	if let Some(a) = xs.iter().position(|a| f(*a).is_some()) {
		f(xs.remove(a))
	} else {
		None
	}
}
