use kowalski::{Config, agent::UnifiedAgent, Agent};
use std::io::{self, Write};
use env_logger;
use log::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
        // Initialize logging, because printing to stdout is too mainstream
        env_logger::init();


    let config = Config::load()?;
    let mut agent = UnifiedAgent::new(config)?;
    // let conv_id = agent.start_conversation("michaelneale/deepseek-r1-goose");


    let conv_id = agent.start_conversation("llama3.2");




    // The model will automatically decide when to use tools
    let mut response = agent.chat_with_history(
        &conv_id,
        "Please search  internet: What are the latest Rust features? Could you search internet before responding?",
        None
    ).await?;

    // Process the response stream

    let mut buffer = String::new();

    dbg!(&response);

    // TODO: This is a hack to get the tool calls out of the response
    // TODO: We should use the tool calls in the response stream!
    // @FIXME: This is a hack to get the tool calls out of the response
    while let Some(chunk) = response.chunk().await? {
        dbg!(&chunk);

        match agent.process_stream_response(&conv_id, &chunk.to_vec()).await {
            Ok(Some(message)) => {
                if let Some(tool_calls) = message.tool_calls {
                    for tool_call in tool_calls {
                        println!("Tool call: {:?}", tool_call);
                        let function = tool_call.function;
                        println!("Function: {:?}", function);
                        let function_name = function.name;
                        let function_arguments = function.arguments;
                        println!("Function name: {:?}", &function_name);
                        println!("Function arguments: {:?}", &function_arguments);   
                        
                        if function_name == "search" {
                            let query = function_arguments.get("query").unwrap();
                            println!("Query: {:?}", query);
                        }

                    }
                }
                else {
                    print!("{}", message.content);
                    io::stdout().flush()?;
                    buffer.push_str(&message.content);
                }
            }
            Ok(None) => {
                println!("\n");
                break;
            }

            Err(e) => {
                eprintln!("\nError: {} (as if you didn't see that coming)", e);
                break;
            }
        }
    }

    Ok(())
}