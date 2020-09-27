/// A rectangle.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct Rectangle {
	x: i32,
	y: i32,
	width: u32,
	height: u32
}

impl Rectangle {
	/// Create a rectangle from X, Y coordinates and the width and height.
	pub fn from_xywh(x: i32, y: i32, width: u32, height: u32) -> Self {
		Self { x, y, width, height }
	}

	/// Get the X location of the rectangle.
	pub fn x(&self) -> i32 {
		self.x
	}

	/// Get the Y location of the rectangle.
	pub fn y(&self) -> i32 {
		self.y
	}

	/// Get the width of the rectangle.
	pub fn width(&self) -> u32 {
		self.width
	}

	/// Get the height of the rectangle.
	pub fn height(&self) -> u32 {
		self.height
	}
}
