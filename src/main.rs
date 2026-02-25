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
                    },
                    {
                        "type": "function",
                        "function": {
                            "name": "Write",
                            "description": "Write content to a file",
                            "parameters": {
                            "type": "object",
                            "required": ["file_path", "content"],
                            "properties": {
                                "file_path": {
                                "type": "string",
                                "description": "The path of the file to write to"
                                },
                                "content": {
                                "type": "string",
                                "description": "The content to write to the file"
                                }
                            }
                            }
                        }
                    },
                    {
                    "type": "function",
                    "function": {
                        "name": "Bash",
                        "description": "Execute a shell command",
                        "parameters": {
                        "type": "object",
                        "required": ["command"],
                        "properties": {
                            "command": {
                            "type": "string",
                            "description": "The command to execute"
                            }
                        }
                        }
                    }
                }
                ]
            }))
            .await?;

        // You can use print statements as follows for debugging, they'll be visible when running tests.
        eprintln!("Logs from your program will appear here!");

        let assistent_msg = response["choices"][0]["message"].clone();
        let content = assistent_msg["content"].as_str();
        messages.push(json!( {
            "role": String::from(assistent_msg["role"].as_str().unwrap()),
            "content":content,
            "tool_calls": assistent_msg["tool_calls"]
        }));
        if assistent_msg["tool_calls"].is_null()
            || assistent_msg["tool_calls"]
                .as_array()
                .map(|a| a.is_empty())
                .unwrap_or(true)
        {
            println!("{}", assistent_msg["content"].as_str().unwrap());
            break;
        } else {
            {
                for tool_call in assistent_msg["tool_calls"].as_array().unwrap() {
                    let tool_call_id = tool_call["id"].as_str().unwrap();

                    let name = assistent_msg["tool_calls"][0]["function"]["name"]
                        .as_str()
                        .unwrap();
                    let arguments = assistent_msg["tool_calls"][0]["function"]["arguments"]
                        .as_str()
                        .unwrap();
                    let args_val: serde_json::Value = serde_json::from_str(arguments).unwrap();
                    if name == String::from("Read") {
                        let f_path = args_val["file_path"].as_str().unwrap();
                        let f_content = std::fs::read_to_string(f_path).unwrap();
                        messages.push(json!( {
                            "role": String::from("tool"),
                            "content": f_content,
                            "tool_call_id": tool_call_id
                        }));
                    } else if name == String::from("Write") {
                        let f_path = args_val["file_path"].as_str().unwrap();
                        let f_write = args_val["content"].as_str().unwrap();

                        if let Some(parent) = std::path::Path::new(f_path).parent() {
                            if !parent.as_os_str().is_empty() {
                                std::fs::create_dir_all(parent)?;
                            }
                        }

                        std::fs::write(f_path, f_write)?;
                        messages.push(json!( {
                            "role": String::from("tool"),
                            "content": f_write,
                            "tool_call_id": tool_call_id
                        }));
                    } else if name == String::from("Bash") {
                        let comm_to_exe = args_val["command"].as_str().unwrap();
                        let output = std::process::Command::new(comm_to_exe)
                            .output()
                            .expect("Failed to execute command");
                        let msg: String;
                        if output.status.success() {
                            msg = String::from_utf8_lossy(&output.stdout).to_string();
                        } else {
                            msg = String::from_utf8_lossy(&output.stderr).to_string();
                        }
                        messages.push(json!( {
                            "role": String::from("tool"),
                            "content": msg,
                            "tool_call_id": tool_call_id
                        }));
                    }
                }
            }
        }
    }
    Ok(())
}
