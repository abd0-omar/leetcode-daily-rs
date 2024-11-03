use std::{fs::File, io::Write, process::Command};

use scraper::{Html, Selector};
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

fn main() {
    let leetcode_api_response =
        reqwest::blocking::get("https://alfa-leetcode-api.onrender.com/dailyQuestion")
            .unwrap()
            .json::<LeetcodeApi>()
            .unwrap()
            .data
            .active_daily_coding_challenge_question;

    // let link = leetcode_api_response
    //     .data
    //     .active_daily_coding_challenge_question
    //     .link;
    // dbg!(&link);
    // we have the link
    // cargo would generate
    // 1 - file name would be title_slug, with maybe the difficulty
    // 2 - comment at the top of the file with the link
    // 3 - the rust function
    // omit the one below for now
    // 4 - testcases
    // let mut args = args().into_iter().skip(1);
    let question_link = format!("https://leetcode.com{}\n", leetcode_api_response.link);

    let question_response = leetcode_api_response.question;
    let question_id = question_response.question_id;
    let title_slug = question_response.title_slug;
    let difficulty = question_response.difficulty;
    let quesion_content = question_response.content;
    let code_snippet = &question_response.code_snippets[15];
    // check if it is rust
    let lang = &code_snippet.lang;
    if lang != "Rust" {
        println!("not Rust!, 7asal 5er");
        return;
    }

    let document = Html::parse_document(&quesion_content);
    // Selector for "Input" "Output" blocks
    let example_selector = Selector::parse("pre").unwrap();

    let mut examples = Vec::new();

    for (idx, example) in document.select(&example_selector).enumerate() {
        let input_output = example.text().collect::<String>();
        // TODO:
        // `input_output` needs better indentation, as intended, pun indented
        let mut new_input_output = String::new();
        for line in input_output.lines() {
            new_input_output.push_str("\t\t\t\t");
            new_input_output.push_str(line);
            new_input_output.push('\n');
        }
        let input_output = new_input_output.trim_end();
        let example = format!("// Example {}:\n{}", idx + 1, input_output);
        examples.push(example);
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
        return;
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
        return;
    }

    // File::create() docs
    // `This function will create a file if it does not exist,
    // and will truncate it if it does.`
    // "truncate" means clear its contents and start fresh if it does exist
    let mut lib_file = File::create(&lib_file_path).expect("Failed to create lib.rs");

    let function_signature = &code_snippet.code;

    let function_signature =
        function_signature.replace("{\n        \n    }\n}", "{\n\t\t\t\ttodo!();\n\t\t}\n}");

    let mut test_cases = String::new();

    for (idx, example) in examples.into_iter().enumerate() {
        /*
                 let result = Solution::{function_signature};
                let output = // from the test cases;
                assert_eq!(result, output);

        */

        // let example = example.trim_end();
        let test_case = format!(
            r#"

    #[test]
    fn it_works{idx}() {{
        {example}
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
}}"#
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
}
