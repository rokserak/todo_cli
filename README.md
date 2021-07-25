# To Do CLI

Simple CLI interface for interaction with Microsoft To Do app.

## Installation

- Register new Microsoft Graph app, follow [official instructions](https://docs.microsoft.com/en-us/graph/auth-register-app-v2) 

- Create `.env` file with following variables, for development
```dotenv
TODO_CLI_CLIENT_ID="MS Graph client id"
TODO_CLI_CLIENT_SECRET="MS Graph client secret"
TODO_CLI_AUTH_URL=https://login.microsoftonline.com/common/oauth2/v2.0/authorize
TODO_CLI_TOKEN_URL=https://login.microsoftonline.com/common/oauth2/v2.0/token
TODO_CLI_REDIRECT_URL=http://localhost:7594/auth
TODO_CLI_DEFAULT_TASK_LIST=infinCUBE
ROCKET_PORT=7495
```

- Or export variables in `.env` file to your bash profile to just use app after compilation 


- Compile
```shell
cargo build --release

# or for development
cargo build
cargo run
```

- Run CLI
````shell
./target/release/todo --help
````

## Usage

- List all open tasks
```shell
./todo --list
```

- Create new task, with task text "New task content"
```shell
./todo New task content
```

- To list or create tasks on specific Task List use `--task_list` flag, by default we use task list defined by `TODO_CLI_DEFAULT_TASK_LIST` variable.
```shell
./todo --list --task_list list_name
./todo --task_list list_name New task text
```
