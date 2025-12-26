use bevy::prelude::*;
use bevy_http_client::prelude::*;
use bits::prelude::*;
use serde::Deserialize;

fn server_question_last_invariants(questions_active_q: Query<Entity, With<QuestionActive>>) {
    let num_active_questions = questions_active_q.iter().count();
    assert!(num_active_questions <= 1);
}

#[derive(EntityEvent)]
pub struct GenerateQuestion {
    pub entity: Entity,
}

#[derive(Deserialize, Debug, Clone, Default)]
struct LLMGeneratedQuestionResponse {}

#[derive(Component)]
#[require(Name)]
struct PendingQuestionEntity;

fn generate_new_question(
    trigger: On<GenerateQuestion>,
    pending_q: Query<Entity, With<PendingQuestionEntity>>,
    mut ev_request: MessageWriter<TypedRequest<LLMGeneratedQuestionResponse>>,
    mut commands: Commands,
) {
    if !pending_q.is_empty() {
        info!("Skipping spawning new generate question task: task already exists");
        return;
    }

    if let Ok(request) = HttpClient::new()
        .post("https://httpbin.org/anything")
        .try_with_type::<LLMGeneratedQuestionResponse>()
    {
        let eid = trigger.entity;
        commands.entity(eid).insert(PendingQuestionEntity);
        ev_request.write(request);
    }
}

fn handle_question_response(
    mut events: ResMut<Messages<TypedResponse<LLMGeneratedQuestionResponse>>>,
    pending_q: Query<Entity, With<PendingQuestionEntity>>,
    old_active_q: Query<Entity, With<QuestionActive>>,
    mut commands: Commands,
) {
    for response in events.drain() {
        let _response: LLMGeneratedQuestionResponse = response.into_inner();
        info!("Received question response: {:?}", _response);

        for eid in &old_active_q {
            commands.entity(eid).remove::<QuestionActive>();
        }

        if let Ok(entity) = pending_q.single() {
            commands.entity(entity).remove::<PendingQuestionEntity>();
            commands.entity(entity).insert((
                Name::new("Question"),
                Question {
                    question: "This is a test question".to_string(),
                    answer: 6,
                },
                QuestionActive,
            ));
        }
    }
}

fn handle_question_error(
    mut ev_error: MessageReader<TypedResponseError<LLMGeneratedQuestionResponse>>,
) {
    for error in ev_error.read() {
        error!("Error generating question: {}", error.err);
    }
}

pub fn server_question_plugin_fn(app: &mut App) {
    app.register_request_type::<LLMGeneratedQuestionResponse>();

    app.add_observer(generate_new_question);

    app.add_systems(Last, server_question_last_invariants);

    app.add_systems(Update, (handle_question_response, handle_question_error));
}
