mod aseprite;

#[allow(dead_code)]
enum AnimNextState<AnimState> {
    Tag(AnimState),
    Remove,
    Despawn,
}

#[allow(dead_code)]
struct AnimStateData<AnimState> {
    tag: String,
    next: AnimNextState<AnimState>,
    fps: f32,
}

#[allow(dead_code)]
trait AnimStateMachine: Sized {
    const FILE_PATH: &str;

    fn get_state_data(&self) -> AnimStateData<Self>;

    fn get_all_state_data() -> Vec<AnimStateData<Self>>;
}

#[allow(dead_code)]
enum AnimPlay {
    Smile,
    Frown,
}

impl AnimStateMachine for AnimPlay {
    const FILE_PATH: &str = "assets/play.aseprite";

    fn get_state_data(&self) -> AnimStateData<Self> {
        match self {
            AnimPlay::Smile => AnimStateData {
                tag: "smile".into(),
                next: AnimNextState::Tag(Self::Smile),
                fps: 16.0,
            },
            AnimPlay::Frown => AnimStateData {
                tag: "frown".into(),
                next: AnimNextState::Tag(Self::Smile),
                fps: 16.0,
            },
        }
    }

    fn get_all_state_data() -> Vec<AnimStateData<Self>> {
        vec![Self::Smile.get_state_data(), Self::Frown.get_state_data()]
    }
}

fn main() -> anyhow::Result<()> {
    use aseprite::ExportBuilder;

    println!("Exporting smile tag with 'fa' prefix included...");
    ExportBuilder::new("assets/play.aseprite", "smile")
        .include_prefix("fa")
        .export()?;
    println!("✓ Exported smile");

    println!("\nExporting frown tag with '_' prefix excluded...");
    ExportBuilder::new("assets/play.aseprite", "frown")
        .exclude_prefix("_")
        .export()?;
    println!("✓ Exported frown");

    println!("\nDone!");
    Ok(())
}

// #[derive(Aseprite)]
// #[aseprite("assets/play.aseprite")]
// enum AnimThisThing {
//     #[next(AnimThisThing::Tag)]
//     #[fps()]
//     Tag,
// }

// enum AnimLikeThis {
//     Tag,
// }
// impl Anim for AnimLikeThis {
//     const FILE = "assets/play.aseprite";

// }
