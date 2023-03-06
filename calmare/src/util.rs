use themelios::types::Game;
use themelios_archive::lookup::{Lookup, ED7Lookup};

pub fn default_lookup(game: Game) -> &'static dyn Lookup {
	use Game::*;
	use themelios_archive_prebuilt as pb;
	match game {
		Fc | FcKai => &*pb::FC,
		FcEvo => &*pb::FC_EVO,
		Sc | ScKai => &*pb::SC,
		ScEvo => &*pb::SC_EVO,
		Tc | TcKai => &*pb::TC,
		TcEvo => &*pb::TC_EVO,
		Zero | ZeroEvo | ZeroKai |
		Ao | AoEvo | AoKai => &ED7Lookup
	}
}
