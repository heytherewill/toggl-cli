mod api;
mod arguments;
mod credentials;
mod models;
use api::ApiClient;
use arguments::CommandLineArguments;
use arguments::Command;
use arguments::Command::Auth;
use arguments::Command::Current;
use arguments::Command::Running;
use arguments::Command::Stop;
use arguments::Command::Start;
use credentials::Credentials;
use structopt::StructOpt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let parsed_args = CommandLineArguments::from_args();
    return execute_subcommand(parsed_args.cmd).await;
}

pub async fn execute_subcommand(command: Option<Command>) -> Result<(), Box<dyn std::error::Error>> {

    match command {
        None => ensure_authentication(display_running_time_entry).await,
        Some(subcommand) => match subcommand {
            Current | Running => ensure_authentication(display_running_time_entry).await,
            Stop => ensure_authentication(stop_running_time_entry).await,
            Start { description: _, project: _ } => (),
            Auth { api_token } => {
                let credentials = Credentials { api_token: api_token };
                let api_client = ApiClient::from_credentials(credentials)?;
                authenticate(api_client).await
            }
        }
    }

    Ok(())
}

async fn ensure_authentication<F: std::future::Future<Output = ()>>(callback: fn(ApiClient) -> F) {
    match Credentials::read() {
        Some(credentials) => {
            match ApiClient::from_credentials(credentials) {
                Err(err) => println!("{}", err),
                Ok(api_client) => callback(api_client).await
            }
        } None => {
            println!("Please set your API token first by calling toggl auth <API_TOKEN>.");
            println!("You can find your API token at https://track.toggl.com/profile.");
        }
    }
}

async fn authenticate(api_client: ApiClient) {
    match api_client.get_user().await {
        Err(api_error) => println!("{}", api_error),
        Ok(user) => {
            match Credentials::persist(user.api_token) {
                Err(credential_error) => println!("{}", credential_error),
                Ok(_) => println!("Successfully authenticated for user with email {}", user.email)
            }
        }
    }
}

async fn display_running_time_entry(api_client: ApiClient) {
    let running_time_entry = api_client.get_running_time_entry().await;
    match running_time_entry {
        Err(error) => println!("{:?}", error),
        Ok(running_time_entry) => {
            match running_time_entry {
                None => println!("No time entry is running at the moment"),
                Some(running_time_entry) => println!("{}", running_time_entry.description)
            }
        }
    }
}

async fn stop_running_time_entry(api_client: ApiClient) {
    let running_time_entry = api_client.get_running_time_entry().await;
    match running_time_entry {
        Err(error) => println!("{:?}", error),
        Ok(running_time_entry) => {
            match running_time_entry {
                None => println!("No time entry is running at the moment"),
                Some(time_entry) => {
                    match api_client.stop_time_entry(time_entry).await {
                        Err(stop_error) => println!("{:?}", stop_error),
                        Ok(_) => println!("Time entry stopped successfully")
                    }
                }
            }
        }
    }
}
