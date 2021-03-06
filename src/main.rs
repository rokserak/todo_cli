use oauth2::basic::{BasicClient, BasicTokenResponse};
use oauth2::reqwest::http_client;
use oauth2::{AuthType, AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken, RedirectUrl, Scope, TokenUrl, TokenResponse};
use std::{thread, fs, io, env};
use std::time::Duration;
use std::fs::File;
use std::io::prelude::*;
use dotenv::dotenv;
use std::path::{Path, PathBuf};
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use reqwest::StatusCode;
use webbrowser;


extern crate dotenv;
extern crate clap;
use clap::{Arg, App, Values};
#[macro_use] extern crate rocket;


fn main() {
  dotenv().ok();

  let matches = App::new("To Do CLI")
    .about("CLI for viewing and creating tasks inside Microsoft To Do App")
    .arg(Arg::with_name("list")
      .short("l")
      .long("list")
      .help("List tasks on specified task list"))
    .arg(Arg::with_name("task_list")
      .short("t")
      .long("task_list")
      .takes_value(true)
      .help("Task list that we want to view or create task on"))
    .arg(Arg::with_name("task_text")
      .required(false)
      .min_values(0))
    .get_matches();

  let list = matches.is_present("list");
  let default_task_list = env::var("TODO_CLI_DEFAULT_TASK_LIST").ok().unwrap();
  let task_list_name = matches.value_of("task_list").unwrap_or(default_task_list.as_str());
  let task_text_values: Values = matches.values_of("task_text").unwrap_or(Values::default());

  let token_file = get_token_file_path().unwrap();
  if !token_file.exists() {
    authenticate_user();
  }

  let task_lists = get_task_lists();
  let task_list = task_lists
    .iter()
    .filter(|task| task.displayName == String::from(task_list_name))
    .last();

  if task_list.is_none() {
    println!("Task List {} does not exist", task_list_name);
    return;
  }

  let task_list = task_list.unwrap();
  println!("Task List: {}", task_list_name);

  if list {
    let tasks = get_tasks_on_list(task_list);
    for (i, task) in tasks.iter().enumerate() {
      println!("Task {} | {} | Status: {}", i, task.title, task.status);
    }
  } else {
    let mut task_text = String::from("");
    for value in task_text_values {
      task_text.push_str(value);
      task_text.push_str(" ");
    }
    if task_text == "" {
      println!("No task created, missing task text");
      return;
    }
    task_text = String::from(task_text.strip_suffix(" ").unwrap());
    println!("Added task: {:?}", task_text);
    create_task(task_list, task_text);
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

  let client_id = env::var("TODO_CLI_CLIENT_ID").ok().unwrap();
  let client_secret = env::var("TODO_CLI_CLIENT_SECRET").ok().unwrap();
  let auth_url = env::var("TODO_CLI_AUTH_URL").ok().unwrap();
  let token_url = env::var("TODO_CLI_TOKEN_URL").ok().unwrap();
  let redirect_url = env::var("TODO_CLI_REDIRECT_URL").ok().unwrap();

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

  let _ = webbrowser::open(authorize_url.as_str());

  let token_file = get_token_file_path().unwrap();
  loop {
    if token_file.exists() {
      thread::sleep(Duration::from_millis(10));
      break;
    }
  }
}

#[get("/auth?<code>")]
fn auth_route(code: &str) -> &'static str {
  let client_id = env::var("TODO_CLI_CLIENT_ID").ok().unwrap();
  let auth_url = env::var("TODO_CLI_AUTH_URL").ok().unwrap();
  let token_url = env::var("TODO_CLI_TOKEN_URL").ok().unwrap();
  let redirect_url = env::var("TODO_CLI_REDIRECT_URL").ok().unwrap();

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
  let client_id = env::var("TODO_CLI_CLIENT_ID").ok().unwrap();
  let auth_url = env::var("TODO_CLI_AUTH_URL").ok().unwrap();
  let token_url = env::var("TODO_CLI_TOKEN_URL").ok().unwrap();

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

  if response.status() == StatusCode::UNAUTHORIZED {
    refresh_token(token);
    return get_request(url);
  }

  let response_text = response.text().unwrap();
  return response_text;
}

fn post_request<T: Serialize>(url: &str, body: &T) -> String {
  let mut token = get_token();
  token = refresh_token(token);

  let client = Client::new();
  let response = client
    .post(url)
    .header("Authorization", token.access_token().secret().as_str())
    .json(body)
    .send().unwrap();

  if response.status() == StatusCode::UNAUTHORIZED {
    refresh_token(token);
    return post_request(url, body);
  }

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

fn get_tasks_on_list(task_list: &TaskList) -> Vec<Task> {
  let url = format!("https://graph.microsoft.com/v1.0/me/todo/lists/{}/tasks?$filter=status eq 'notStarted'", task_list.id);
  let response_text = get_request(url.as_str());
  let tasks_on_list: TasksOnList = serde_json::from_str(response_text.as_str()).unwrap();
  let tasks = tasks_on_list.value;
  return tasks;
}

#[derive(Serialize, Deserialize, Debug)]
struct NewTask {
  title: String
}

fn create_task(task_list: &TaskList, task_content: String) {
  let url = format!("https://graph.microsoft.com/v1.0/me/todo/lists/{}/tasks", task_list.id);
  let task = NewTask {
    title: task_content
  };
  let _ = post_request(url.as_str(), &task);
}
