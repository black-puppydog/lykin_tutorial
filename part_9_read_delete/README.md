# lykin tutorial

## Part 9: Read, Unread and Delete

### Introduction

In the last installment we implemented the functionality necessary to display posts in the web interface of our application, bringing it close to completion. Today we're going to add the finishing touches. We'll count and display the number of unread posts for each peer we subscribe to and add the ability to mark a post as read or unread. To finish things off, we'll allow the user to delete individual posts via the web interface. This installment will touch the database, route handlers and templates. Let's get started!

### Outline

 - Count unread posts
 - Display unread post count
 - Mark a post as read
 - Mark a post as unread
 - Remove a post from the database
 - Update navigation template 

### Count Unread Posts

We'll begin by implementing a database method that takes a public key as input and returns the total number of unread posts authored by that peer in our key-value store. The logic is fairly simple: retrieve all posts by the peer, iterate through them and increment a counter every time an unread post is encountered.

`src/db.rs`

```rust
impl Database {
    // ...

    // Sum the total number of unread posts for the peer represented by the
    // given public key.
    pub fn get_unread_post_count(&self, public_key: &str) -> u16 {
        debug!(
            "Counting total number of unread posts for peer {}",
            &public_key
        );

        let mut unread_post_counter = 0;

        self.post_tree
            .scan_prefix(public_key.as_bytes())
            .map(|post| post.unwrap())
            .for_each(|post| {
                debug!(
                    "Deserializing post data for {} from bincode",
                    String::from_utf8_lossy(&post.0).into_owned()
                );
                let deserialized_post: Post = bincode::deserialize(&post.1).unwrap();
                if !deserialized_post.read {
                    unread_post_counter += 1
                }
            });

        unread_post_counter
    }
}
```

### Display Unread Post Count

Now we can update the peer list of our web application to show the number of unread posts next to the name of each peer. This behaviour is similar to what you might see in an email client, where the number of unread messages is displayed alongside the name of each folder in your mailbox.

First we need to update the `home`, `posts` and `post` route handlers of our application to retrieve the unread post counts and pass them into the template as context variables.

`src/routes.rs`

```rust
#[get("/")]
pub async fn home(db: &State<Database>, flash: Option<FlashMessage<'_>>) -> Template {
    let peers = db.get_peers();
    let mut peers_unread = Vec::new();
    for peer in peers {
        // Count the total unread posts for the given peer.
        let unread_count = db.get_unread_post_count(&peer.public_key);
        // Push a tuple of the peer data and peer unread post count
        // to the `peers_unread` vector.
        peers_unread.push((peer, unread_count.to_string()));
    }

    Template::render("base", context! { peers: &peers_unread, flash: flash })
}

#[get("/posts/<public_key>")]
pub async fn posts(db: &State<Database>, public_key: &str) -> Template {
    let peers = db.get_peers();
    let mut peers_unread = Vec::new();
    for peer in peers {
        let unread_count = db.get_unread_post_count(&peer.public_key);
        peers_unread.push((peer, unread_count.to_string()));
    }

    let posts = db.get_posts(public_key).unwrap();

    let context = context! {
        selected_peer: &public_key,
        peers: &peers_unread,
        posts: &posts
    };

    Template::render("base", context)
}

#[get("/posts/<public_key>/<msg_id>")]
pub async fn post(db: &State<Database>, public_key: &str, msg_id: &str) -> Template {
    let peers = db.get_peers();
    let mut peers_unread = Vec::new();
    for peer in peers {
        let unread_count = db.get_unread_post_count(&peer.public_key);
        peers_unread.push((peer, unread_count.to_string()));
    }

    let posts = db.get_posts(public_key).unwrap();
    let post = db.get_post(public_key, msg_id).unwrap();

    let context = context! {
        peers: &peers_unread,
        selected_peer: &public_key,
        selected_post: &msg_id,
        posts: &posts,
        post: &post
    };

    Template::render("base", context)
}
```

You'll notice that the main change in the code above, when compared to the code from previous installments, is the `peers_unread` vector and populating loop:

```rust
let mut peers_unread = Vec::new();
for peer in peers {
    let unread_count = db.get_unread_post_count(&peer.public_key);
    peers_unread.push((peer, unread_count.to_string()));
}
```

The other difference is that we now pass `context! { peers: &peers_unread, ... }` instead of `context! { peers: &peers, ... }`.

Now we need to update the peers list template to utilise the newly-provided unread post count data.

`templates/peer_list.html.tera`

```html
<div class="peers">
  <ul>
  {% for peer in peers -%} 
    <li>
      <a class="flex-container" href="/posts/{{ peer.0.public_key | urlencode_strict }}">
        <code{% if selected_peer and peer.0.public_key == selected_peer %} style="font-weight: bold;"{% endif %}>
        {% if peer.0.name %}
          {{ peer.0.name }}
        {% else %}
          {{ peer.0.public_key }}
        {% endif %}
        </code>
        {% if peer.1 != "0" %}<p>{{ peer.1 }}</p>{% endif %}
      </a>
    </li>
  {%- endfor %}
  </ul>
</div>
```

Since the `peers` context variable is a tuple of `(peer, unread_post_count)`, we use tuple indexing when referencing the values (ie. `peer.0` for the `peer` data and `peer.1` for the `unread_post_count` data).

Run the application (`cargo run`) and you should now see the unread post count displayed next to the name of each peer in the peers list.

### Mark a Post as Read

Much like an email client, we want to be able to mark individual posts as either `read` or `unread`. We already have icons in place in our `topbar.html.tera` template with which to perform these actions. Now we need to write a route handler to mark a particular post as read.

`src/routes.rs`

```rust
#[get("/posts/<public_key>/<msg_id>/read")]
pub async fn mark_post_read(db: &State<Database>, public_key: &str, msg_id: &str) -> Redirect {
    // Retrieve the post from the database using the public key and msg_id
    // from the URL.
    if let Ok(Some(mut post)) = db.get_post(public_key, msg_id) {
        // Mark the post as read.
        post.read = true;
        // Reinsert the modified post into the database.
        db.add_post(public_key, post).unwrap();
    } else {
        warn!(
            "Failed to find post {} authored by {} in 'posts' database tree",
            msg_id, public_key
        )
    }

    Redirect::to(uri!(post(public_key, msg_id)))
}
```

### Mark a Post as Unread

We can now write the equivalent route handler for marking a post as unread.

`src/routes.rs`

```rust
#[get("/posts/<public_key>/<msg_id>/unread")]
pub async fn mark_post_unread(db: &State<Database>, public_key: &str, msg_id: &str) -> Redirect {
    if let Ok(Some(mut post)) = db.get_post(public_key, msg_id) {
        post.read = false;
        db.add_post(public_key, post).unwrap();
    } else {
        warn!(
            "Failed to find post {} authored by {} in 'posts' database tree",
            msg_id, public_key
        )
    }

    Redirect::to(uri!(post(public_key, msg_id)))
}
```

We still need to mount these routes to our Rocket application in `src/main.rs` and update the logic in our navigation template to wrap the `read` and `unread` icons in anchor elements with the correct URLs. We'll take care of that once we've written the backend code to remove a post from the database.

### Remove a Post From the Database

We already have a `remove_peer()` method in our database which is used when we unsubscribe from a peer. Now we'll write the equivalent method for a post.

Remember that we are deleting the post from our key-value store, _not_ the sbot database! The message will remain in the log kept by the sbot.

`src/db.rs`

```rust
impl Database {
    // ...

    // Remove a single post from the post tree, authored by the given public
    // key and defined by the given message ID.
    pub fn remove_post(&self, public_key: &str, msg_id: &str) -> Result<()> {
        let post_key = format!("{}_{}", public_key, msg_id);
        debug!("Removing post {} from 'posts' database tree", &post_key);

        // .remove() would ordinarily return the value of the deleted entry
        // as an Option, returning None if the post_key was not found.
        // We don't care about the value of the deleted entry so we simply
        // map the Option to ().
        self.post_tree.remove(post_key.as_bytes()).map(|_| ())
    }
}
```

Now we need to write a route handler to respond to a delete request. Much like the route handlers for marking a post as read and unread, we include the public key of the peer and the message ID of the post in the URL. We then use those values when invoking the `remove_post()` method.

`src/routes.rs`

```rust
#[get("/posts/<public_key>/<msg_id>/delete")]
pub async fn delete_post(db: &State<Database>, public_key: &str, msg_id: &str) -> Redirect {
    // Delete the post from the database.
    match db.remove_post(public_key, msg_id) {
        Ok(_) => info!(
            "Removed post {} by {} from 'posts' database tree",
            msg_id, public_key
        ),
        Err(e) => warn!(
            "Failed to remove post {} by {} from 'posts' database tree: {}",
            msg_id, public_key, e
        ),
    }

    Redirect::to(uri!(posts(public_key)))
}
```

The three routes we've created so far can now be mounted to the Rocket application.

`src/main.rs`

```rust
#[launch]
async fn rocket() -> _ {
    // ...

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
                posts,
                mark_post_read,
                mark_post_unread,
                delete_post
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

### Update Navigation Template

With the backend functionality in place, we can now update the navigation template to ensure the correct URLs are set for the 'mark as read, 'mark as unread' and 'delete post' elements. We will only enable those elements when a post is selected, signalling to the user when action is possible.

`templates/topbar.html.tera`

```html
<div class="nav">
  <div class="flex-container">
    <a href="/posts/download_latest" title="Download latest posts">
      <img src="/icons/download.png">
    </a>
    {% if post_is_selected %}
      {% set selected_peer_encoded = selected_peer | urlencode_strict %}
      {% if post.read %}
        {% set mark_unread_url = "/posts/" ~ selected_peer_encoded ~ "/" ~ selected_post_encoded ~ "/unread" %}
        <a class="disabled icon" title="Mark as read">
          <img src="/icons/read_post.png">
        </a>
        <a href={{ mark_unread_url }} class="icon" title="Mark as unread">
          <img src="/icons/unread_post.png">
        </a>
      {% else %}
        {% set mark_read_url = "/posts/" ~ selected_peer_encoded ~ "/" ~ selected_post_encoded ~ "/read" %}
        <a href={{ mark_read_url }} class="icon" title="Mark as read">
          <img src="/icons/read_post.png">
        </a>
        <a class="disabled icon" title="Mark as unread">
          <img src="/icons/unread_post.png">
        </a>
      {% endif %}
      {% set delete_post_url = "/posts/" ~ selected_peer_encoded ~ "/" ~ selected_post_encoded ~ "/delete" %}
      <a href={{ delete_post_url }} class="icon" title="Delete post">
        <img src="/icons/delete_post.png">
      </a>
    {% else %}
      <a class="disabled icon" title="Mark as read">
        <img src="/icons/read_post.png">
      </a>
      <a class="disabled icon" title="Mark as unread">
        <img src="/icons/unread_post.png">
      </a>
      <a class="disabled icon" title="Delete post">
        <img src="/icons/delete_post.png">
      </a>
    {% endif %}
    <form class="flex-container" action="/subscribe" method="post">
      <label for="public_key">Public Key</label>
      {% if selected_peer %}
        <input type="text" id="public_key" name="public_key" maxlength=53 value={{ selected_peer }}>
      {% else %}
        <input type="text" id="public_key" name="public_key" maxlength=53>
      {% endif %}
      <input type="submit" value="Subscribe">
      <input type="submit" value="Unsubscribe" formaction="/unsubscribe">
    </form>
    {% if flash and flash.kind == "error" %}
    <p class="flash-message">[ {{ flash.message }} ]</p>
    {% endif %}
  </div>
</div>
```

Now we have one more template-related change to make. We need to check the read / unread value of each post in the post list and render the text in bold if it is unread.

`templates/post_list.html.tera`

```html
<div class="posts">
  {% if posts %}
  <ul>
  {% for post in posts -%} 
    <li{% if selected_post and post.key == selected_post %} class="selected"{% endif %}>
      <a class="flex-container"{% if not post.read %} style="font-weight: bold;"{% endif %} href="/posts/{{ selected_peer | urlencode_strict }}/{{ post.key | urlencode_strict }}">
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

Notice the `{% if not post.read %}` syntax in the code above; that is where we selectively bold the line item for unread posts.

Everything should now be in place. Run the application (`cargo run`) and see how the user interface changes as you mark posts as read / unread and delete posts.

### Conclusion

We did it! We wrote a Scuttlebutt client application in Rust. Congratulations on making it this far in the tutorial! I really hope you've learned something through this experience and that you're feeling inspired to write your own applications or modify this one.

In the next installment of the series I'll share some ideas for improving and extending the application.

## Funding

This work has been funded by a Scuttlebutt Community Grant.

## Contributions

I would love to continue working on the Rust Scuttlebutt ecosystem, writing code and documentation, but I need your help. Please consider contributing to [my Liberapay account](https://liberapay.com/glyph) to support me in my coding and cultivation efforts.
