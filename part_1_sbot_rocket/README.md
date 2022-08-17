# lykin tutorial

## Part 1: Sbot and Web Server

### Introduction

This is the first installment in a tutorial series that will walk you through the creation of a simple Scuttlebutt client application. The application, named lykin, will present an email inbox-like UI and function as a Scuttlebutt reader.

A summary of the features:

 - Subscribe to a peer
 - Unsubscribe from a peer
 - List subscribed peers
 - List root posts for each peer
 - Read a post
 - Mark a post as read
 - Mark a post as unread
 - Delete a post
 - Fetch new posts

lykin will use [golgi](http://golgi.mycelial.technology/), a Rust RPC library, to demonstrate the following Scuttlebutt commands:

 - whoami()
 - friends_is_following()
 - follow()
 - unfollow()
 - create_history_stream()

Filtering of messages will also be demonstrated, for example: how to obtain only root posts from the totality of peer messages.

### Prerequisities

lykin is written primarily in Rust and also includes basic HTML and CSS. I'm assuming you have some experience with the Rust programming language and have a development environment installed. See the [Rust Getting started page](https://www.rust-lang.org/learn/get-started) for installation details.

### Outline

Here's what we'll tackle in this first part of the series:

 - Install and configure go-sbot
 - Setup a basic web server and home route
 - Initialise a connection to the sbot
 - Call the `whoami` RPC method
 - Display the `whoami` ID via the home route

### Install and Configure Go-Sbot

lykin is a client application that connects to a Scuttlebutt node - generally referred to as an sbot or Scuttlebutt server. The sbot does all of the heavy-lifting for us; it stores Scuttlebutt messages in an append-only log and replicates those messages between peers. Our application will use the golgi Rust library to interact with an instance of [go-sbot](https://github.com/ssbc/go-ssb) (an implementation of a Scuttlebutt server written in Go).

Begin by following the [installation instructions](https://github.com/ssbc/go-ssb#installation) for GoSSB. Running `go-sbot` for the first time will create and populate the `$HOME/.ssb-go` on your computer. Open `$HOME/.ssb-go/config.toml` to configure your sbot. I recommend setting `hops = 1` and defining an IP and port combination which will not cause a conflict with any other Scuttlebutt application you may be running (I use the following: `lis = "127.0.0.1:8021"`).

You may wish to define a `systemd` to easily start and stop the `go-sbot`:

```bash
cat > /etc/systemd/system/go-sbot.service<< EOF
[Unit]
Description=GoSSB server.

[Service]
User=$USER
ExecStart=/usr/bin/go-sbot
Restart=always

[Install]
WantedBy=multi-user.target
EOF
```

Then reload the `systemd` configuration and start the service:

```bash
sudo systemctl daemon-reload
sudo systemctl start go-sbot.service
```

### Create a New Rust Project

Now we're almost ready to start writing some Rust code. Let's create a home for our project and ensure that our Rust development environment is working as expected:

```bash
cargo new lykin --bin
cd lykin
cargo run
```

You should see output similar to the following:

```bash
Compiling lykin v0.1.0 (/home/glyph/Projects/rust/lykin)
Finished dev [unoptimized + debuginfo] target(s) in 0.27s
Running `target/debug/lykin`
Hello, world!
```

### Setup the Web Server

We will be using [Rocket](https://rocket.rs/) as the web server for our application. I have chosen Rocket because it is well-known, thoroughly documented and feature rich. However, be aware that it suffers from dependency bloat and is not a good candidate for lean applications or low-resource development environments.

Add the latest version of Rocket to your `Cargo.toml` file (manifest):

`rocket = "0.5.0-rc.1"`

Now let's write the code we need to deploy a server with a single route:

`src/main.rs`

```rust
use rocket::{get, launch, routes};

#[get("/")]
async fn home() -> String {
    String::from("lykin")
}

#[launch]
async fn rocket() -> _ {
    rocket::build().mount("/", routes![home])
}
```

Save the changes and execute the code:

```bash
cargo run
```

Visit `127.0.0.1:8000` in your browser and you should see `lykin` written on the page.

### Initialise a Connection to the Sbot

Now we can write our first Scuttlebutt-related code. Begin by adding the `golgi` dependency to your `Cargo.toml` file:

`golgi = { git = "https://git.coopcloud.tech/golgi-ssb/golgi.git" }`

`golgi` is an RPC client library that allows us to interact with a running sbot.

We're going to write a function to define the connection parameters needed to communicate successfully with our locally-running sbot instance. This includes the IP and port on which the `go-sbot` is listening, as well as the location of the keystore being used by the `go-sbot` (ie. where the `secret` file lives...the file which contains the public-private keypair used by the sbot):

`src/main.rs`

```rust
use std::env;

use golgi::{sbot::Keystore, Sbot};

async fn init_sbot() -> Result<Sbot, String> {
    let go_sbot_port = env::var("GO_SBOT_PORT").unwrap_or_else(|_| "8021".to_string());

    let keystore = Keystore::GoSbot;
    let ip_port = Some(format!("127.0.0.1:{}", go_sbot_port));
    let net_id = None;

    Sbot::init(keystore, ip_port, net_id)
        .await
        .map_err(|e| e.to_string())
}
```

As you can see in the code snippet above, we're checking the `GO_SBOT_PORT` environment variable for the port definition of the sbot. This will allow you to set a custom port when running lykin (e.g. `GO_SBOT_PORT=8030 cargo run`). The code defaults to `8021` if the environment variable is unset.

We define the default keystore for GoSbot (`/$HOME/.ssb-go`) using the `GoSbot` variant of the `Keystore` `enum` and set `net_id` as `None`. When initialised with a `None` value for `net_id`, `golgi` uses the standard network key (aka. caps key) for the Scuttleverse. This allows us to interact and share messages with peers on the main network.

`Sbot::init()` returns an instance of the `Sbot` `struct` which implements all the methods we require to communicate with the `go-sbot`. 

### Call the `whoami` RPC Method

Let's make our first RPC call to help ensure the `go-sbot` is running correctly. We'll call the `whoami` method which should return the public key (SSB ID) of our local sbot.

```rust
async fn whoami() -> Result<String, String> {
    let mut sbot = init_sbot().await?;
    sbot.whoami().await.map_err(|e| e.to_string())
}
```

The code snippet is quite simple: we initialise a connection to the sbot using the function we defined previously and then invoke the `.whoami()` method on the sbot instance, mapping the error type (`GolgiError`) to a `String` for the sake of simplicity. The `Ok` variant of the `Result` type will contain the public key.

### Display the `whoami` ID

We can now update the code of the `home()` route to include the output of the `whoami()` function call, using a simple `match` statement for error handling and reporting:

```rust
#[get("/")]
async fn home() -> String {
    match whoami().await {
        Ok(id) => id,
        Err(e) => format!("whoami call failed: {}", e),
    }
}
```

Running the code and visiting `127.0.0.1:8000` in the browser should show the public key of the local sbot, provided it's running and functioning correctly, otherwise an error message will be displayed.

### Complete Code

All together, the code from this installment of the tutorial looks like this:

```rust
use std::env;

use golgi::{sbot::Keystore, Sbot};
use rocket::{get, launch, routes};

async fn init_sbot() -> Result<Sbot, String> {
    let go_sbot_port = env::var("GO_SBOT_PORT").unwrap_or_else(|_| "8021".to_string());

    let keystore = Keystore::GoSbot;
    let ip_port = Some(format!("127.0.0.1:{}", go_sbot_port));
    let net_id = None;

    Sbot::init(keystore, ip_port, net_id)
        .await
        .map_err(|e| e.to_string())
}

async fn whoami() -> Result<String, String> {
    let mut sbot = init_sbot().await?;
    sbot.whoami().await.map_err(|e| e.to_string())
}

#[get("/")]
async fn home() -> String {
    match whoami().await {
        Ok(id) => id,
        Err(e) => format!("whoami call failed: {}", e),
    }
}

#[launch]
async fn rocket() -> _ {
    rocket::build().mount("/", routes![home])
}
```

### Conclusion

That's all for the first part of this tutorial series. We installed and configured go-sbot, wrote a simple web server and made our first RPC call to the sbot. Not bad for 29 lines of code! In the next installment we'll setup the basic scaffolding for subscribing to Scuttlebutt peers.

## Funding

This work has been funded by a Scuttlebutt Community Grant.
