use bevy::input::common_conditions::input_toggle_active;
use bevy::prelude::*;
use bevy_inspector_egui::bevy_egui::EguiPlugin;
use bevy_inspector_egui::quick::ResourceInspectorPlugin;
use bits::player::{BettingGuessDisplay, PlayerBettingScreen, player_betting_plugin_fn};
use bits::prelude::*;
use bits::window::get_window_plugin_with_title;

#[derive(Resource)]
struct BettingScreenEntity(Entity);

fn startup(mut commands: Commands) {
    spawn_bloom_camera(&mut commands);

    let initial = PlayerBettingScreen::new("How many moons does Jupiter have")
        .with_units("moons")
        .with_seconds_remaining(60.0)
        .with_player_score(10)
        .with_guesses(vec![
            BettingGuessDisplay {
                value: 0,
                owners: vec![],
                my_free_bets: 0,
                my_paid_bets: 0,
            },
            BettingGuessDisplay {
                value: 79,
                owners: vec!["ALICE".into()],
                my_free_bets: 1,
                my_paid_bets: 0,
            },
            BettingGuessDisplay {
                value: 95,
                owners: vec!["BOB".into(), "ME".into()],
                my_free_bets: 1,
                my_paid_bets: 2,
            },
        ]);

    // Insert as resource (edit buffer)
    commands.insert_resource(initial.clone());

    // Spawn as component (display)
    let entity = commands
        .spawn((Name::new("PlayerBettingScreen"), initial))
        .id();

    commands.insert_resource(BettingScreenEntity(entity));
}

fn sync_on_enter(
    keyboard: Res<ButtonInput<KeyCode>>,
    resource: Res<PlayerBettingScreen>,
    entity: Res<BettingScreenEntity>,
    mut screen_q: Query<&mut PlayerBettingScreen>,
) {
    if keyboard.just_pressed(KeyCode::Enter) {
        if let Ok(mut screen) = screen_q.get_mut(entity.0) {
            // Preserve outputs when syncing
            let pending = screen.pending_bet.clone();
            let lock = screen.lock_requested;
            *screen = resource.clone();
            screen.pending_bet = pending;
            screen.lock_requested = lock;
            info!("Synced PlayerBettingScreen from resource");
        }
    }
}

fn log_outputs(mut screen_q: Query<&mut PlayerBettingScreen>) {
    let Ok(mut screen) = screen_q.single_mut() else {
        return;
    };

    if let Some(bet) = screen.pending_bet.take() {
        info!(
            "Bet requested: guess={}, free={}, paid={}",
            bet.guess, bet.num_free, bet.num_paid
        );
        // Simulate server accepting - update the guess display
        for guess in &mut screen.guesses {
            if guess.value == bet.guess {
                guess.my_free_bets = bet.num_free;
                guess.my_paid_bets = bet.num_paid;
            }
        }
    }

    if screen.lock_requested {
        info!("Lock requested!");
        screen.is_locked = true;
        screen.lock_requested = false;
    }
}

fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(get_window_plugin_with_title("Player: Betting"))
                .set(ImagePlugin::default_nearest()),
        )
        .add_plugins(EguiPlugin::default())
        .add_plugins(
            bevy_inspector_egui::quick::WorldInspectorPlugin::default()
                .run_if(input_toggle_active(false, KeyCode::Tab)),
        )
        .add_plugins(ResourceInspectorPlugin::<PlayerBettingScreen>::default())
        .register_type::<PlayerBettingScreen>()
        .add_plugins(bits_ui_plugin_fn)
        .add_plugins(player_betting_plugin_fn)
        .add_systems(Startup, startup)
        .add_systems(Update, (sync_on_enter, log_outputs))
        .run();
}

