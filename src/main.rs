use async_openai::{Client, config::OpenAIConfig};
use clap::Parser;
use serde_json::{Value, json};
use std::{env, process};

#[derive(Parser)]
#[command(author, version, about)]
struct Args {
    #[arg(short = 'p', long)]
    prompt: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let base_url = env::var("OPENROUTER_BASE_URL")
        .unwrap_or_else(|_| "https://openrouter.ai/api/v1".to_string());

    let api_key = env::var("OPENROUTER_API_KEY").unwrap_or_else(|_| {
        eprintln!("OPENROUTER_API_KEY is not set");
        process::exit(1);
    });

    let config = OpenAIConfig::new()
        .with_api_base(base_url)
        .with_api_key(api_key);

    let client = Client::with_config(config);

    // struct Message {
    //     role: String,
    //     content: String,
    //     tool_call_id: String,
    // }
    let mut messages = Vec::new();
    let args = Args::parse();
    messages.push(json!( {
        "role": String::from("user"),
        "content": args.prompt.clone(),
    }));
    loop {
        #[allow(unused_variables)]
        let response: Value = client
            .chat()
            .create_byot(json!({
                "messages":messages,
                "model": "anthropic/claude-haiku-4.5",
                "tools":[
                    {
                        "type": "function",
                        "function": {
                            "name": "Read",
                            "description": "Read and return the contents of a file",
                            "parameters": {
                                "type": "object",
                                "properties": {
                                "file_path": {
                                    "type": "string",
                                    "description": "The path to the file to read"
                                    }
                                },
                                "required": ["file_path"]
                            }
                        }
                    }
                ]
            }))
            .await?;

        // You can use print statements as follows for debugging, they'll be visible when running tests.
        eprintln!("Logs from your program will appear here!");

        let content = response["choices"][0]["message"]["content"]
            .as_str()
            .unwrap();
        messages.push(json!( {
            "role": String::from(response["choices"][0]["message"]["role"].as_str().unwrap()),
            "content":content,

        }));
        // TODO: Uncomment the lines below to pass the first stage
        if !response["choices"][0]["message"]["tool_calls"].is_null() {
            let name = response["choices"][0]["message"]["tool_calls"][0]["function"]["name"]
                .as_str()
                .unwrap();
            let arguments =
                response["choices"][0]["message"]["tool_calls"][0]["function"]["arguments"]
                    .as_str()
                    .unwrap();
            let args_val: serde_json::Value = serde_json::from_str(arguments).unwrap();
            let f_path = args_val["file_path"].as_str().unwrap();
            let f_content = std::fs::read_to_string(f_path).unwrap();
            if name == String::from("Read") {
                messages.push(json!( {
                    "role": String::from("tool"),
                    "content": f_content,
                    "tool_use_id": String::from(
                        response["choices"][0]["message"]["tool_calls"][0]["id"]
                            .as_str()
                            .unwrap(),
                    ),
                }));
            }
        } else {
            println!("{}", content);
            break;
        }
    }
    Ok(())
}
