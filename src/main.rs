use actix_web::{HttpServer, App, get, web::{self, Data, Path}, middleware, Responder, HttpResponse, HttpRequest, body::BoxBody, ResponseError};
use kaiseki::ed6::Archives;

pub mod ed6 {
	pub mod magic;
	pub mod scena;
}

#[derive(Debug)]
pub struct Error(eyre::Error);

impl std::fmt::Display for Error {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		self.0.fmt(f)
	}
}

impl<E: Into<eyre::Error>> From<E> for Error {
	fn from(e: E) -> Self {
		Error(e.into())
	}
}

impl ResponseError for Error {
}

pub type Result<T, E=Error> = std::result::Result<T, E>;

#[derive(Debug)]
pub struct Html(String);
impl Responder for Html {
	type Body = BoxBody;
	fn respond_to(self, _: &HttpRequest) -> HttpResponse<Self::Body> {
		HttpResponse::Ok()
			.content_type("text/html")
			.body(self.0)
	}
}

#[derive(Debug)]
pub struct Image(image::RgbaImage);
impl Responder for Image {
	type Body = BoxBody;
	fn respond_to(self, _: &HttpRequest) -> HttpResponse<Self::Body> {
		let mut data = Vec::new();
		self.0.write_to(&mut std::io::Cursor::new(&mut data), image::ImageOutputFormat::Png).unwrap();
		HttpResponse::Ok()
			.content_type("image/png")
			.body(data)
	}
}

#[get("/magic")]
async fn magic(arch: Data<Archives>) -> Result<Html> {
	let data = arch.get_compressed_by_name(0x2, b"T_MAGIC ._DT")?.1;
	let magics = kaiseki::ed6::magic::Magic::read(&data)?;
	let doc = ed6::magic::render(&magics);
	Ok(Html(doc.render_to_string()))
}

#[get("/scena/{name:\\w{1,8}}")]
async fn scena(arch: Data<Archives>, name: Path<String>) -> Result<Option<Html>> {
	let asm = false;
	let mut s = kaiseki::ByteString(*b"        ._SN");
	s[..name.len()].copy_from_slice(name.as_bytes());
	let data = match arch.get_compressed_by_name(0x1, s) {
		Ok(d) => d,
		Err(kaiseki::ed6::archive::Error::InvalidName { .. } ) => return Ok(None),
		Err(e) => return Err(e.into()),
	}.1;

	let scena = kaiseki::ed6::scena::read(&data)?;
	let doc = ed6::scena::render(&scena, asm);
	Ok(Some(Html(doc.render_to_string())))
}

#[get("/fc/ui/{name}.png")]
async fn ui_png(arch: Data<Archives>, name: Path<String>) -> Result<Option<Image>> {
	let low = false;
	use kaiseki::image::{self, Format};
	let (info1, info2) = match &name[..] {
		"icon1" => ((b"C_ICON1 ._CH", 256, 256, Format::Rgba4444), (b"H_ICON1 ._CH", 512, 512, Format::Rgba4444)),
		"icon2" => ((b"C_ICON2 ._CH", 256, 256, Format::Rgba4444), (b"H_ICON2 ._CH", 512, 512, Format::Rgba4444)),
		_ => return Ok(None)
	};

	let (name, width, height, format) = if low { info1 } else { info2 };

	let data = arch.get_compressed_by_name(0x0, kaiseki::ByteString(*name))?.1;
	let image = image::read(&data, width, height, format)?;
	Ok(Some(Image(image)))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
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

	HttpServer::new(|| {
		App::new()
			.wrap(middleware::Compress::default())
			.service(
				actix_files::Files::new("/assets", concat!(env!("CARGO_MANIFEST_DIR"), "/assets"))
				.show_files_listing()
				.redirect_to_slash_directory()
			)
			.service(
				web::scope("/fc")
				.app_data(Data::new(Archives::new("data/fc")))
				.service(magic)
				.service(scena)
				.service(ui_png)
			)
	})
	.bind(("127.0.0.1", 8000))?
	.run()
	.await
}
