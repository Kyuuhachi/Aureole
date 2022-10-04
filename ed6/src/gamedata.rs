use std::io;

#[derive(derive_more::Deref)]
pub struct GameData {
	#[deref]
	data: Box<dyn GameDataImpl + Sync>,
}

impl GameData {
    pub fn new(data: impl GameDataImpl + Sync + 'static) -> Self { Self { data: Box::new(data) } }
}

pub trait GameDataImpl {
	fn name(&self, v: [u8; 4]) -> io::Result<&str>;
	fn index(&self, name: &str) -> io::Result<[u8; 4]>;
	fn get(&self, name: &str) -> io::Result<&[u8]>;
	// This is not a great abstraction, but it's the best I can come up with right now
	fn get_decomp(&self, name: &str) -> io::Result<Vec<u8>> {
		self.get(name).map(|a| a.to_owned())
	}

	fn list(&self) -> Box<dyn Iterator<Item=&str> + '_>;
}
