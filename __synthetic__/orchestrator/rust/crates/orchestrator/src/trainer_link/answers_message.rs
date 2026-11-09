use crate::trainer_link::answer::Answer;

use serde::Deserialize;

/// 18: the stdin line.
#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct AnswersMessage {
    pub answers: Vec<Answer>,
}
