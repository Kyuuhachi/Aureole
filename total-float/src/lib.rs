macro_rules! float {
	($f:ident, $F:ident) => {
		#[derive(Copy, Clone)]
		pub struct $F(pub $f);

		impl std::fmt::Debug for $F {
			fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
				self.0.fmt(f)
			}
		}

		impl From<$f> for $F {
			fn from(value: $f) -> Self {
				$F(value)
			}
		}

		impl From<$F> for $f {
			fn from(value: $F) -> Self {
				value.0
			}
		}

		impl PartialEq for $F {
			fn eq(&self, other: &Self) -> bool {
				self.cmp(other).is_eq()
			}
		}

		impl Eq for $F {}

		impl PartialOrd for $F {
			fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
				Some(self.cmp(other))
			}
		}

		impl Ord for $F {
			fn cmp(&self, other: &Self) -> std::cmp::Ordering {
				$f::total_cmp(&self.0, &other.0)
			}
		}

		impl std::hash::Hash for $F {
			fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
				self.0.to_bits().hash(state);
			}
		}
	}
}

float!(f32, F32);
float!(f64, F64);
