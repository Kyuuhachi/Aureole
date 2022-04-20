use anyhow::Result;
use kaiseki::ed6::Archives;
use kaiseki::ed6::magic::*;
use rocket::State;
use rocket::response::content::Html;

pub mod ed6 {
	pub mod magic;
}

type HtmlOut = Result<Html<String>, rocket::response::Debug<anyhow::Error>>;

#[rocket::get("/fc/magic")]
fn fc_magic(arch: &State<Archives>) -> HtmlOut {
	let data = arch.get_compressed_by_name(0x2, *b"T_MAGIC ._DT")?.1;
	let magics = Magic::read(&data)?;
	let doc = ed6::magic::render(&magics);
	let mut s = String::new();
	doc.render(&mut s).unwrap();
	Ok(Html(s))
}

#[rocket::launch]
fn rocket() -> _ {
	rocket::build()
		.manage(Archives::new("data/fc"))
		.mount("/", rocket::routes![fc_magic])
}
