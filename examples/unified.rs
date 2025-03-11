use kowalski::{Config, agent::UnifiedAgent, Agent};
use std::io::{self, Write};
use env_logger;
use log::info;
use serde_json;

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
        "Please search  internet: What are the latest Rust features? Could you search including latest news?",
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
                        info!("Tool call: {:?}", tool_call);
                        let function = tool_call.function;
                        info!("Function: {:?}", function);
                        let function_name = function.name;
                        let function_arguments = function.arguments;
                        info!("Function name: {:?}", &function_name);
                        info!("Function arguments: {:?}", &function_arguments);   
                        
                        if function_name == "search" {
                            let query = function_arguments.get("query").unwrap();
                            info!("Query: {:?}", query);
                            let cache = function_arguments.get("use_cache").unwrap_or(&serde_json::Value::Null); 
                            info!("Cache: {:?}", cache);
                            let max_results = function_arguments.get("max_results").unwrap_or(&serde_json::Value::Null);
                            info!("Max results: {:?}", max_results);
                            let include_images = function_arguments.get("include_images").unwrap_or(&serde_json::Value::Null);
                            info!("Include images: {:?}", include_images);
                            let include_videos = function_arguments.get("include_videos").unwrap_or(&serde_json::Value::Null);
                            info!("Include videos: {:?}", include_videos);
                            let include_news = function_arguments.get("include_news").unwrap_or(&serde_json::Value::Null);
                            info!("Include news: {:?}", include_news);
                            let include_maps = function_arguments.get("include_maps").unwrap_or(&serde_json::Value::Null);
                            info!("Include maps: {:?}", include_maps);
                            let include_shopping = function_arguments.get("include_shopping").unwrap_or(&serde_json::Value::Null);
                            info!("Include shopping: {:?}", include_shopping);
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