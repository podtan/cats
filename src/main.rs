//! Main binary for CATS CLI

use clap::{Arg, Command};
use cats::{create_tool_registry, ToolArgs};
use std::collections::HashMap;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut app = Command::new("cats")
        .version("0.1.0")
        .about("Coding Agent ToolS - A comprehensive toolkit for building AI-powered coding agents")
        .subcommand_required(true)
        .arg_required_else_help(true);

    // Add predefined subcommands for known tools
    let tool_commands = vec![
        ("open", "Opens the file at the given path in the editor"),
        ("goto", "Moves the window to show specific line number"),
        ("scroll_up", "Moves the window up by the window size"),
        ("scroll_down", "Moves the window down by the window size"),
        ("create", "Creates and opens a new file with the given name"),
        (
            "find_file",
            "Finds all files with the given name or pattern",
        ),
        ("search_file", "Searches for text in a specific file"),
        ("search_dir", "Searches for text in all files in directory"),
        ("edit", "Edit files with search/replace"),
        ("insert", "Insert text into files"),
        (
            "_state",
            "Display current state of open files and tool context",
        ),
        ("filemap", "Generate project file structure map"),
        ("submit", "Submit completed task"),
    ];

    for (name, description) in &tool_commands {
        app = app.subcommand(
            Command::new(*name).about(*description).arg(
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
