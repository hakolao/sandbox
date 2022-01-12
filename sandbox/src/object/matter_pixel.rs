/// Pixel consisting of matter & its corresponding color
/// Object's pixel part may be of wood, but color could vary...
#[derive(Debug, Clone, Copy, Hash, Ord, PartialOrd, Eq, PartialEq)]
pub struct MatterPixel {
    pub matter: u32,
    pub color_index: usize,
    pub is_alive: bool,
}

impl MatterPixel {
    pub fn zero(empty_matter: u32) -> MatterPixel {
        MatterPixel {
            matter: empty_matter,
            color_index: 0,
            is_alive: false,
        }
    }
}
