pub mod button;
pub mod letters;
pub mod text;

pub use button::{AnimButton, BUTTON_SIZE, ButtonLetterAnimChild, button_plugin_fn};
pub use letters::{LETTER_SIZE, char_to_letter_anim};
pub use text::{AnimatedText, AnimatedTextSize, text_plugin_fn};
