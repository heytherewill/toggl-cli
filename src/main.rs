mod api;
mod arguments;
mod credentials;
mod models;
use api::{ApiClient, V9ApiClient};
use arguments::Command;
use arguments::Command::Auth;
use arguments::Command::Continue;
use arguments::Command::Current;
use arguments::Command::List;
use arguments::Command::Running;
use arguments::Command::Start;
use arguments::Command::Stop;
use arguments::CommandLineArguments;
use chrono::Utc;
use colored::Colorize;
use credentials::Credentials;
use models::{ResultWithDefaultError, TimeEntry};
use structopt::StructOpt;

const CLIENT_NAME: &str = "github.com/heytherewill/toggl-cli";

#[tokio::main]
async fn main() -> ResultWithDefaultError<()> {
    let parsed_args = CommandLineArguments::from_args();
    return execute_subcommand(parsed_args.cmd).await;
}

pub async fn execute_subcommand(command: Option<Command>) -> ResultWithDefaultError<()> {
    match command {
        None => display_running_time_entry().await?,
        Some(subcommand) => match subcommand {
            Current | Running => display_running_time_entry().await?,
            Stop => stop_running_time_entry().await?,
            Start {
                description: _,
                project: _,
            } => (),
            Continue => continue_time_entry().await?,
            Auth { api_token } => authenticate(api_token).await?,
            List { number } => display_time_entries(number).await?,
        },
    }

    Ok(())
}

fn ensure_authentication() -> ResultWithDefaultError<impl ApiClient> {
    return match Credentials::read() {
        Ok(credentials) => V9ApiClient::from_credentials(credentials),
        Err(err) => {
            println!(
                "{}\n{} {}",
                "Please set your API token first by calling toggl auth <API_TOKEN>.".red(),
                "You can find your API token at".blue().bold(),
                "https://track.toggl.com/profile".blue().bold().underline()
            );
            return Err(err);
        }
    };
}

async fn authenticate(api_token: String) -> ResultWithDefaultError<()> {
    let credentials = Credentials { api_token };
    let api_client = V9ApiClient::from_credentials(credentials)?;
    let user = api_client.get_user().await?;
    let _credentials = Credentials::persist(user.api_token)?;
    println!(
        "{} {}",
        "Successfully authenticated for user with email:".green(),
        user.email.green().bold(),
    );

    Ok(())
}

async fn display_running_time_entry() -> ResultWithDefaultError<()> {
    let api_client = ensure_authentication()?;
    match api_client.get_running_time_entry().await? {
        None => println!("{}", "No time entry is running at the moment".yellow()),
        Some(running_time_entry) => println!("{}", running_time_entry),
    }

    Ok(())
}

async fn continue_time_entry() -> ResultWithDefaultError<()> {
    let api_client = ensure_authentication()?;
    let time_entries = api_client.get_time_entries().await?;
    match time_entries.first() {
        None => println!("{}", "No time entries in last 90 days".red()),
        Some(time_entry) => {
            let continued_entry = start_time_entry(time_entry.clone()).await?;
            println!(
                "{}\n{}",
                "Time entry continued successfully".green(),
                continued_entry
            )
        }
    }
    Ok(())
}

async fn stop_running_time_entry() -> ResultWithDefaultError<()> {
    let api_client = ensure_authentication()?;
    match api_client.get_running_time_entry().await? {
        None => println!("{}", "No time entry is running at the moment".yellow()),
        Some(running_time_entry) => {
            let _stopped_time_entry = stop_time_entry(running_time_entry).await?;
            println!("{}", "Time entry stopped successfully".green());
        }
    }

    Ok(())
}

async fn display_time_entries(count: Option<usize>) -> ResultWithDefaultError<()> {
    let api_client = ensure_authentication()?;
    match api_client.get_time_entries().await {
        Err(error) => println!(
            "{}\n{}",
            "Couldn't fetch time entries the from API".red(),
            error
        ),
        Ok(time_entries) => time_entries
            .iter()
            .take(count.unwrap_or(usize::max_value()))
            .for_each(|time_entry| println!("{}", time_entry)),
    }

    Ok(())
}

async fn start_time_entry(time_entry: TimeEntry) -> ResultWithDefaultError<TimeEntry> {
    let api_client = ensure_authentication()?;
    let start = Utc::now();
    let time_entry_to_create = TimeEntry {
        start,
        stop: None,
        duration: -start.timestamp(),
        created_with: Some(CLIENT_NAME.to_string()),
        ..time_entry
    };
    return api_client.create_time_entry(time_entry_to_create).await;
}

async fn stop_time_entry(time_entry: TimeEntry) -> ResultWithDefaultError<TimeEntry> {
    let api_client = ensure_authentication()?;
    let stop_time = Utc::now();
    let stopped_time_entry = TimeEntry {
        stop: Some(stop_time),
        duration: (stop_time - time_entry.start).num_seconds(),
        ..time_entry
    };
    return api_client.update_time_entry(stopped_time_entry).await;
}
