mod skybox_processor;
mod texture_compressor;

use std::str::FromStr;

use ikari::block_on;
use ikari::file_manager::GamePathMaker;
use skybox_processor::SkyboxProcessorArgs;
use texture_compressor::TextureCompressorArgs;

lazy_static::lazy_static! {
    pub static ref PATH_MAKER: GamePathMaker = GamePathMaker::new(None);
}

const HELP: &str = "\
ikari cli

Usage: clikari --command CMD [OPTIONS]

Options:
  --command CMD  Required  The command to run. Possible values include:
                             compress_textures
                             process_skybox
  --help         Optional  Display this help message
";

const TEXTURE_COMPRESSOR_HELP: &str = "\
Compress all textures found in a given folder by recursive search. The compressed textures
will be stored at the same path with the same name/extension but with the '_compressed' suffix.
It will work with gltf files (ikari will look for a '_compressed' counterpart) but only if the
texture is in a separate file and not embedded in the gltf file.

Usage: clikari compress_textures --search_folder /path/to/folder [OPTIONS]

Options:
  --search_folder FOLDER      Required  The folder to search in to find textures to compress
  --max_thread_count VAL      Optional  The maximum number of threads used to process the data in parallel.
                                        Worker threads are spawned in parallel, each of which gets single texture 
                                        and spawns additional threads to process it in parallel according to the formula:
                                            threads_per_worker = (max_thread_count / texture_count).floor().max(1)
                                            worker_threads     = (max_thread_count / threads_per_worker).floor()
                                        Defaults to the number of logical CPUs on the system.
  --force                     Optional  Force re-compress all textures regardless of whether their
                                        _compressed.bin counterpart already exists
  --help                      Optional  Display this help message
";

const SKYBOX_PROCESSOR_HELP: &str = "\
Pre-process skybox file(s) for use in ikari

Usage: clikari process_skybox --background_path /path/to/background.jpg --out_folder /path/to/folder [OPTIONS]

Options:
  --background_path FILE        Required  The background image of the skybox (this will be the background of your scene)
  --environment_hdr_path FILE   Optional  The hdr environment map (used for ambient lighting and reflections)
                                          Background image is used if option is not supplied
  --out_folder FOLDER           Required  Output folder
  --help                        Optional  Display this help message
";

enum CommandName {
    CompressTextures,
    ProcessSkybox,
}

impl FromStr for CommandName {
    type Err = &'static str;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        match input {
            "compress_textures" => Ok(CommandName::CompressTextures),
            "process_skybox" => Ok(CommandName::ProcessSkybox),
            _ => Err("Command not recognized"),
        }
    }
}

enum Command {
    Help,
    CompressTextures(TextureCompressorArgs),
    CompressTexturesHelp,
    ProcessSkybox(SkyboxProcessorArgs),
    ProcessSkyboxHelp,
}

enum ArgParseError {
    Root(String),
    CompressTextures(String),
    ProcessSkybox(String),
}

impl Command {
    pub fn from_env() -> Result<Self, ArgParseError> {
        let mut args = pico_args::Arguments::from_env();

        let error_mapper = |err| ArgParseError::Root(format!("{err}"));
        let command_result: Result<CommandName, _> =
            args.value_from_str("--command").map_err(error_mapper);

        match command_result {
            Ok(CommandName::CompressTextures) => {
                if args.contains("--help") {
                    return Ok(Self::CompressTexturesHelp);
                }

                let error_mapper = |err| ArgParseError::CompressTextures(format!("{err}"));
                return Ok(Self::CompressTextures(TextureCompressorArgs {
                    search_folder: args
                        .value_from_str("--search_folder")
                        .map_err(error_mapper)?,
                    max_thread_count: args
                        .opt_value_from_str("--max_thread_count")
                        .map_err(error_mapper)?,
                    force: args.contains("--force"),
                }));
            }
            Ok(CommandName::ProcessSkybox) => {
                if args.contains("--help") {
                    return Ok(Self::ProcessSkyboxHelp);
                }

                let error_mapper = |err| ArgParseError::ProcessSkybox(format!("{err}"));
                return Ok(Self::ProcessSkybox(SkyboxProcessorArgs {
                    background_path: args
                        .value_from_str("--background_path")
                        .map_err(error_mapper)?,
                    environment_hdr_path: args
                        .opt_value_from_str("--environment_hdr_path")
                        .map_err(error_mapper)?,
                    out_folder: args.value_from_str("--out_folder").map_err(error_mapper)?,
                }));
            }
            _ => {}
        };

        if args.contains("--help") {
            return Ok(Self::Help);
        }

        // only show missing command error if --help is not supplied
        command_result?;

        Err(ArgParseError::Root(String::from("No command specified")))
    }
}

fn main() {
    if !env_var_is_defined("RUST_BACKTRACE") {
        std::env::set_var("RUST_BACKTRACE", "1");
    }

    if env_var_is_defined("RUST_LOG") {
        env_logger::init();
    } else {
        env_logger::builder()
            .filter_level(log::LevelFilter::Error)
            .filter(Some(env!("CARGO_PKG_NAME")), log::LevelFilter::Info)
            .filter(Some(env!("CARGO_BIN_NAME")), log::LevelFilter::Info)
            .filter(Some("ikari"), log::LevelFilter::Info)
            .filter(Some("wgpu"), log::LevelFilter::Warn)
            .init();
    }

    match Command::from_env() {
        Ok(Command::CompressTextures(args)) => {
            block_on(texture_compressor::run(args));
        }
        Ok(Command::ProcessSkybox(args)) => {
            block_on(skybox_processor::run(args));
        }
        Ok(Command::Help) => {
            println!("{HELP}");
        }
        Ok(Command::CompressTexturesHelp) => {
            println!("{TEXTURE_COMPRESSOR_HELP}");
        }
        Ok(Command::ProcessSkyboxHelp) => {
            println!("{SKYBOX_PROCESSOR_HELP}");
        }
        Err(err) => {
            let (err, helpmsg) = match err {
                ArgParseError::Root(err) => (err, HELP),
                ArgParseError::CompressTextures(err) => (err, TEXTURE_COMPRESSOR_HELP),
                ArgParseError::ProcessSkybox(err) => (err, SKYBOX_PROCESSOR_HELP),
            };
            println!("Error: {err}\n\n{helpmsg}");
        }
    };
}

fn env_var_is_defined(var: &str) -> bool {
    match std::env::var(var) {
        Ok(val) => !val.is_empty(),
        Err(_) => false,
    }
}
