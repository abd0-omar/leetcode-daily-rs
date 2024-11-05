use anyhow::Context;
use scraper::{Html, Selector};
use std::{fs::File, io::Write, process::Command};
use thiserror::Error;

use serde::Deserialize;
#[derive(Deserialize, Debug)]
struct LeetcodeApi {
    data: Data,
}

#[derive(Deserialize, Debug)]
struct Data {
    #[serde(rename(deserialize = "activeDailyCodingChallengeQuestion"))]
    active_daily_coding_challenge_question: DailyQuestion,
}

#[derive(Deserialize, Debug)]
struct DailyQuestion {
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
    #[error("Too many requests from this IP, try again in 1 hour")]
    DecodeError(#[from] reqwest::Error),

    #[error("Reqwest failed to send a request, for some reason")]
    UnexpectedError(#[source] reqwest::Error),
}

fn leetcode_reqwest() -> Result<DailyQuestion, ReqwestApiError> {
    let response = reqwest::blocking::get("https://alfa-leetcode-api.onrender.com/dailyQuestion")
        .map_err(ReqwestApiError::UnexpectedError)?;

    let decoding_json = response
        .json::<LeetcodeApi>()
        .map_err(ReqwestApiError::DecodeError)?;

    Ok(decoding_json.data.active_daily_coding_challenge_question)
}

fn main() -> Result<(), anyhow::Error> {
    let leetcode_api_response = leetcode_reqwest().context("Failed api request")?;
    let question_link = format!("https://leetcode.com{}", leetcode_api_response.link);

    let question_response = leetcode_api_response.question;
    let question_id = question_response.question_id;
    let title_slug = question_response.title_slug;
    let difficulty = question_response.difficulty.to_lowercase();
    let quesion_content = question_response.content;
    let code_snippet = &question_response.code_snippets[15];
    // check if it is rust
    let lang = &code_snippet.lang;
    if lang != "Rust" {
        println!("not Rust!, 7asal 5er");
        return Ok(());
    }

    let document = Html::parse_document(&quesion_content);

    // sometimes it's <pre> sometimes it's <example-io> classes
    let example_selector = Selector::parse(".example-io").unwrap();

    let mut examples: Vec<String> = Vec::new();

    let examples_selector: Vec<_> = document.select(&example_selector).collect();
    if !examples_selector.is_empty() {
        let mut i = 0;
        while i < examples_selector.len() {
            let input = examples_selector[i].text().collect::<String>();
            let output = examples_selector[i + 1].text().collect::<String>();
            let formatted = format!("let {};\n\t\tlet output = {};", input, output);
            // add vecs
            let formatted = formatted.replace("[", "vec![");
            // add .to_string()
            let mut new_formatted = String::new();
            let mut skip = true;
            for letter in formatted.chars() {
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
            examples.push(new_formatted);
            dbg!(input);
            dbg!(output);
            i += 2;
        }
    } else {
        let example_selector = Selector::parse("pre").unwrap();
        // let examples_selector: Vec<_> = document.select(&example_selector).collect();
        for wow in document.select(&example_selector) {
            let text: String = wow.text().collect();
            let mut parts = text.split('\n');
            let input = parts.next().unwrap();
            let output = parts.next().unwrap();

            // remove Input:
            let input = input.replace("Input:", "");
            // remove Output:
            let output = output.replace("Output:", "");

            let formatted = format!("let{};\n\t\tlet output ={};", input, output);
            // add vecs
            let formatted = formatted.replace("[", "vec![");
            // add .to_string()
            let mut new_formatted = String::new();
            let mut skip = true;
            for letter in formatted.chars() {
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
            examples.push(new_formatted);
            dbg!(input);
            dbg!(output);
        }
    }

    let file_path = format!("{}_{}_{}", title_slug, question_id, difficulty);

    let create_cargo_lib = Command::new("sh")
        .arg("-c")
        .arg(format!("cargo new --lib {}", file_path))
        .status()
        .unwrap();

    if create_cargo_lib.success() {
        println!("Successfully created {}", file_path);
    } else {
        eprintln!("Failed to create {}", file_path);
        return Ok(());
    }

    let lib_file_path = format!("{}/src/lib.rs", file_path);

    let delete_lib_content = Command::new("sh")
        .arg("-c")
        .arg(format!("echo '' > {}", lib_file_path))
        .status()
        .unwrap();

    if delete_lib_content.success() {
        println!("Successfully cleared {}", lib_file_path);
    } else {
        eprintln!("Failed to clear {}", lib_file_path);
        return Ok(());
    }

    // File::create() docs
    // `This function will create a file if it does not exist,
    // and will truncate it if it does.`
    // "truncate" means clear its contents and start fresh if it does exist
    let mut lib_file = File::create(&lib_file_path).expect("Failed to create lib.rs");

    let function_signature = &code_snippet.code;

    let function_signature =
        function_signature.replace("{\n        \n    }\n}", "{\n\t\ttodo!();\n\t}\n}");

    let first_bracket_in_func_signature = function_signature.find('(').unwrap();
    let pure_function_name = &function_signature[27..first_bracket_in_func_signature];
    let last_bracket_in_func_signature = function_signature.find(')').unwrap();

    let slice =
        &function_signature[first_bracket_in_func_signature + 1..last_bracket_in_func_signature];

    let parts = slice.split_whitespace().collect::<Vec<_>>();

    let mut function_name = String::new();

    let mut i = 0;
    while i < parts.len() {
        if i == parts.len() - 2 {
            function_name.push_str(&parts[i].replace(":", ""));
        } else {
            function_name.push_str(&parts[i].replace(":", ", "));
        }
        i += 2;
    }

    dbg!(&function_name);

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
        let result = Solution::{pure_function_name}({function_name});
    }}"#
        );
        test_cases.push_str(&test_case);
    }

    let file_content = format!(
        r#"// {question_link}
struct Solution;

{function_signature}

#[cfg(test)]
mod tests {{
    use super::*;{test_cases}
}}
"#
    );

    lib_file
        .write_all(file_content.as_bytes())
        .expect("Failed to write to lib.rs");

    println!("Successfully wrote to lib {}", lib_file_path);
    // TODO: adjust test cases to use the @ c
    // from this
    // //Example 1:
    // Input: s = "abcde", goal = "cdeab"
    // Output: true
    //
    // //Example 2:
    // Input: s = "abcde", goal = "abced"
    // Output: false
    //
    // to this
    // //Example 1:
    // let s = "abcde", goal = "cdeab";
    // let output =  true;
    //
    // //Example 2:
    // let s = "abcde", goal = "abced";
    // let output =  false;
    //
    // then adjust it more with doing the strings and vecs
    // //Example 1:
    // let Input: s = "abcde".to_string(), goal = "cdeab".to_string();
    // let output =  true;
    //
    // //Example 2:
    // let s = "abcde", goal = "abced";
    // let output =  false;
    //
    // then lastly try  to remove the commas, but idk how would be done for now
    //
    // rust fmt at the end of the program, if all not formatted
    //
    // remove "Example 1, 2, 3" doesn't add anything
    //
    // TODO: for now
    // 1 - [ ] Input output
    // - [x] remove input and output and add ; at the end
    // - [x] adjust the vecs by replacing '[' with 'vec!['
    // - [x] adjust the strings by replacing to every second '"' with '".to_string()'
    // - [ ] replace commas that are not in the input or in the vecs with '; let '
    //
    // TODO:
    // 2 - [x] Solution::func_name(params)
    // - [x] Solution::()
    // - [x] parse the function signature to split the between the brackets "()" by ','
    // - [x] get the index of '(' and the end of ')' and split by ',' for what's between them
    //
    // TODO:
    // 3 - simple cleaning
    // - [ ] replace all "\t" with spaces
    // - [ ] add 2 allow deadcode above struct Solution and impl Solution
    //
    // TODO:
    // 4 - Error handling
    // - [x] reqwest
    // - [ ] the rest
    Ok(())
}
