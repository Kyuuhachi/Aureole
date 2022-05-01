use std::{borrow::Cow, str::FromStr};

use percent_encoding::percent_decode_str;
use kaiseki::{ed6::{Archives, magic::Magic}, util::ByteString};
use crate::{Result, Html, Image, ed6};

pub struct App {
	arch: Archives,
	tables: Tables,
}

pub struct Tables {
	pub magic: Vec<Magic>,
}

impl Tables {
	pub fn read(arch: &Archives) -> Result<Self> {
		Ok(Tables {
			magic: Magic::read(&arch.get_compressed_by_name(0x2, b"T_MAGIC ._DT")?.1)?,
		})
	}
}

impl App {
	pub fn new(path: &str) -> Result<Self> {
		let arch = Archives::new(path);
		Ok(Self {
			tables: Tables::read(&arch)?,
			arch,
		})
	}

	#[tracing::instrument(skip(self))]
	pub async fn magic(&self) -> Result<Html> {
		let doc = ed6::magic::render(&self.tables.magic);
		Ok(Html(doc))
	}

	#[tracing::instrument(skip(self))]
	pub async fn scena(&self, name: &str, asm: bool) -> Result<Option<Html>> {
		let mut s = ByteString(*b"        ._SN");
		s[..name.len()].copy_from_slice(name.as_bytes());
		let data = match self.arch.get_compressed_by_name(0x1, s) {
			Ok(d) => d,
			Err(kaiseki::ed6::archive::Error::InvalidName { .. } ) => return Ok(None),
			Err(e) => return Err(e.into()),
		}.1;

		let scena = kaiseki::ed6::scena::read(&data)?;
		let doc = ed6::scena::render(&scena, &self.arch, &self.tables, asm);
		Ok(Some(Html(doc)))
	}

	#[tracing::instrument(skip(self))]
	pub async fn ui_png(&self, name: &str, low: bool) -> Result<Option<Image>> {
		use kaiseki::image::{self, Format};
		let (info1, info2) = match name {
			"btn01"   => ((0x00, b"C_BTN01 ._CH", 256, 256, Format::Rgba4444),
			              (0x00, b"H_BTN01 ._CH", 512, 512, Format::Rgba4444)),
			"btn02"   => ((0x00, b"C_BTN02 ._CH", 256, 256, Format::Rgba4444),
			              (0x00, b"H_BTN02 ._CH", 512, 512, Format::Rgba4444)),
			"camp01"  => ((0x00, b"C_CAMP01._CH", 256, 256, Format::Rgba4444),
			              (0x00, b"H_CAMP01._CH", 512, 512, Format::Rgba4444)),
			"camp02"  => ((0x00, b"C_CAMP02._CH", 256, 256, Format::Rgba1555),
			              (0x00, b"H_CAMP02._CH", 512, 512, Format::Rgba1555)),
			"camp03"  => ((0x00, b"C_CAMP03._CH", 256, 256, Format::Rgba1555),
			              (0x00, b"H_CAMP03._CH", 512, 512, Format::Rgba1555)),
			"camp04"  => ((0x00, b"C_CAMP04._CH", 256, 256, Format::Rgba1555),
			              (0x00, b"H_CAMP04._CH", 512, 512, Format::Rgba1555)),
			"cmps"    => ((0x00, b"C_CMPS  ._CH", 256, 256, Format::Rgba4444),
			              (0x00, b"H_CMPS  ._CH", 512, 512, Format::Rgba4444)),
			"cook"    => ((0x00, b"C_COOK  ._CH", 256, 256, Format::Rgba4444),
			              (0x00, b"C_COOK  ._CH", 256, 256, Format::Rgba4444)), // no H exists
			"emotio"  => ((0x00, b"C_EMOTIO._CH", 256, 256, Format::Rgba4444),
			              (0x00, b"H_EMOTIO._CH", 512, 512, Format::Rgba4444)),
			"icon1"   => ((0x00, b"C_ICON1 ._CH", 256, 256, Format::Rgba4444),
			              (0x00, b"H_ICON1 ._CH", 512, 512, Format::Rgba4444)),
			"icon2"   => ((0x00, b"C_ICON2 ._CH", 256, 256, Format::Rgba4444),
			              (0x00, b"H_ICON2 ._CH", 512, 512, Format::Rgba4444)),
			"mouse"   => ((0x00, b"C_MOUSE ._CH", 256, 256, Format::Rgba4444),
			              (0x00, b"H_MOUSE ._CH", 512, 512, Format::Rgba4444)),
			"note1"   => ((0x00, b"C_NOTE1 ._CH", 256, 256, Format::Rgba4444),
			              (0x00, b"H_NOTE1 ._CH", 512, 512, Format::Rgba4444)),
			"waku1"   => ((0x00, b"C_WAKU1 ._CH", 256, 256, Format::Rgba4444),
			              (0x00, b"H_WAKU1 ._CH", 512, 512, Format::Rgba4444)),
			"waku3"   => ((0x00, b"C_WAKU3 ._CH", 256, 256, Format::Rgba4444),
			              (0x00, b"C_WAKU3 ._CH", 256, 256, Format::Rgba4444)),

			"battle"  => ((0x0F, b"BATTLE  ._CH", 256, 256, Format::Rgba4444),
			              (0x0F, b"HBATTLE ._CH", 512, 512, Format::Rgba4444)),
			"battle2" => ((0x0F, b"BATTLE2 ._CH", 256, 256, Format::Rgba4444),
			              (0x0F, b"HBATTLE2._CH", 512, 512, Format::Rgba4444)),
			"battle3" => ((0x0F, b"BATTLE3 ._CH", 256, 256, Format::Rgba4444),
			              (0x0F, b"HBATTLE3._CH", 512, 512, Format::Rgba4444)),
			"btlinfo" => ((0x0F, b"BTLINFO ._CH", 256, 256, Format::Rgba4444),
			              (0x0F, b"HBTLINFO._CH", 512, 512, Format::Rgba4444)),
			"btlmenu" => ((0x0F, b"BTLMENU ._CH", 256, 256, Format::Rgba4444),
			              (0x0F, b"HBTLMENU._CH", 512, 512, Format::Rgba4444)),
			_ => return Ok(None)
		};

		let (arch, fname, width, height, format) = if low { info1 } else { info2 };

		let data = self.arch.get_compressed_by_name(arch, ByteString(*fname))?.1;
		let image = image::read(&data, width, height, format)?;
		if name == "emotio" {
			Ok(Some(Image(image::linearize(image, 8, 8, 64, 1))))
		} else {
			Ok(Some(Image(image)))
		}
	}

	#[tracing::instrument(skip(self))]
	pub async fn ui_index(&self) -> Result<Html> {
		let doc = choubun::document(|doc| {
			for s in [
				"btn01", "btn02",
				"camp01", "camp02", "camp03", "camp04",
				"cmps",
				"cook",
				"emotio",
				"icon1", "icon2",
				"mouse",
				"note1",
				"waku1", "waku3",
				"battle", "battle2", "battle3",
				"btlinfo", "btlmenu",
			] {
				doc.body.node("h1", |a| a.text(s));
				doc.body.leaf("img", |a| a.attr("src", format!("{s}.png")));
			}
		});
		Ok(Html(doc))
	}
}

trait QueryArg: Default {
	fn parse(&mut self, val: Option<&str>) -> Option<()>;
}

impl QueryArg for bool {
	fn parse(&mut self, val: Option<&str>) -> Option<()> {
		*self = match val {
			None | Some("1") => Some(true),
			Some("0") => Some(false),
			_ => None,
		}?;
		Some(())
	}
}

impl<T: FromStr> QueryArg for Option<T> {
	fn parse(&mut self, val: Option<&str>) -> Option<()> {
		*self = Some(val?.parse().ok()?);
		Some(())
	}
}

impl<T: FromStr> QueryArg for Vec<T> {
	fn parse(&mut self, val: Option<&str>) -> Option<()> {
		self.push(val?.parse().ok()?);
		Some(())
	}
}

impl App {
	pub fn into_actix(self, path: &str) -> actix_web::Scope {
		fn urldecode(v: &str) -> Option<Cow<str>> {
			percent_decode_str(v).decode_utf8().ok()
		}

		use actix_web::{HttpRequest, web, error, Responder};
		web::scope(path)
		.app_data(self)

		.route("/magic", web::get().to({
			async fn magic(req: HttpRequest) -> Result<impl Responder, error::Error> {
				let app = req.app_data::<App>().unwrap();
				Ok(app.magic().await)
			}
			magic
		}))

		.route("/scena/{name:\\w{1,8}}", web::get().to({
			async fn scena(req: HttpRequest) -> Result<impl Responder, error::Error> {
				let _name = req.match_info().get("name").unwrap();
				let mut _asm = <bool as Default>::default();

				if let Some(query) = req.uri().query() {
					for part in query.split('&') {
						(|| -> Option<()> {
							let mut iter = part.splitn(2, '=');
							let k = urldecode(iter.next().unwrap())?;
							if k == "asm" {
								let v = match iter.next() {
									Some(v) => Some(urldecode(v)?),
									None => None,
								};
								_asm.parse(v.as_deref())?;
							}
							Some(())
						})().ok_or_else(|| error::ErrorBadRequest(part.to_owned()))?;
					}
				}

				let app = req.app_data::<App>().unwrap();
				Ok(app.scena(_name, _asm).await)
			}
			scena
		}))

		.route("/ui/{name}.png", web::get().to({
			async fn ui_png(req: HttpRequest) -> Result<impl Responder, error::Error> {
				let _name = req.match_info().get("name").unwrap();
				let mut _low = <bool as Default>::default();

				if let Some(query) = req.uri().query() {
					for part in query.split('&') {
						(|| -> Option<()> {
							let mut iter = part.splitn(2, '=');
							let k = urldecode(iter.next().unwrap())?;
							if k == "low" {
								let v = match iter.next() {
									Some(v) => Some(urldecode(v)?),
									None => None,
								};
								_low.parse(v.as_deref())?;
							}
							Some(())
						})().ok_or_else(|| error::ErrorBadRequest(part.to_owned()))?;
					}
				}

				let app = req.app_data::<App>().unwrap();
				Ok(app.ui_png(_name, _low).await)
			}
			ui_png
		}))

		.route("/ui/", web::get().to({
			async fn ui_index(req: HttpRequest) -> Result<impl Responder, error::Error> {
				let app = req.app_data::<App>().unwrap();
				Ok(app.ui_index().await)
			}
			ui_index
		}))
	}
}
