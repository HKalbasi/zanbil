use std::{iter::Iterator, process::Stdio};

use clap::Parser;
use serde::Deserialize;
use toml_edit::DocumentMut;

#[derive(Debug, Parser)]
enum Command {
    Init {
        name: String,
        #[arg(long)]
        lib: bool,
    },
}

#[derive(Debug, Parser)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,

    /// Arguments to forward to cargo, if no builtin command is selected
    #[arg(trailing_var_arg = true)]
    forward_args: Vec<String>,
}

#[derive(Debug, Default, Deserialize)]
struct Config {
    zanbil_build_local_path: Option<String>,
}

impl Config {
    fn read() -> Self {
        let Some(dirs) = directories::ProjectDirs::from("zanbil", "zanbil", "zanbil") else {
            return Self::default();
        };
        let Ok(file) = std::fs::read_to_string(dirs.config_dir().join("config.toml")) else {
            return Self::default();
        };
        toml::from_str(&file).unwrap()
    }
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let config = Config::read();
    let Some(command) = cli.command else {
        // Forward to cargo
        let output = spawn_cargo(&cli.forward_args);

        std::process::exit(output.status.code().unwrap_or(1));
    };
    match command {
        Command::Init { name, lib } => {
            if lib {
                spawn_cargo(["init", &name, "--lib"]);
            } else {
                spawn_cargo(["init", &name]);
            }
            std::env::set_current_dir(&name)?;
            let mut toml = std::fs::read_to_string("Cargo.toml")?.parse::<DocumentMut>()?;
            toml["package"]["links"] = toml_edit::value(&name);
            std::fs::write("Cargo.toml", toml.to_string())?;
            if lib {
                std::fs::write("src/lib.rs", include_str!("../templates/lib.rs"))?;
                std::fs::write("src/lib.c", include_str!("../templates/lib.c"))?;
                std::fs::write("src/lib.h", include_str!("../templates/lib.h"))?;
            } else {
                std::fs::write("src/main.rs", include_str!("../templates/main.rs"))?;
                std::fs::write("src/main.c", include_str!("../templates/main.c"))?;
            }
            std::fs::write("build.rs", include_str!("../templates/build.rs"))?;
            let cargo_add_args = if let Some(p) = config.zanbil_build_local_path {
                &["add", "--build", "--path", p.leak()] as &[&str]
            } else {
                &[
                    "add",
                    "--build",
                    "--git",
                    "https://github.com/HKalbasi/zanbil",
                    "zanbil-build",
                ]
            };
            spawn_cargo(cargo_add_args);
        }
    }
    Ok(())
}

fn spawn_cargo<I: IntoIterator>(args: I) -> std::process::Output
where
    <I as IntoIterator>::Item: AsRef<std::ffi::OsStr>,
{
    let mut cargo_cmd = std::process::Command::new("cargo");

    // Add all forwarded arguments to cargo
    cargo_cmd.args(args);

    // Execute cargo with forwarded arguments
    let output = cargo_cmd
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .expect("Failed to spawn cargo")
        .wait_with_output()
        .expect("Failed to wait for cargo");
    output
}
