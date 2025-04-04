mod api_request;
mod execute_command;
use api_request::*;
use clap::Parser;
use execute_command::*;

use anyhow::Context;
use scraper::{Html, Selector};
use std::{fs::File, io::Write};
use thiserror::Error;

const FOUR_SPACES: &'static str = "    ";

#[derive(Parser, Debug)]
#[command(name = "leetcode tests t7t rgleek")]
#[command(version = "1.0.0")]
#[command(version, about = "lololololy", long_about = None)]
// read from cargo file
// https://docs.rs/clap/latest/clap/_derive/_tutorial/chapter_1/index.html#:~:text=You%20can%20use,from%20%60Cargo.toml%60
struct Args {
    #[arg(long)]
    id: Option<u8>,
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let args = Args::parse();
    dbg!(&args);

    if let Some(id) = args.id {}
    // return Ok(());

    let leetcode_api_response = leetcode_reqwest()
        .await
        .context("Failed api request")?
        .data
        .active_daily_coding_challenge_question;

    let question_link = format!("https://leetcode.com{}", leetcode_api_response.link);
    let question_content = leetcode_api_response.question.content;

    let code_snippet = Lang::try_parse(&leetcode_api_response.question.code_snippets)?;

    let title_slug = leetcode_api_response.question.title_slug;
    let question_id = leetcode_api_response.question.question_id;
    let difficulty = leetcode_api_response.question.difficulty.to_lowercase();

    let dir_name = format!("{}_{}_{}", title_slug, question_id, difficulty);
    CommandStructure::new("cargo new --lib", &dir_name)
        .execute_command()
        .context(format!("Failed to create new cargo library `{}`", dir_name))?;

    let lib_file_path = format!("{}/src/lib.rs", dir_name);
    CommandStructure::new("echo '' >", &lib_file_path)
        .execute_command()
        .context(format!("Failed to clear contents of `{}`", lib_file_path))?;

    let file_content = generate_file_contents(&question_content, &question_link, code_snippet);

    write_to_lib_file(&file_content, &lib_file_path)?;

    let cargo_path = format!("{}/Cargo.toml", &dir_name);
    CommandStructure::new("cargo fmt --manifest-path", &cargo_path)
        .execute_command()
        .context(format!("Failed to format {}", &lib_file_path))?;

    Ok(())
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
        // edge case at this problem // https://leetcode.com/problems/find-the-power-of-k-size-subarrays-i/
        for ex in &examples_selector {
            dbg!(ex.text().collect::<String>());
        }

        let mut i = 0;
        while i < examples_selector.len() {
            let mut error_by_one = false;
            dbg!(i);
            let input = examples_selector[i].text().collect::<String>();
            // let output = examples_selector[i + 1].text().collect::<String>();
            let output = {
                if let Some(out) = examples_selector.get(i + 1) {
                    let output_text = out.text().collect::<String>();
                    if output_text.contains('=') {
                        // then the "output" didn't get scraped, and instead it scraped the next
                        // input
                        error_by_one = true;
                        "".to_string()
                    } else {
                        output_text.to_string()
                    }
                } else {
                    "".to_string()
                }
            };

            let input = comma_seperated_and_camel_case_to_snake_case(input);

            let formatted = format!(
                "let {};\n{FOUR_SPACES}{FOUR_SPACES}let output = {};",
                input, output
            );
            let formatted = add_vec_and_to_string(formatted);
            examples.push(formatted);
            i += 2;
            if error_by_one {
                i -= 1;
            }
        }
    }

    examples
}

// seperation of concerns
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

// TODO:
// 8 - [ ] support for more langs
// - [ ] cpp
// - [ ] python
// - [ ] javascript, idk someone would do it for some reason
// - won't add java, cuz it's java
//
// TODO:
// 9 - [] bug that changes the input to camelCase
// https://leetcode.com/problems/adding-spaces-to-a-string/
//
// TODO:
// 10 - [ ] bug on binary trees
// // https://leetcode.com/problems/reverse-odd-levels-of-binary-tree/
//
// TODO:
// 11 - [ ] bug, when output is empty list, compiler doesn't know the type and can't infer it
// // https://leetcode.com/problems/string-matching-in-an-array/description/
//
// TODO:
// 12 - [ ] bug, example input is one letter uppercase 'A', so the cli converts it to '_a'
// // https://leetcode.com/problems/find-the-prefix-common-array-of-two-arrays/description/
// TODO:
// 13 - [ ] bug, 2349. Design a Number Container System,
// https://leetcode.com/problems/design-a-number-container-system/
// [src/main.rs:28:5] &args = Args {
//     id: None,
// }
//     Creating library `design-a-number-container-system_2434_medium` package
// note: see more `Cargo.toml` keys and their definitions at https://doc.rust-lang.org/cargo/reference/manifest.html
// Command: cargo new --lib, with arg: design-a-number-container-system_2434_medium
//  Successfully executed
// Command: echo '' >, with arg: design-a-number-container-system_2434_medium/src/lib.rs
//  Successfully executed
// Successfully wrote to lib design-a-number-container-system_2434_medium/src/lib.rs
// error: expected identifier, found `}`
//   --> /home/abdo/duck/leet-him-code/design-a-number-container-system_2434_medium/src/lib.rs:45:32
//    |
// 45 |         let result = Solution::}
//    |                                ^ expected identifier
//
// Error: Failed to format design-a-number-container-system_2434_medium/src/lib.rs
//
// Caused by:
//     `Command: cargo fmt --manifest-path, with arg: design-a-number-container-system_2434_medium/Cargo.toml
//     ` failed to execute successfully
//
// 14 - [ ] count-days-without-meetings, the final test case doesn't have the expected output (which was the number zero)
