use oauth2::basic::{BasicClient, BasicTokenResponse};
use oauth2::reqwest::http_client;
use oauth2::{AuthType, AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken, RedirectUrl, Scope, TokenUrl, TokenResponse};
use std::{thread, fs, io};
use std::time::Duration;
use std::fs::File;
use std::io::prelude::*;
use dotenv::dotenv;
use std::path::{Path, PathBuf};
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};


extern crate dotenv;
#[macro_use] extern crate rocket;


fn main() {
  dotenv().ok();

  let token_file = get_token_file_path().unwrap();
  if !token_file.exists() {
    authenticate_user();
  }

  let task_lists = get_task_lists();
  for list in task_lists {
    println!("{} - {}", list.displayName, list.id);
    let tasks = get_tasks_on_list(list);
    for (i, task) in tasks.iter().enumerate() {
      println!("{} - {}", i, task.title);
    }
  }
}

#[rocket::main]
async fn launch_rocket() {
  let _ = rocket::build().mount("/", routes![auth_route]).launch().await;
}

fn authenticate_user() {
  thread::spawn(|| {
    launch_rocket();
  });
  thread::sleep(Duration::from_secs(1));

  let client_id = dotenv::var("TODO_CLI_CLIENT_ID").ok().unwrap();
  let client_secret = dotenv::var("TODO_CLI_CLIENT_SECRET").ok().unwrap();
  let auth_url = dotenv::var("TODO_CLI_AUTH_URL").ok().unwrap();
  let token_url = dotenv::var("TODO_CLI_TOKEN_URL").ok().unwrap();
  let redirect_url = dotenv::var("TODO_CLI_REDIRECT_URL").ok().unwrap();

  let graph_client_id = ClientId::new(client_id);
  let graph_client_secret = ClientSecret::new(client_secret);
  let graph_auth_url = AuthUrl::new(auth_url).expect("Invalid authorization endpoint URL");
  let graph_token_url = TokenUrl::new(token_url).expect("Invalid token endpoint URL");

  let client = BasicClient::new(graph_client_id, Some(graph_client_secret), graph_auth_url,
                                Some(graph_token_url))
    .set_auth_type(AuthType::RequestBody)
    .set_redirect_url(RedirectUrl::new(redirect_url).expect("Invalid redirect URL"));

  let (authorize_url, _csrf_state) = client
    .authorize_url(CsrfToken::new_random)
    .add_scope(Scope::new("offline_access".to_string()))
    .add_scope(Scope::new("https://graph.microsoft.com/Tasks.ReadWrite".to_string()))
    .add_scope(Scope::new("https://graph.microsoft.com/Tasks.ReadWrite.Shared".to_string()))
    .url();

  println!("Open this URL in your browser:\n{}\n", authorize_url.to_string());

  let token_file = get_token_file_path().unwrap();
  loop {
    if token_file.exists() {
      break;
    }
  }
}

#[get("/auth?<code>")]
fn auth_route(code: &str) -> &'static str {
  let client_id = dotenv::var("TODO_CLI_CLIENT_ID").ok().unwrap();
  let auth_url = dotenv::var("TODO_CLI_AUTH_URL").ok().unwrap();
  let token_url = dotenv::var("TODO_CLI_TOKEN_URL").ok().unwrap();
  let redirect_url = dotenv::var("TODO_CLI_REDIRECT_URL").ok().unwrap();

  let graph_client_id = ClientId::new(client_id);
  let graph_auth_url = AuthUrl::new(auth_url).expect("Invalid authorization endpoint URL");
  let graph_token_url = TokenUrl::new(token_url).expect("Invalid token endpoint URL");

  let client = BasicClient::new(graph_client_id, None, graph_auth_url,
                                Some(graph_token_url))
    .set_auth_type(AuthType::RequestBody)
    .set_redirect_url(RedirectUrl::new(redirect_url).expect("Invalid redirect URL"));

  let token = client
    .exchange_code(AuthorizationCode::new(code.to_string()))
    .request(http_client);

  if token.is_ok() {
    save_token(&token.unwrap());
  } else {
    return "Something went wrong during authentication. Please try again.";
  }

  return "You have successfully authenticated, this tab can now be closed.";
}

fn get_token_file_path() -> io::Result<PathBuf> {
  let mut cli_config_dir = dirs::config_dir().unwrap().to_str().unwrap().to_owned();
  cli_config_dir.push_str("/todo_cli");
  if !Path::new(cli_config_dir.as_str()).exists() {
    let _ = fs::create_dir(cli_config_dir);
  }

  let mut token_file_path = dirs::config_dir().unwrap().to_str().unwrap().to_owned();
  token_file_path.push_str("/todo_cli/token.yaml");
  return Ok(Path::new(token_file_path.as_str()).to_owned());
}

fn get_token() -> BasicTokenResponse {
  let token_file = get_token_file_path().unwrap();
  let token = fs::read_to_string(token_file).expect("Failed to read token");
  let token: BasicTokenResponse = serde_yaml::from_str(token.as_str()).unwrap();
  return token;
}

fn save_token(token: &BasicTokenResponse) {
  let token_file_path = get_token_file_path().unwrap();
  let mut token_file = File::create(token_file_path.to_str().unwrap()).unwrap();
  let token_serialized = serde_yaml::to_string(token).ok().unwrap();
  let _ = token_file.write_all(token_serialized.as_bytes());
}

fn refresh_token(token: BasicTokenResponse) -> BasicTokenResponse {
  let client_id = dotenv::var("TODO_CLI_CLIENT_ID").ok().unwrap();
  let auth_url = dotenv::var("TODO_CLI_AUTH_URL").ok().unwrap();
  let token_url = dotenv::var("TODO_CLI_TOKEN_URL").ok().unwrap();

  let graph_client_id = ClientId::new(client_id);
  let graph_auth_url = AuthUrl::new(auth_url).expect("Invalid authorization endpoint URL");
  let graph_token_url = TokenUrl::new(token_url).expect("Invalid token endpoint URL");

  let client = BasicClient::new(graph_client_id, None, graph_auth_url, Some(graph_token_url))
    .set_auth_type(AuthType::RequestBody);

  let _refresh_token = token.refresh_token().unwrap();
  let new_token_request = client.exchange_refresh_token(_refresh_token)
    .request(http_client);
  let new_token = new_token_request.unwrap();
  save_token(&new_token);
  return new_token;
}

fn get_request(url: &str) -> String {
  let mut token = get_token();
  token = refresh_token(token);

  let client = Client::new();
  let response = client
    .get(url)
    .header("Authorization", token.access_token().secret().as_str())
    .send().unwrap();

  let response_text = response.text().unwrap();
  return response_text;
}

#[derive(Serialize, Deserialize, Debug)]
struct TaskList {
  displayName: String,
  id: String
}

#[derive(Serialize, Deserialize, Debug)]
struct TaskLists {
  value: Vec<TaskList>
}

fn get_task_lists() -> Vec<TaskList> {
  let response_text = get_request("https://graph.microsoft.com/v1.0/me/todo/lists");
  let task_lists: TaskLists = serde_json::from_str(response_text.as_str()).unwrap();
  return task_lists.value;
}

#[derive(Serialize, Deserialize, Debug)]
struct Task {
  title: String,
  status: String
}

#[derive(Serialize, Deserialize, Debug)]
struct TasksOnList {
  value: Vec<Task>
}

fn get_tasks_on_list(task_list: TaskList) -> Vec<Task> {
  let url = format!("https://graph.microsoft.com/v1.0/me/todo/lists/{}/tasks", task_list.id);
  let response_text = get_request(url.as_str());
  let tasks_on_list: TasksOnList = serde_json::from_str(response_text.as_str()).unwrap();
  let tasks = tasks_on_list.value;
  return tasks.into_iter().filter(|task| task.status == "notStarted").collect();
}
