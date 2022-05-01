use kaiseki::{ed6::Archives, util::ByteString};
use crate::{Result, Html, Image, ed6};

pub struct App {
	pub arch: Archives,
}

impl App {
	#[tracing::instrument(skip(self))]
	pub async fn magic(&self) -> Result<Html> {
		let data = self.arch.get_compressed_by_name(0x2, b"T_MAGIC ._DT")?.1;
		let magics = kaiseki::ed6::magic::Magic::read(&data)?;
		let doc = ed6::magic::render(&magics);
		Ok(Html(doc.render_to_string()))
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
		let doc = ed6::scena::render(&scena, &self.arch, asm);
		Ok(Some(Html(doc.render_to_string())))
	}

	#[tracing::instrument(skip(self))]
	pub async fn ui_png(&self, name: &str, low: bool) -> Result<Option<Image>> {
		use kaiseki::image::{self, Format};
		let (info1, info2) = match name {
			"icon1" => ((b"C_ICON1 ._CH", 256, 256, Format::Rgba4444), (b"H_ICON1 ._CH", 512, 512, Format::Rgba4444)),
			"icon2" => ((b"C_ICON2 ._CH", 256, 256, Format::Rgba4444), (b"H_ICON2 ._CH", 512, 512, Format::Rgba4444)),
			_ => return Ok(None)
		};

		let (name, width, height, format) = if low { info1 } else { info2 };

		let data = self.arch.get_compressed_by_name(0x0, ByteString(*name))?.1;
		let image = image::read(&data, width, height, format)?;
		Ok(Some(Image(image)))
	}
}

impl App {
	pub fn into_actix(self, path: &str) -> actix_web::Scope {
		use actix_web::web;
		web::scope(path)
		.app_data(web::Data::new(self))
		.route("/magic",
			web::get().to(|app: web::Data<Self>| async move {
				app.magic().await
			})
		)
		.route("/scena/{name:\\w{1,8}}",
			web::get().to(|app: web::Data<Self>, name: web::Path<String>| async move {
				app.scena(&name, false).await
			})
		)
		.route("/ui/{name}.png",
			web::get().to(|app: web::Data<Self>, name: web::Path<String>| async move {
				app.ui_png(&name, false).await
			})
		)
	}
}
