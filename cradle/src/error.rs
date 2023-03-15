#[derive(Debug, thiserror::Error)]
pub enum Error {
	#[error("invalid image size")]
	Size,
	#[error("invalid data")]
	Invalid,
	#[error("{source}")]
	Read { #[from] source: hamu::read::Error, backtrace: std::backtrace::Backtrace },
	#[error("{source}")]
	Write { #[from] source: hamu::write::Error, backtrace: std::backtrace::Backtrace },
}
