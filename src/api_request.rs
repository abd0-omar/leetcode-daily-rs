use serde::Deserialize;
use serde_json::json;

const LEETCODE_API: &'static str = "https://leetcode.com/graphql/";

#[derive(Deserialize)]
struct QuestionData {
    #[allow(dead_code)]
    active_daily_coding_challenge_question: ActiveDailyCodingChallengeQuestion,
}

#[derive(Deserialize, Debug)]
pub struct GraphQlLeetcodeResponse {
    pub data: Data,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Data {
    pub active_daily_coding_challenge_question: ActiveDailyCodingChallengeQuestion,
}

#[derive(Deserialize, Debug)]
pub struct ActiveDailyCodingChallengeQuestion {
    pub link: String,
    pub question: Question,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Question {
    pub title_slug: String,
    pub content: String,
    pub difficulty: String,
    pub code_snippets: Vec<Lang>,
    pub question_id: String,
}

#[derive(Deserialize, Debug)]
pub struct Lang {
    pub lang: String,
    pub code: String,
}

// if snippet.lang != "Rust" {
//     // search for rust
//     for lang in code_snippets {
//         if lang.lang == "Rust" {
//             dbg!(lang);
//             return Ok(lang);
//         }
//     }
//     return Err(anyhow::Error::msg("no Rust!, 7asal 5er"));
// } else {
//     Ok(snippet)
// }

impl Lang {
    pub fn try_parse(code_snippets: &[Lang]) -> Result<&Lang, anyhow::Error> {
        // Rust is the 15 indexed in the code_snippets vec
        // check if it is rust
        if let Some(snippet) = code_snippets.get(15) {
            if snippet.lang != "Rust" {
                // search for rust
                for lang in code_snippets {
                    if lang.lang == "Rust" {
                        dbg!(lang);
                        return Ok(lang);
                    }
                }
                return Err(anyhow::Error::msg("no Rust!, 7asal 5er"));
            }
            Ok(snippet)
        } else {
            Err(anyhow::Error::msg("index out of bounds"))
        }
    }
}

#[derive(thiserror::Error, Debug)]
pub enum ReqwestApiError {
    #[error("Failed to decode the JSON response")]
    DecodeError(#[from] reqwest::Error),

    #[error("Reqwest failed to send a request, for some reason")]
    UnexpectedError(#[source] reqwest::Error),
}

pub async fn leetcode_reqwest() -> Result<GraphQlLeetcodeResponse, ReqwestApiError> {
    let query = r#" query questionOfToday {
        activeDailyCodingChallengeQuestion {
            link
            question {
                difficulty
                titleSlug
                content
                questionId
                codeSnippets {
                    lang
                    code
                }
            }
        }
    }
    "#;

    let payload = json!(
        {
            "query" : query,
            "variables" :{},
            "operationName" : "questionOfToday"
        }
    );

    Ok(reqwest::Client::new()
        .post(LEETCODE_API)
        .json(&payload)
        .send()
        .await
        .map_err(ReqwestApiError::UnexpectedError)?
        .json::<GraphQlLeetcodeResponse>()
        .await
        .map_err(ReqwestApiError::DecodeError)?)
}

#[derive(Deserialize, Debug)]
struct GraphQlIDLeetcode {
    pub data: Data,
}

#[derive(Deserialize, Debug)]
struct selectProblem {
    pub question: Question,
}

pub async fn leetcode_reqwest_with_id(id: u8) -> Result<GraphQlLeetcodeResponse, ReqwestApiError> {
    let query = r#" query selectProblem($questionId: id!) {
        question(questionId: $questionId) {
            titleSlug
        }
    }
    "#;

    let payload = json!(
        {
            "query" : query,
            "variables" :{
                "id": id
            },
            "operationName" : "selectProblem"
        }
    );

    Ok(reqwest::Client::new()
        .post(LEETCODE_API)
        .json(&payload)
        .send()
        .await
        .map_err(ReqwestApiError::UnexpectedError)?
        .json::<GraphQlLeetcodeResponse>()
        .await
        .map_err(ReqwestApiError::DecodeError)?)
}
