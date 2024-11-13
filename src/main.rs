use anyhow::Context;
use scraper::{Html, Selector};
use serde::Deserialize;
use serde_json::json;
use std::{fs::File, io::Write, process::Command};
use thiserror::Error;

const FOUR_SPACES: &'static str = "    ";
const LEETCODE_API: &'static str = "https://leetcode.com/graphql/";

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let leetcode_api_response = leetcode_reqwest()
        .await
        .context("Failed api request")?
        .data
        .active_daily_coding_challenge;

    let question_link = format!("https://leetcode.com{}", leetcode_api_response.link);
    let question_content = leetcode_api_response.question.content;

    // Rust is the 15 indexed in the code_snippets vec
    // check if it is rust
    let code_snippet = &leetcode_api_response.question.code_snippets[15];
    if code_snippet.lang != "Rust" {
        return Err(anyhow::Error::msg("not Rust!, 7asal 5er"));
    }

    let title_slug = leetcode_api_response.question.title_slug;
    let question_id = leetcode_api_response.question.question_id;
    let difficulty = leetcode_api_response.question.difficulty.to_lowercase();

    let dir_name = format!("{}_{}_{}", title_slug, question_id, difficulty);
    execute_command("cargo new --lib", &dir_name)
        .context(format!("Failed to create new cargo library `{}`", dir_name))?;

    let lib_file_path = format!("{}/src/lib.rs", dir_name);
    execute_command("echo '' >", &lib_file_path)
        .context(format!("Failed to clear contents of `{}`", lib_file_path))?;

    let file_content = generate_file_contents(&question_content, &question_link, code_snippet);

    write_to_lib_file(&file_content, &lib_file_path)?;

    execute_command(
        "cargo fmt --manifest-path",
        &format!("{}/Cargo.toml", &dir_name),
    )
    .context(format!("Failed to format {}", &lib_file_path))?;

    Ok(())
}

#[derive(Deserialize)]
struct QuestionData {
    #[serde(rename(deserialize = "ActiveDailyCodingChallengeQuestion"))]
    #[allow(dead_code)]
    active_daily_coding_challenge_question: ActiveDailyCodingChallengeQuestion,
}

#[derive(Deserialize, Debug)]
struct GraphQlLeetcodeResponse {
    data: Data,
}

#[derive(Deserialize, Debug)]
struct Data {
    #[serde(rename(deserialize = "activeDailyCodingChallengeQuestion"))]
    active_daily_coding_challenge: ActiveDailyCodingChallengeQuestion,
}

#[derive(Deserialize, Debug)]
struct ActiveDailyCodingChallengeQuestion {
    link: String,
    question: Question,
}

#[derive(Deserialize, Debug)]
struct Question {
    #[serde(rename(deserialize = "titleSlug"))]
    title_slug: String,
    content: String,
    difficulty: String,
    #[serde(rename(deserialize = "codeSnippets"))]
    code_snippets: Vec<Lang>,
    #[serde(rename(deserialize = "questionId"))]
    question_id: String,
}

#[derive(Deserialize, Debug)]
struct Lang {
    lang: String,
    code: String,
}

#[derive(Error, Debug)]
pub enum ReqwestApiError {
    #[error("Failed to decode the JSON response")]
    DecodeError(#[from] reqwest::Error),

    #[error("Reqwest failed to send a request, for some reason")]
    UnexpectedError(#[source] reqwest::Error),
}

async fn leetcode_reqwest() -> Result<GraphQlLeetcodeResponse, ReqwestApiError> {
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

#[derive(Error, Debug)]
enum CommandError {
    #[error("Failed to execute the `{0}` process")]
    ExecuteProcessError(#[from] std::io::Error),
    #[error("Command `{0}` failed to execute successfully")]
    CommandExecutionError(String),
}

fn execute_command(command: &str, file_path: &str) -> Result<(), CommandError> {
    let status = Command::new("sh")
        .arg("-c")
        .arg(format!("{} {}", command, file_path))
        .status()
        .map_err(CommandError::ExecuteProcessError)?;

    if status.success() {
        println!("Command `{} {}` Successfully executed", command, file_path);
        Ok(())
    } else {
        Err(CommandError::CommandExecutionError(format!(
            "`{} {}`",
            command, file_path
        )))
    }
}

fn generate_file_contents(
    question_content: &str,
    question_link: &str,
    code_snippet: &Lang,
) -> String {
    let examples = extract_examples(question_content);

    let (test_cases, function_signature) = generate_test_cases(code_snippet, examples);

    let file_content = format!(
        r#"// {question_link}
#[allow(dead_code)]
struct Solution;

#[allow(dead_code)]
{function_signature}

#[cfg(test)]
mod tests {{
    use super::*;{test_cases}
}}
"#
    );

    file_content
}

fn extract_examples(question_content: &str) -> Vec<String> {
    let document = Html::parse_document(&question_content);

    let pre_selector = Selector::parse("pre").unwrap();

    let mut examples: Vec<String> = Vec::new();

    if document.select(&pre_selector).next().is_some() {
        for input_output in document.select(&pre_selector) {
            let input_output: String = input_output.text().collect();
            let mut input_output_parts = input_output.split('\n');
            let input = input_output_parts
                .next()
                .unwrap_or("")
                .replace("Input:", "");
            let output = input_output_parts
                .next()
                .unwrap_or("")
                .replace("Output:", "");

            let input = comma_seperated_and_camel_case_to_snake_case(input);

            let formatted = format!(
                "let{};\n{FOUR_SPACES}{FOUR_SPACES}let output ={};",
                input, output
            );
            let formatted = add_vec_and_to_string(formatted);
            examples.push(formatted);
        }
    } else {
        let example_io_selector = Selector::parse(".example-io").unwrap();
        let examples_selector: Vec<_> = document.select(&example_io_selector).collect();

        let mut i = 0;
        while i < examples_selector.len() {
            let input = examples_selector[i].text().collect::<String>();
            let output = examples_selector[i + 1].text().collect::<String>();

            let input = comma_seperated_and_camel_case_to_snake_case(input);

            let formatted = format!(
                "let {};\n{FOUR_SPACES}{FOUR_SPACES}let output = {};",
                input, output
            );
            let formatted = add_vec_and_to_string(formatted);
            examples.push(formatted);
            i += 2;
        }
    }

    examples
}

fn comma_seperated_and_camel_case_to_snake_case(input: String) -> String {
    // comma separated inputs
    let input = input.replace(", ", "; let ");

    // convert camelCase to snake_case
    let mut result = String::new();
    for letter in input.chars() {
        if letter.is_uppercase() {
            result.push_str(&format!("_{}", letter.to_lowercase()));
            continue;
        }
        result.push(letter);
    }
    result
}

fn add_vec_and_to_string(input: String) -> String {
    let mut new_formatted = String::new();
    let mut skip = true;
    for letter in input.replace("[", "vec![").chars() {
        if letter == '"' {
            if skip {
                skip = false;
            } else {
                new_formatted.push_str("\".to_string()");
                skip = true;
                continue;
            }
        }
        new_formatted.push(letter);
    }
    new_formatted
}

fn generate_test_cases(code_snippet: &Lang, examples: Vec<String>) -> (String, String) {
    let function_signature_and_body = code_snippet.code.replace(
        &format!("\n{}{}\n", FOUR_SPACES, FOUR_SPACES),
        &format!("\n{}{}todo!();\n", FOUR_SPACES, FOUR_SPACES),
    );

    let first_bracket_in_func_signature = function_signature_and_body
        .find('(')
        .expect("for some reason there is no `(`");
    let pure_function_name = &function_signature_and_body[27..first_bracket_in_func_signature];
    let last_bracket_in_func_signature = function_signature_and_body
        .find(')')
        .expect("for some reason there is no `)`");

    let function_params_raw = &function_signature_and_body
        [first_bracket_in_func_signature + 1..last_bracket_in_func_signature];

    let function_params_parts_unformatted =
        function_params_raw.split_whitespace().collect::<Vec<_>>();

    let mut function_params_comma_formatted = String::new();

    let mut i = 0;
    while i < function_params_parts_unformatted.len() {
        if i == function_params_parts_unformatted.len() - 2 {
            function_params_comma_formatted
                .push_str(&function_params_parts_unformatted[i].replace(":", ""));
        } else {
            function_params_comma_formatted
                .push_str(&function_params_parts_unformatted[i].replace(":", ", "));
        }
        i += 2;
    }

    let mut test_cases = String::new();

    for (idx, example) in examples.into_iter().enumerate() {
        /*
                let output = // from the test cases;
                let result = Solution::{function_signature};
                result then output
                RadiOhead
                assert_eq!(result, output);
        */
        let test_case = format!(
            r#"

    #[test]
    fn it_works{idx}() {{
        {example}
        let result = Solution::{pure_function_name}({function_params_comma_formatted});
        assert_eq!(result, output);
    }}"#
        );
        test_cases.push_str(&test_case);
    }

    (test_cases, function_signature_and_body)
}

#[derive(Error, Debug)]
enum CreateWriteLibFileError {
    #[error("Failed to create aka clear lib.rs")]
    CreateClearLibFile(std::io::Error),
    #[error("Failed to write to lib.rs")]
    WriteToLibFile(std::io::Error),
}

fn write_to_lib_file(
    file_content: &str,
    lib_file_path: &str,
) -> Result<(), CreateWriteLibFileError> {
    // File::create() docs
    // This function will create a file if it does not exist,
    // and will truncate it if it does.
    // "truncate" means clear its contents and start fresh if it does exist
    let mut lib_file =
        File::create(lib_file_path).map_err(CreateWriteLibFileError::CreateClearLibFile)?;

    lib_file
        .write_all(file_content.as_bytes())
        .map_err(CreateWriteLibFileError::WriteToLibFile)?;

    println!("Successfully wrote to lib {}", lib_file_path);
    Ok(())
}

// TODO:
// 1 - [x] Input output
// - [x] remove input and output and add ; at the end
// - [x] adjust the vecs by replacing '[' with 'vec!['
// - [x] adjust the strings by replacing to every second '"' with '".to_string()'
// - [x] replace commas that are not in the input or in the vecs with '; let '
// - [x] if input is camelCase then make it snake_case
//
// TODO:
// 2 - [x] Solution::func_name(params)
// - [x] Solution::()
// - [x] parse the function signature to split the between the brackets "()" by ','
// - [x] get the index of '(' and the end of ')' and split by ',' for what's between them
//
// TODO:
// 3 - [ ] simple cleaning
// - [x] replace all "\t" with spaces
// - [x] add 2 allow deadcode above struct Solution and impl Solution
// - [ ] I guess I could benefit from more cleanup
// - [x] add cargo format instead of manual indenting and adding spaces
//
// TODO:
// 4 - [x] Error handling
// - [x] reqwest
// - [x] the rest
//
// TODO:
// 5 - [ ] make structs to be more organized

// TODO:
// 6 - [x] add graphql straight from leetcode instead of the random rest api

// TODO:
// 7 - [ ] add clap cli
// - [ ] default option generate leecode daily
// - [ ] option to generate leetcode proplem with id
