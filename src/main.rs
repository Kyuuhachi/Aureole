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
		.mount("/", rocket::routes![fc_magic, fc_scena])
}
