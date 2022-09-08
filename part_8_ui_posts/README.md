# lykin tutorial

## Part 8: Post List and Post Content

### Introduction

In the last tutorial installment we added the ability to sync the latest posts and names for each peer we subscribe to. The goal of this installment is to update the web interface to display a list of posts when a peer is selected from the peer list and to display the text of a post when one is selected. In order to achieve this, we'll need to add methods to the key-value database to retrieve a post or a batch of posts. We'll also need to add a number of endpoints to our webserver and update the control-flow logic in our templates.

### Outline

 - Get a post from the database
 - Get a batch of posts from the database
 - Add a posts route handler
 - Add a post route handler
 - Mount the post route handlers
 - Update templates
   - Peer list
   - Post list
   - Post content

### Get a Post From the Database

We previously wrote database methods for adding a single post and a batch of posts to the key-value store. Now we need to write methods for retrieving that data. Let's start with a method to retrieve a single post. The method will take a public key and a message ID (aka. message reference or sigil link) as parameters, concatenate those values to create a `post_key` and then attempt to get the value of that key from the database.

`src/db.rs`

```rust
impl Database {
    // ...

    // Get a single post from the post tree, authored by the given public key
    // and defined by the given message ID. The byte value for the matching
    // entry, if found, is deserialized from bincode into an instance of the
    // Post struct.
    pub fn get_post(&self, public_key: &str, msg_id: &str) -> Result<Option<Post>> {
        let post_key = format!("{}_{}", public_key, msg_id);
        debug!(
            "Retrieving post data for {} from 'posts' database tree",
            &post_key
        );

        let post = self
            .post_tree
            .get(post_key.as_bytes())
            .unwrap()
            .map(|post| {
                debug!("Deserializing post data for {} from bincode", &post_key);
                bincode::deserialize(&post).unwrap()
            });

        Ok(post)
    }
}
```

### Get a Batch of Posts From the Database

The corresponding method for retrieving a batch of posts is very similar. We pass in the public key of the desired peer as a parameter and retrieve all posts with a key beginning with that public key (notice `scan_prefix()` in the code below). Once we've populated a vector with all the posts by the given public key, we sort the list according to the timestamp of each post. This will allow us to easily display the posts in descending chronological order in the web interface.

`src/routes.rs`

```rust
impl Database {
    // ...

    // Get a list of all posts in the post tree authored by the given public
    // key and sort them by timestamp in descending order. The byte value for
    // each matching entry is deserialized from bincode into an instance of
    // the Post struct.
    pub fn get_posts(&self, public_key: &str) -> Result<Vec<Post>> {
        debug!("Retrieving data for all posts in the 'posts' database tree");
        let mut posts = Vec::new();

        self.post_tree
            .scan_prefix(public_key.as_bytes())
            .map(|post| post.unwrap())
            .for_each(|post| {
                debug!(
                    "Deserializing post data for {} from bincode",
                    String::from_utf8_lossy(&post.0).into_owned()
                );
                posts.push(bincode::deserialize(&post.1).unwrap())
            });

        posts.sort_by(|a: &Post, b: &Post| b.timestamp.cmp(&a.timestamp));

        Ok(posts)
    }
}
```

### Add a Posts Route Handler

Imagine interacting with the lykin interface for a moment: we load the application and see a list of peers down the left-hand side; these are the peers we subscribe to. When we click on the name of one of the peers in the list, we want to see a list of posts authored by that peer - each one with a subject line and date. Then, when we click on one of the posts in the list, we want to see the content of that post.

Let's write an endpoint that will take a public key and render the user interface with a list of posts:

`src/routes.rs`

```rust
#[get("/posts/<public_key>")]
pub async fn posts(db: &State<Database>, public_key: &str) -> Template {
    // Fetch the list of peers we subscribe to.
    let peers = db.get_peers();

    // Fetch the posts for the given peer from the key-value database.
    let posts = db.get_posts(public_key).unwrap();

    // Define context data to be rendered in the template.
    let context = context! {
        peers: &peers,
        // This variable allows us to track which peer is currently selected
        // from within the template. We'll use this variable to render the
        // name of the selected peer in bold.
        selected_peer: &public_key,
        posts: &posts
    };

    Template::render("base", context)
}
```

There's not much to the code above: get the peers, get the posts, generate a template context from the data, render the template and return it to the caller.

### Add a Post Route Handler

Now we want to add an endpoint that will return a template populated with a list of peers, a list of posts _and_ the content of a specific post. This is the route handler that will be called when we click on a post in the post list.

`src/routes.rs`

```rust
#[get("/posts/<public_key>/<msg_id>")]
pub async fn post(db: &State<Database>, public_key: &str, msg_id: &str) -> Template {
    let peers = db.get_peers();
    let posts = db.get_posts(public_key).unwrap();
    let post = db.get_post(public_key, msg_id).unwrap();

    let context = context! {
        peers: &peers,
        selected_peer: &public_key,
        selected_post: &msg_id,
        posts: &posts,
        post: &post
    };

    Template::render("base", context)
}
```

The code above is almost identical to the code in the `posts` route handler, with the exception of the `msg_id` parameter, the `get_post` database call and the addition of `post` and `selected_post` to the template context. As with `selected_peer`, `selected_post` gives us a means of bolding the text of the selected post in the list of posts. If this is at all confusing, things should become clearer as we update the templates. Let's turn to that task now.

### Mount the Post Route Handlers

Let's register the `post` and `posts` route handlers by mounting them to our Rocket instance.

`src/main.rs`

```rust
#[launch]
async fn rocket() -> _ {
    // ...

    info!("Launching web server");
    rocket::build()
        .manage(db)
        .manage(tx)
        .mount(
            "/",
            routes![
                home,
                subscribe_form,
                unsubscribe_form,
                download_latest_posts,
                post,
                posts
            ],
        )
        .mount("/", FileServer::from(relative!("static")))
        .attach(Template::fairing())
        .attach(AdHoc::on_shutdown("cancel task loop", |_| {
            Box::pin(async move {
                tx_clone.send(Task::Cancel).await.unwrap();
            })
        }))
} 

```

### Update Peer List Template

We need to update the peer list template so that each name in the list is wrapped in an anchor element with an `href` tag value of `/posts/<public_key>`. The `selected_peer` context variable will come in handy here: we can use it to render the name of a peer as bold text if it is the currently selected peer.

`templates/peer_list.html.tera`

```html
<div class="peers">
  <ul>
  {% for peer in peers -%} 
    <li>
      <a class="flex-container" href="/posts/{{ peer.public_key | urlencode_strict }}">
        <code{% if selected_peer and peer.public_key == selected_peer %} style="font-weight: bold;"{% endif %}>
        {% if peer.name %}
          {{ peer.name }}
        {% else %}
          {{ peer.public_key }}
        {% endif %}
        </code>
      </a>
    </li>
  {%- endfor %}
  </ul>
</div>
```

Notice the `href` tag value above: `/posts/{{ peer.public_key | urlencode_strict }}`. `urlencode_strict` is a Tera filter that encodes all non-alphanumeric characters in a string including forward slashes (see [the docs](https://tera.netlify.app/docs/#urlencode-strict)).

We also check if the `selected_peer` context variable exists. If it does, and if it matches the value of the peer's public key, we render the name in bold text.

One other small improvement introduced here is selective rendering of the peer name. It's possible that our local key-value database may not contain a name for a peer we've subscribed to (for instance, if that peer is outside of our hops range or we simply haven't replicated any data for it yet). In the case that the peer's name is not known, we simply render the public key instead.

### Update Post List Template

When we wrote the initial post list template we simply printed `Subject placeholder` for each post in the list. Let's update that to display the subject and date for each post.

`templates/post_list.html.tera`

```html
<div class="posts">
  {% if posts %}
  <ul>
  {% for post in posts -%} 
    <li{% if selected_post and post.key == selected_post %} class="selected"{% endif %}>
      <a class="flex-container" href="/posts/{{ selected_peer | urlencode_strict }}/{{ post.key | urlencode_strict }}">
        <code>
        {% if post.subject %}
          {{ post.subject | trim_start_matches(pat='"') }}...
        {% else %}
          {{ post.text | trim_start_matches(pat='"') | trim_end_matches(pat='"') }}
        {% endif %}
        </code>
        <p>{{ post.date }}</p>
      </a>
    </li>
  {%- endfor %}
  </ul>
  {% endif %}
</div>
```

Here we see the `selected_post` context variable in action, in much the same way as the `selected_peer` variable was utilised in the peer list template. The `selected` class is applied to the selected post; this changes the background colour of the element to make it stand out from the rest of the posts.

The `href` tag value of each post in the list is constructed using the `selected_peer` and `post.key` values, both of which are strictly URL-encoded using a Tera filter. Then comes the code to display the post subject, if it exists, along with the post date. If `post.subject` is `None` then we display the `post.text` instead. This would occur if the post text contains less than 52 characters (the length defined for the subject text). Finally, the `post.date` is displayed as the last element in the list item.

### Update Post Content Template

Before wrapping up this installment of the series, we're going to make one small change to the post content template to remove the inverted commas which wrap the text of each post in our database. We'll also call the `trim` Tera filter to remove any leading and trailing whitespace characters:

`templates/post_content.html.tera`

```html
<div class="content">
{% if post %}
  {{ post.text | trim_start_matches(pat='"') | trim_end_matches(pat='"') | trim }}
{% endif %}
</div>
```

Now you can run the application with `cargo run` and test it out! Remember, you may need to wipe the key-value database if you encounter any `500` errors when navigating to the web interface in your browser.

### Conclusion

In this installment we brought our user-interface to life by added the ability to list posts and display post content. We wrote methods to retrieve posts from the key-value database and added route handlers to render post lists and post content. We also updated the HTML templates of our application to render the post-related data.

Most of the core logic of our application is now complete! In the next installment we'll add the ability to mark individual posts as read or unread and will display the total number of unread posts for each peer. We'll also add a means of deleting individual posts; all via the web interface.

## Funding

This work has been funded by a Scuttlebutt Community Grant.

## Contributions

I would love to continue working on the Rust Scuttlebutt ecosystem, writing code and documentation, but I need your help. Please consider contributing to [my Liberapay account](https://liberapay.com/glyph) to support me in my coding and cultivation efforts.
