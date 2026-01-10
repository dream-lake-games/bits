use bevy::prelude::Color;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Palatte {
    Black,
    Midnight,
    Dusk,
    Twilight,
    Evening,
    Violet,
    Amethyst,
    Lilac,
    Sakura,
    Blush,
    Mist,
    Snow,
    Cream,
    Honey,
    Apricot,
    Coral,
    Ember,
    Fuchsia,
    Rose,
    Berry,
    Wine,
    Jam,
}

impl Palatte {
    pub fn to_color(self) -> Color {
        let (r, g, b) = match self {
            Palatte::Black => (0, 0, 0),
            Palatte::Midnight => (10, 18, 34),
            Palatte::Dusk => (38, 51, 76),
            Palatte::Twilight => (53, 64, 121),
            Palatte::Evening => (81, 87, 178),
            Palatte::Violet => (106, 72, 255),
            Palatte::Amethyst => (163, 117, 255),
            Palatte::Lilac => (204, 153, 255),
            Palatte::Sakura => (255, 179, 248),
            Palatte::Blush => (255, 223, 250),
            Palatte::Mist => (255, 238, 255),
            Palatte::Snow => (255, 255, 255),
            Palatte::Cream => (255, 252, 184),
            Palatte::Honey => (255, 228, 167),
            Palatte::Apricot => (255, 186, 134),
            Palatte::Coral => (255, 140, 102),
            Palatte::Ember => (255, 104, 90),
            Palatte::Fuchsia => (255, 35, 90),
            Palatte::Rose => (198, 32, 87),
            Palatte::Berry => (107, 19, 64),
            Palatte::Wine => (64, 11, 47),
            Palatte::Jam => (42, 7, 43),
        };
        Color::srgb_u8(r, g, b)
    }
}
