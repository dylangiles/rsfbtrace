use clap::Parser;
use std::error::Error;
use std::fs::OpenOptions;
use std::io::{Error as IOError, Result as IOResult, Write};
use std::process::Command;

const OPT_CONNECTIONS: &str = "connections";
const OPT_TRANSACTIONS: &str = "transactions";
const OPT_STATEMENT_PREPARE: &str = "statement_prepare";
const OPT_STATEMENT_FREE: &str = "statement_free";
const OPT_STATEMENT_START: &str = "statement_start";
const OPT_STATEMENT_FINISH: &str = "statement_finish";
const OPT_PROCEDURE_START: &str = "procedure_start";
const OPT_PROCEDURE_FINISH: &str = "procedure_finish";
const OPT_TRIGGER_START: &str = "trigger_start";
const OPT_TRIGGER_FINISH: &str = "trigger_finish";
const OPT_CONTEXT: &str = "context";
const OPT_ERRORS: &str = "errors";
const OPT_SWEEP: &str = "sweep";

const LEGAL_OPTS: &[&str] = &[
    OPT_TRANSACTIONS,
    OPT_STATEMENT_PREPARE,
    OPT_STATEMENT_FREE,
    OPT_STATEMENT_START,
    OPT_STATEMENT_FINISH,
    OPT_PROCEDURE_START,
    OPT_PROCEDURE_FINISH,
    OPT_TRIGGER_START,
    OPT_TRIGGER_FINISH,
    OPT_CONTEXT,
    OPT_ERRORS,
    OPT_SWEEP,
];

const CONFIG_FILE_NAME: &str = "fbtrace.conf";
const TRACE_NAME: &str = "rust-fbtrace";

#[derive(Debug)]
enum AppError {
    InvalidOpt(String),
    Dyn(Box<dyn Error>),
    Io(IOError),
}

impl std::error::Error for AppError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }

    fn description(&self) -> &str {
        "description() is deprecated; use Display"
    }

    fn cause(&self) -> Option<&dyn Error> {
        self.source()
    }
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidOpt(o) => {
                write!(
                    f,
                    "The specified event '{o}' is not a valid for subscription. Valid events are {LEGAL_OPTS:?}."
                )
            }
            Self::Dyn(d) => write!(f, "{d}"),
            Self::Io(i) => write!(f, "{i}"),
        }
    }
}

#[derive(Parser, Debug)]
#[command(author, about, long_about = None)]
struct Args {
    /// Optional remote hostname
    #[arg(long, default_value = None)]
    host: Option<String>,

    /// Optional SQL filter
    #[arg(short, long)]
    include_filter: Option<String>,

    /// Firebird username
    #[arg(short, long)]
    user: String,

    /// Firebird password
    #[arg(short, long)]
    pass: String,

    #[arg(short, long, default_value_t = 65536)]
    max_sql: usize,

    /// Database matcher [default: all databases]
    #[arg(short, long, default_value = None)]
    database_matcher: Option<String>,

    #[arg(short, long, num_args(1..))]
    events: Vec<String>,
}

fn main() -> Result<(), AppError> {
    let args: Args = Args::parse();
    for event in &args.events {
        if !LEGAL_OPTS.contains(&event.as_str()) {
            let err = AppError::InvalidOpt(event.into());
            println!("{err}");
            return Err(err);
        }
    }

    if let Err(e) = write_config_file(&args) {
        return Err(AppError::Io(e));
    };

    let _ = match Command::new("fbtracemgr")
        .args([
            "-SE",
            &args
                .host
                .as_ref() // required because .map_or takes an owned
                .map_or("service_mgr".into(), |x| format!("{x}:service_mgr"))
                .as_str(),
            "-USER",
            &args.user,
            "-PASS",
            &args.pass,
            "-START",
            "-NAME",
            TRACE_NAME,
            "-CONFIG",
            CONFIG_FILE_NAME,
        ])
        .spawn()
    {
        Ok(mut r) => r.wait(),
        Err(e) => return Err(AppError::Dyn(Box::new(e))),
    };

    Ok(())
}

fn write_config_file(args: &Args) -> IOResult<()> {
    let mut f = match OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(CONFIG_FILE_NAME)
    {
        Ok(fi) => fi,
        Err(e) => {
            println!("{e} at open file");
            return Err(e);
        }
    };

    macro_rules! e {
        ($event:expr) => {{
            let temp: String = $event.into();
            args.events.contains(&temp)
        }};
    }

    let db_pattern = match &args.database_matcher {
        Some(p) => format!("<database {p}>"),
        None => "<database>".into(),
    };

    f.write_all(
        format!(
            r#"
{}
    enabled true
    {}
    log_connections {}
    log_transactions {}
    log_statement_prepare {}
    log_statement_free {}
    log_statement_start {} 
    log_statement_finish {}
    log_procedure_start {}
    log_procedure_finish {}
    log_trigger_start {}
    log_trigger_finish {}
    log_context {}
    log_errors {}
    log_sweep {}
    print_plan false
    print_perf false
    log_blr_requests false
    print_blr false
    log_dyn_requests false
    print_dyn false
    time_threshold 100
    max_sql_length {}
    max_blr_length 500
    max_dyn_length 500
    max_arg_length 80
    max_arg_count 30
</database>"#,
            db_pattern,
            if let Some(inc) = &args.include_filter {
                format!(r#"include_filter "{inc}""#)
            } else {
                "".into()
            },
            e!(OPT_CONNECTIONS),
            e!(OPT_TRANSACTIONS),
            e!(OPT_STATEMENT_PREPARE),
            e!(OPT_STATEMENT_FREE),
            e!(OPT_STATEMENT_START),
            e!(OPT_STATEMENT_FINISH),
            e!(OPT_PROCEDURE_START),
            e!(OPT_PROCEDURE_FINISH),
            e!(OPT_TRIGGER_START),
            e!(OPT_TRIGGER_FINISH),
            e!(OPT_CONTEXT),
            e!(OPT_ERRORS),
            e!(OPT_SWEEP),
            &args.max_sql
        )
        .as_bytes(),
    )?;

    Ok(())
}
