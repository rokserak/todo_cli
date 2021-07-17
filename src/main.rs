use oauth2::basic::BasicClient;
use oauth2::reqwest::http_client;
use oauth2::{AuthType, AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken, RedirectUrl, Scope, TokenUrl};
use std::{thread, fs};
use std::time::Duration;
extern crate dotenv;
use std::fs::File;
use std::io::prelude::*;
use dotenv::dotenv;

#[macro_use] extern crate rocket;


fn main() {
  dotenv().ok();

  thread::spawn(|| {
    launch_rocket();
  });
  thread::sleep(Duration::from_secs(1));
  authenticate_user();
}

#[rocket::main]
async fn launch_rocket() {
  let _ = rocket::build().mount("/", routes![auth_route]).launch().await;
}

fn authenticate_user() {
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
  while true {}
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
    let token_serialized = serde_yaml::to_string(&token.unwrap()).ok().unwrap();

    let mut cli_config_dir = dirs::config_dir().unwrap().to_str().unwrap().to_owned();
    cli_config_dir.push_str("/todo_cli");
    fs::create_dir(cli_config_dir);

    let mut token_file_path = dirs::config_dir().unwrap().to_str().unwrap().to_owned();
    token_file_path.push_str("/todo_cli/token.yaml");
    let mut file = File::create(token_file_path.as_str()).unwrap();
    file.write_all(token_serialized.as_bytes());
  } else {
    return "Something went wrong during authentication. Please try again.";
  }

  return "You have successfully authenticated, this tab can now be closed.";
}
