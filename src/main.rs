use kaiseki::ed6::Archives;
use kaiseki::ed6::magic::*;
use rocket::State;
use rocket::http::Status;
use rocket::response::Responder;
use rocket::response::content::Html;

pub mod ed6 {
	pub mod magic;
}

#[derive(Debug)]
pub struct Error(eyre::Error);

impl<E: Into<eyre::Error>> From<E> for Error {
	fn from(e: E) -> Self {
		Error(e.into())
	}
}

impl<'r> Responder<'r, 'static> for Error {
	fn respond_to(self, _: &'r rocket::Request<'_>) -> rocket::response::Result<'static> {
		eprintln!("{:?}", self.0);
		Err(Status::InternalServerError)
	}
}

pub type Result<T, E=Error> = std::result::Result<T, E>;

#[rocket::get("/fc/magic")]
fn fc_magic(arch: &State<Archives>) -> Result<Html<String>> {
	let data = arch.get_compressed_by_name(0x2, *b"T_MAGIC ._DT")?.1;
	let magics = Magic::read(&data)?;
	let doc = ed6::magic::render(&magics);
	Ok(Html(doc.render_to_string()))
}

#[rocket::launch]
fn rocket() -> _ {
	color_eyre::install().unwrap();
	rocket::build()
		.manage(Archives::new("data/fc"))
		.mount("/", rocket::routes![fc_magic])
}
