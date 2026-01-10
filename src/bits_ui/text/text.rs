///
///
/// What do i need?
/// text + box -> letter locations
///
/// each frame, each letter checks it's parent
/// the parent has a map (i32, i32) -> LetterState
/// if it's no longer blessed it destroys itself
/// the parent is responsible for spawning new letters and updating it's state but NOT despawning them
///
///
///
struct StfuRust;

// struct Text {
//     pub text_system: Option<BoxedSystem<(), String>>,
// }
