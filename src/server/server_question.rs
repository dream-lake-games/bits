use bevy::prelude::*;
use bevy_http_client::prelude::*;
use bits::prelude::*;
use serde::Deserialize;

#[derive(Deserialize, Debug, Clone, Default)]
struct LLMGeneratedQuestionResponse {}

fn start_generation_if_needed(
    question_generated_q: Query<Entity, Or<(With<QuestionGenerated>, With<QuestionGenerating>)>>,
    mut commands: Commands,
    mut ev_request: MessageWriter<TypedRequest<LLMGeneratedQuestionResponse>>,
) {
    if !question_generated_q.is_empty() {
        return;
    }

    if let Ok(request) = HttpClient::new()
        .post("https://httpbin.org/anything")
        .try_with_type::<LLMGeneratedQuestionResponse>()
    {
        commands.spawn((
            Name::new("Question"),
            Question::default(),
            QuestionGenerating,
        ));
        ev_request.write(request);
    }
}

fn handle_generation_response(
    mut events: ResMut<Messages<TypedResponse<LLMGeneratedQuestionResponse>>>,
    mut generating_q: Query<(Entity, &mut Question), With<QuestionGenerating>>,
    mut commands: Commands,
) {
    let Ok((generating_eid, mut question)) = generating_q.single_mut() else {
        return;
    };
    let Some(response) = events.drain().last() else {
        return;
    };
    debug!("Got new question response: {:?}", response);

    commands
        .entity(generating_eid)
        .remove::<QuestionGenerating>();
    commands.entity(generating_eid).insert(QuestionGenerated);
    question.question = String::from("yooo fake question");
    question.answer = 6;
}

fn handle_generation_failed(
    mut ev_error: MessageReader<TypedResponseError<LLMGeneratedQuestionResponse>>,
    generating_q: Query<(Entity,), With<QuestionGenerating>>,
    mut commands: Commands,
) {
    let Ok((generation_eid,)) = generating_q.single() else {
        return;
    };
    let Some(last_error) = ev_error.read().last() else {
        return;
    };
    error!("Error generating question: {:?}", last_error);
    commands.entity(generation_eid).despawn();
}

pub fn server_question_plugin_fn(app: &mut App) {
    app.register_request_type::<LLMGeneratedQuestionResponse>();

    app.add_systems(
        FixedUpdate,
        (
            start_generation_if_needed,
            handle_generation_response,
            handle_generation_failed,
        )
            .chain(),
    );
}
