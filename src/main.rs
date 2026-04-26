//! Main binary for CATS CLI

use clap::{Arg, Command};
use cats::{create_tool_registry, ToolArgs};
use std::collections::HashMap;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Create registry to get available tools dynamically
    let registry = create_tool_registry();
    
    // Collect tool information (name, description) with static lifetime
    let tool_info: Vec<(&'static str, &'static str)> = registry
        .list_tools()
        .into_iter()
        .filter_map(|name| {
            registry.get_tool(&name).map(|tool| {
                let name_static: &'static str = Box::leak(name.into_boxed_str());
                let desc_static: &'static str = Box::leak(tool.description().to_string().into_boxed_str());
                (name_static, desc_static)
            })
        })
        .collect();
    
    let mut app = Command::new("cats")
        .version(env!("CARGO_PKG_VERSION"))
        .about("Coding Agent ToolS - A comprehensive toolkit for building AI-powered coding agents")
        .subcommand_required(true)
        .arg_required_else_help(true);

    // Dynamically add subcommands from collected tool info
    for (tool_name, description) in &tool_info {
        app = app.subcommand(
            Command::new(*tool_name)
                .about(*description)
                .arg(
                    Arg::new("args")
                        .help("Tool arguments")
                        .num_args(0..)
                        .value_name("ARGS"),
                ),
        );
    }

    let matches = app.get_matches();

    // Create registry for execution
    let mut registry = create_tool_registry();

    match matches.subcommand() {
        Some((tool_name, sub_matches)) => {
            let args: Vec<String> = sub_matches
                .get_many::<String>("args")
                .unwrap_or_default()
                .cloned()
                .collect();

            // Try to parse args as JSON if they look like JSON
            let tool_args = if args.len() == 1 && args[0].trim_start().starts_with('{') {
                // Try to parse as JSON and convert to named args
                match serde_json::from_str::<serde_json::Value>(&args[0]) {
                    Ok(json_value) => {
                        if let Some(obj) = json_value.as_object() {
                            let mut named_args = HashMap::new();
                            for (key, value) in obj {
                                if let Some(str_val) = value.as_str() {
                                    named_args.insert(key.clone(), str_val.to_string());
                                } else if let Some(bool_val) = value.as_bool() {
                                    named_args.insert(key.clone(), bool_val.to_string());
                                } else if let Some(num_val) = value.as_u64() {
                                    named_args.insert(key.clone(), num_val.to_string());
                                } else if let Some(num_val) = value.as_i64() {
                                    named_args.insert(key.clone(), num_val.to_string());
                                } else if let Some(num_val) = value.as_f64() {
                                    named_args.insert(key.clone(), num_val.to_string());
                                } else {
                                    named_args.insert(key.clone(), value.to_string());
                                }
                            }
                            ToolArgs::with_named_args(Vec::new(), named_args)
                        } else {
                            ToolArgs::with_named_args(args, HashMap::new())
                        }
                    }
                    Err(_) => ToolArgs::with_named_args(args, HashMap::new())
                }
            } else {
                ToolArgs::with_named_args(args, HashMap::new())
            };

            match registry.execute_tool(tool_name, &tool_args) {
                Ok(result) => {
                    println!("{}", result.message);
                    if !result.success {
                        std::process::exit(1);
                    }
                }
                Err(e) => {
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                }
            }
        }
        None => {
            eprintln!("No tool specified");
            std::process::exit(1);
        }
    }

    Ok(())
}
