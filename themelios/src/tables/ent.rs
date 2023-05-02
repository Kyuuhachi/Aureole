use glam::Vec3;
use gospel::read::{Reader, Le as _};
use gospel::write::{Writer, Le as _};
use themelios_common::util::*;
use crate::types::*;

#[derive(Debug, Clone, PartialEq)]
pub struct ED6Ent {
	pub name: TString,
	pub bbox: (Vec3, Vec3),
	pub pos: Vec3,
	pub angle: Angle,
	pub unk1: u16,
	// 1: disabled
	pub flags: u16,
	pub unk2: u16,

	pub dest_name: String,
	pub dest: FileId,
	pub dest_entrance: EntranceId,
	pub unk3: u16,

	pub cam_from: Vec3,
	pub cam_deg: f32,
	pub cam_zoom: f32,
	pub cam_pers: f32,
	pub cam_at: Vec3,
	pub cam_limit: (Angle, Angle),

	pub town: TownId,
	pub unk4: u16,
}

impl ED6Ent {
	pub fn read(data: &[u8]) -> Result<Vec<ED6Ent>, ReadError> {
		let mut f = Reader::new(data);
		let mut table = Vec::new();
		for _ in 0..f.u16()? {
			let name = TString(f.sized_string::<16>()?);
			let bbox = (f.vec3()?, f.vec3()?);
			let pos = f.vec3()?;
			let angle = Angle(f.i16()?);
			let unk1 = f.u16()?;
			let flags = f.u16()?;
			let unk2 = f.u16()?;

			f.slice(16)?; // I'm pretty sure this is junk.
			let dest_name = f.sized_string::<16>()?;
			let dest = FileId(f.u32()?);
			let dest_entrance = EntranceId(cast(f.u16()?)?);
			let unk3 = f.u16()?;

			let cam_from = f.vec3()?;
			let cam_deg = f.f32()?;
			let cam_zoom = f.f32()?;
			let cam_pers = f.f32()?;
			let cam_at = f.vec3()?;
			let cam_limit = (Angle(f.i16()?), Angle(f.i16()?));

			let town = TownId(f.u16()?);
			let unk4 = f.u16()?;

			table.push(ED6Ent {
				name, bbox, pos, angle, unk1, flags, unk2,
				dest_name, dest, dest_entrance, unk3,
				cam_from, cam_deg, cam_zoom, cam_pers, cam_at, cam_limit,
				town, unk4,
			});
		}
		Ok(table)
	}

	pub fn write(table: &[ED6Ent]) -> Result<Vec<u8>, WriteError> {
		let mut f = Writer::new();
		f.u16(cast(table.len())?);
		for a in table {
			f.sized_string::<16>(&a.name.0)?;
			f.vec3(a.bbox.0);
			f.vec3(a.bbox.1);
			f.vec3(a.pos);
			f.i16(a.angle.0);
			f.u16(a.unk1);
			f.u16(a.flags);
			f.u16(a.unk2);

			f.slice(&[0; 16]);
			f.sized_string::<16>(&a.dest_name)?;
			f.u32(a.dest.0);
			f.u16(a.dest_entrance.0 as u16);
			f.u16(a.unk3);

			f.vec3(a.cam_from);
			f.f32(a.cam_deg);
			f.f32(a.cam_zoom);
			f.f32(a.cam_pers);
			f.vec3(a.cam_at);
			f.i16(a.cam_limit.0.0);
			f.i16(a.cam_limit.1.0);

			f.u16(a.town.0);
			f.u16(a.unk4);
		}
		Ok(f.finish()?)
	}
}
