use actix_web::{
	HttpServer,
	App,
	Responder,
	ResponseError,
	HttpResponse,
	HttpRequest,
	middleware,
	body::BoxBody,
};
use kaiseki::ed6::Archives;

pub mod ed6 {
	pub mod magic;
	pub mod scena;
}

pub mod app;

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

impl ResponseError for Error {}

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

#[actix_web::main]
async fn main() -> std::io::Result<()> {
	use tracing_subscriber::prelude::*;

	tracing_subscriber::registry()
		.with(tracing_subscriber::fmt::layer())
		.with(tracing_subscriber::EnvFilter::from_default_env())
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
			.wrap(tracing_actix_web::TracingLogger::default())
			.wrap(middleware::Logger::default())
			.service(
				actix_files::Files::new("/assets", concat!(env!("CARGO_MANIFEST_DIR"), "/assets"))
				.show_files_listing()
				.redirect_to_slash_directory()
			)
			.service(app::App {
				arch: Archives::new("data/fc")
			}.into_actix("/fc"))
	})
	.bind(("127.0.0.1", 8000))?
	.run()
	.await
}
