use std::ffi::OsString;
use std::path::PathBuf;

pub(crate) const USAGE: &str = "Invokrum deterministic overlay composition\n\nUSAGE:\n  invokrum [--version]\n  invokrum validate --pack <path> [--profile <id>] [--format human|json]\n  invokrum compose --pack <path> --profile <id> [--output <path>] [--force]\n  invokrum inspect --pack <path> --profile <id> [--format human|json]\n  invokrum lock --pack <path> --profile <id> [--output <path>] [--force]\n  invokrum verify --lock <path> --pack <path> --profile <id> [--format human|json]\n  invokrum diff <baseline-lock> <candidate-lock> [--format human|json]\n  invokrum install <candidate> --bundle-manifest <path> --subject <sha256> [install options]\n  invokrum rpc\n\nOPTIONS:\n  --force       Atomically replace an existing regular output file.\n  --format      Select human or stable JSON output.\n  --no-color    Accepted for automation; Invokrum currently emits no ANSI output.\n  --output      Write raw context or canonical lock bytes to a file instead of stdout.\n\nINSTALL:\n  Run `invokrum install --help` for candidate-format, publisher-authentication, trust, and store options.\n\nRPC:\n  Reads one bounded invokrum.host/v1 JSON request from stdin.\n  Writes one JSON response to stdout and never performs writes or network access.\n";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum OutputFormat {
    Human,
    Json,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct PackSelection {
    pub pack: PathBuf,
    pub profile: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Destination {
    pub output: Option<PathBuf>,
    pub force: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum Command {
    Version,
    Help,
    Validate {
        pack: PathBuf,
        profile: Option<String>,
        format: OutputFormat,
    },
    Compose {
        selection: PackSelection,
        destination: Destination,
    },
    Inspect {
        selection: PackSelection,
        format: OutputFormat,
    },
    Lock {
        selection: PackSelection,
        destination: Destination,
    },
    Verify {
        lock: PathBuf,
        selection: PackSelection,
        format: OutputFormat,
    },
    Diff {
        baseline: PathBuf,
        candidate: PathBuf,
        format: OutputFormat,
    },
    Rpc,
}

pub(crate) fn parse(arguments: impl IntoIterator<Item = OsString>) -> Result<Command, String> {
    let arguments = arguments
        .into_iter()
        .map(|argument| {
            argument
                .into_string()
                .map_err(|_| "arguments must be valid UTF-8".to_owned())
        })
        .collect::<Result<Vec<_>, _>>()?;
    let mut cursor = Cursor::new(arguments);
    let Some(command) = cursor.next() else {
        return Ok(Command::Version);
    };

    match command.as_str() {
        "--version" | "version" => finish(cursor, Command::Version),
        "--help" | "-h" | "help" => finish(cursor, Command::Help),
        "validate" => parse_validate(cursor),
        "compose" => parse_compose(cursor),
        "inspect" => parse_inspect(cursor),
        "lock" => parse_lock(cursor),
        "verify" => parse_verify(cursor),
        "diff" => parse_diff(cursor),
        "rpc" => parse_rpc(cursor),
        _ => Err(format!("unknown command `{command}`")),
    }
}

fn parse_validate(mut cursor: Cursor) -> Result<Command, String> {
    let mut pack = None;
    let mut profile = None;
    let mut format = OutputFormat::Human;
    let mut format_seen = false;

    while let Some(argument) = cursor.next() {
        match argument.as_str() {
            "--pack" => assign(&mut pack, PathBuf::from(cursor.value("--pack")?), "--pack")?,
            "--profile" => assign(&mut profile, cursor.value("--profile")?, "--profile")?,
            "--format" => {
                if format_seen {
                    return Err("option `--format` was supplied more than once".to_owned());
                }
                format = parse_format(&cursor.value("--format")?)?;
                format_seen = true;
            }
            "--no-color" => {}
            "--help" | "-h" => return finish(cursor, Command::Help),
            _ => return Err(format!("unknown validate option `{argument}`")),
        }
    }

    Ok(Command::Validate {
        pack: required(pack, "--pack")?,
        profile,
        format,
    })
}

fn parse_compose(mut cursor: Cursor) -> Result<Command, String> {
    let (selection, destination) = parse_pack_destination(&mut cursor, "compose")?;
    Ok(Command::Compose {
        selection,
        destination,
    })
}

fn parse_inspect(mut cursor: Cursor) -> Result<Command, String> {
    let mut pack = None;
    let mut profile = None;
    let mut format = OutputFormat::Human;
    let mut format_seen = false;

    while let Some(argument) = cursor.next() {
        match argument.as_str() {
            "--pack" => assign(&mut pack, PathBuf::from(cursor.value("--pack")?), "--pack")?,
            "--profile" => assign(&mut profile, cursor.value("--profile")?, "--profile")?,
            "--format" => {
                if format_seen {
                    return Err("option `--format` was supplied more than once".to_owned());
                }
                format = parse_format(&cursor.value("--format")?)?;
                format_seen = true;
            }
            "--no-color" => {}
            "--help" | "-h" => return finish(cursor, Command::Help),
            _ => return Err(format!("unknown inspect option `{argument}`")),
        }
    }

    Ok(Command::Inspect {
        selection: PackSelection {
            pack: required(pack, "--pack")?,
            profile: required(profile, "--profile")?,
        },
        format,
    })
}

fn parse_lock(mut cursor: Cursor) -> Result<Command, String> {
    let (selection, destination) = parse_pack_destination(&mut cursor, "lock")?;
    Ok(Command::Lock {
        selection,
        destination,
    })
}

fn parse_verify(mut cursor: Cursor) -> Result<Command, String> {
    let mut lock = None;
    let mut pack = None;
    let mut profile = None;
    let mut format = OutputFormat::Human;
    let mut format_seen = false;

    while let Some(argument) = cursor.next() {
        match argument.as_str() {
            "--lock" => assign(&mut lock, PathBuf::from(cursor.value("--lock")?), "--lock")?,
            "--pack" => assign(&mut pack, PathBuf::from(cursor.value("--pack")?), "--pack")?,
            "--profile" => assign(&mut profile, cursor.value("--profile")?, "--profile")?,
            "--format" => {
                if format_seen {
                    return Err("option `--format` was supplied more than once".to_owned());
                }
                format = parse_format(&cursor.value("--format")?)?;
                format_seen = true;
            }
            "--no-color" => {}
            "--help" | "-h" => return finish(cursor, Command::Help),
            _ => return Err(format!("unknown verify option `{argument}`")),
        }
    }

    Ok(Command::Verify {
        lock: required(lock, "--lock")?,
        selection: PackSelection {
            pack: required(pack, "--pack")?,
            profile: required(profile, "--profile")?,
        },
        format,
    })
}

fn parse_diff(mut cursor: Cursor) -> Result<Command, String> {
    let mut paths = Vec::new();
    let mut format = OutputFormat::Human;
    let mut format_seen = false;

    while let Some(argument) = cursor.next() {
        match argument.as_str() {
            "--format" => {
                if format_seen {
                    return Err("option `--format` was supplied more than once".to_owned());
                }
                format = parse_format(&cursor.value("--format")?)?;
                format_seen = true;
            }
            "--no-color" => {}
            "--help" | "-h" => return finish(cursor, Command::Help),
            option if option.starts_with('-') => {
                return Err(format!("unknown diff option `{option}`"));
            }
            path => paths.push(PathBuf::from(path)),
        }
    }

    if paths.len() != 2 {
        return Err("diff requires exactly a baseline lock and candidate lock".to_owned());
    }
    let candidate = paths.pop().expect("length checked");
    let baseline = paths.pop().expect("length checked");
    Ok(Command::Diff {
        baseline,
        candidate,
        format,
    })
}

fn parse_rpc(cursor: Cursor) -> Result<Command, String> {
    finish(cursor, Command::Rpc)
}

fn parse_pack_destination(
    cursor: &mut Cursor,
    command: &str,
) -> Result<(PackSelection, Destination), String> {
    let mut pack = None;
    let mut profile = None;
    let mut output = None;
    let mut force = false;
    let mut force_seen = false;

    while let Some(argument) = cursor.next() {
        match argument.as_str() {
            "--pack" => assign(&mut pack, PathBuf::from(cursor.value("--pack")?), "--pack")?,
            "--profile" => assign(&mut profile, cursor.value("--profile")?, "--profile")?,
            "--output" => assign(
                &mut output,
                PathBuf::from(cursor.value("--output")?),
                "--output",
            )?,
            "--force" => {
                if force_seen {
                    return Err("option `--force` was supplied more than once".to_owned());
                }
                force = true;
                force_seen = true;
            }
            "--no-color" => {}
            "--help" | "-h" => return Err("help".to_owned()),
            _ => return Err(format!("unknown {command} option `{argument}`")),
        }
    }

    if force && output.is_none() {
        return Err("`--force` requires `--output`".to_owned());
    }
    Ok((
        PackSelection {
            pack: required(pack, "--pack")?,
            profile: required(profile, "--profile")?,
        },
        Destination { output, force },
    ))
}

fn parse_format(value: &str) -> Result<OutputFormat, String> {
    match value {
        "human" => Ok(OutputFormat::Human),
        "json" => Ok(OutputFormat::Json),
        _ => Err(format!("unsupported output format `{value}`")),
    }
}

fn assign<T>(slot: &mut Option<T>, value: T, option: &str) -> Result<(), String> {
    if slot.replace(value).is_some() {
        Err(format!("option `{option}` was supplied more than once"))
    } else {
        Ok(())
    }
}

fn required<T>(value: Option<T>, option: &str) -> Result<T, String> {
    value.ok_or_else(|| format!("missing required option `{option}`"))
}

fn finish(mut cursor: Cursor, command: Command) -> Result<Command, String> {
    if let Some(argument) = cursor.next() {
        Err(format!("unexpected argument `{argument}`"))
    } else {
        Ok(command)
    }
}

#[derive(Clone, Debug)]
struct Cursor {
    arguments: Vec<String>,
    index: usize,
}

impl Cursor {
    fn new(arguments: Vec<String>) -> Self {
        Self {
            arguments,
            index: 0,
        }
    }

    fn next(&mut self) -> Option<String> {
        let value = self.arguments.get(self.index)?.clone();
        self.index += 1;
        Some(value)
    }

    fn value(&mut self, option: &str) -> Result<String, String> {
        self.next()
            .ok_or_else(|| format!("option `{option}` requires a value"))
    }
}
