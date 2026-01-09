use bevy::prelude::*;
use bevy::tasks::Task;
use bevy::tasks::futures_lite::future;
use bits::prelude::*;
use lightyear::prelude::*;

#[derive(Resource, Default)]
struct QuestionGenTask {
    task: Option<Task<anyhow::Result<GeneratedQuestion>>>,
}

fn start_generation_if_needed(
    question_q: Query<Entity, Or<(With<QuestionGenerated>, With<QuestionGenerating>)>>,
    ai_clients: Option<Res<AIClients>>,
    mut task_res: ResMut<QuestionGenTask>,
    mut commands: Commands,
) {
    if !question_q.is_empty() {
        return;
    }

    if task_res.task.is_some() {
        return;
    }

    let Some(ai_clients) = ai_clients else {
        return;
    };

    let Some(exa) = ai_clients.exa.clone() else {
        warn!("No EXA_API_KEY set, cannot generate questions");
        return;
    };

    let openai = ai_clients.openai.clone();

    debug!("Starting question generation task");

    commands.spawn((
        Name::new("Question"),
        Replicate::to_clients(NetworkTarget::All),
        Question::default(),
        QuestionGenerating,
    ));

    let task = bevy::tasks::IoTaskPool::get().spawn(async move {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("Failed to create Tokio runtime");
        rt.block_on(generate_question(
            &openai,
            &exa,
            "general trivia and interesting facts",
        ))
    });

    task_res.task = Some(task);
}

fn poll_generation_task(
    mut task_res: ResMut<QuestionGenTask>,
    mut generating_q: Query<(Entity, &mut Question), With<QuestionGenerating>>,
    mut commands: Commands,
) {
    let Some(task) = task_res.task.as_mut() else {
        return;
    };

    let Some(result) = future::block_on(future::poll_once(task)) else {
        return;
    };

    task_res.task = None;

    let Ok((generating_eid, mut question)) = generating_q.single_mut() else {
        return;
    };

    match result {
        Ok(generated) => {
            info!(
                "Generated question: {} (answer: {} {:?})",
                generated.question, generated.answer, generated.units
            );
            question.question = generated.question;
            question.answer = generated.answer;
            question.units = generated.units;
            question.sources = generated.sources;

            commands
                .entity(generating_eid)
                .remove::<QuestionGenerating>();
            commands.entity(generating_eid).insert(QuestionGenerated);
        }
        Err(e) => {
            error!("Failed to generate question: {}", e);
            commands.entity(generating_eid).despawn();
        }
    }
}

pub fn server_question_plugin_fn(app: &mut App) {
    app.init_resource::<QuestionGenTask>();

    app.add_systems(
        FixedUpdate,
        (start_generation_if_needed, poll_generation_task).chain(),
    );
}
