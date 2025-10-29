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
        .version("0.1.1")
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

            let tool_args = ToolArgs::with_named_args(args, HashMap::new());

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
