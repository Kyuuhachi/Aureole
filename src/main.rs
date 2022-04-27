use kaiseki::ed6::Archives;
use rocket::State;
use rocket::http::Status;
use rocket::response::Responder;
use rocket::response::content::Html;

pub mod ed6 {
	pub mod magic;
	pub mod scena;
}

#[derive(Debug)]
pub enum Error {
	Error(eyre::Error),
	NotFound,
}

impl<E: Into<eyre::Error>> From<E> for Error {
	fn from(e: E) -> Self {
		Error::Error(e.into())
	}
}

impl<'r> Responder<'r, 'static> for Error {
	fn respond_to(self, _: &'r rocket::Request<'_>) -> rocket::response::Result<'static> {
		Err(match self {
			Error::Error(e) => {
				eprintln!("{:?}", e);
				Status::InternalServerError
			},
			Error::NotFound => Status::NotFound,
		})
	}
}

pub type Result<T, E=Error> = std::result::Result<T, E>;

#[derive(Debug)]
pub struct Image(image::RgbaImage);
impl<'r> Responder<'r, 'static> for Image {
	fn respond_to(self, req: &'r rocket::Request<'_>) -> rocket::response::Result<'static> {
		let mut data = Vec::new();
		self.0.write_to(&mut std::io::Cursor::new(&mut data), image::ImageOutputFormat::Png).unwrap();
		rocket::response::content::Custom(rocket::http::ContentType::PNG, data).respond_to(req)
	}
}

#[rocket::get("/fc/magic")]
fn fc_magic(arch: &State<Archives>) -> Result<Html<String>> {
	let data = arch.get_compressed_by_name(0x2, b"T_MAGIC ._DT")?.1;
	let magics = kaiseki::ed6::magic::Magic::read(&data)?;
	let doc = ed6::magic::render(&magics);
	Ok(Html(doc.render_to_string()))
}

#[rocket::get("/fc/scena/<name>?<asm>")]
fn fc_scena(arch: &State<Archives>, name: &str, asm: bool) -> Result<Html<String>> {
	if name.len() > 8 { return Err(Error::NotFound) }
	let mut s = kaiseki::ByteString(*b"        ._SN");
	s[..name.len()].copy_from_slice(name.as_bytes());
	let data = match arch.get_compressed_by_name(0x1, s) {
		Ok(d) => d,
		Err(kaiseki::ed6::archive::Error::InvalidName { .. } ) => return Err(Error::NotFound),
		Err(e) => return Err(e.into()),
	}.1;

	let scena = kaiseki::ed6::scena::read(&data)?;
	let doc = ed6::scena::render(&scena, asm);
	Ok(Html(doc.render_to_string()))
}

#[rocket::get("/fc/ui/<name>?<low>")]
fn fc_ui_png(arch: &State<Archives>, name: &str, low: bool) -> Result<Image> {
	use kaiseki::image::{self, Format};
	let (info1, info2) = match name {
		"icon1.png" => ((b"C_ICON1 ._CH", 256, 256, Format::Rgba4444), (b"H_ICON1 ._CH", 512, 512, Format::Rgba4444)),
		"icon2.png" => ((b"C_ICON2 ._CH", 256, 256, Format::Rgba4444), (b"H_ICON2 ._CH", 512, 512, Format::Rgba4444)),
		_ => return Err(Error::NotFound)
	};

	let (name, width, height, format) = if low { info1 } else { info2 };

	let data = arch.get_compressed_by_name(0x0, kaiseki::ByteString(*name))?.1;
	let image = image::read(&data, width, height, format)?;
	Ok(Image(image))
}

#[rocket::launch]
fn rocket() -> _ {
	use tracing_subscriber::{prelude::*, EnvFilter};

	tracing_subscriber::registry()
		.with(tracing_subscriber::fmt::layer())
		.with(EnvFilter::from_default_env())
		.with(tracing_error::ErrorLayer::default())
		.init();

	color_eyre::config::HookBuilder::default()
		.add_frame_filter(Box::new(|frames| {
			if let Some(a) = frames.iter().rposition(|f| matches!(&f.filename, Some(a) if a.starts_with(env!("CARGO_MANIFEST_DIR")))) {
				frames.truncate(a+2)
			}
		})).install().unwrap();

	rocket::build()
		.manage(Archives::new("data/fc"))
		.mount("/assets", rocket::fs::FileServer::from(rocket::fs::relative!("assets")))
		.mount("/", rocket::routes![fc_magic, fc_scena, fc_ui_png])
}
