# lykin tutorial

## Part 6: UI Layout and Peers List

### Introduction

Up to this point in the series we've been primarily focused on backend development; we've created a webserver, setup a key-value store, written functions for interacting with an sbot instance and made a task loop to run background processes. It's time to focus on the UI of our application.

Today we'll write Tera HTML templates and create the layout of our user interface using CSS. We will then begin to populate the templates with data from our key-value store, such as the list of peers we're subscribed to. This is an exciting phase in the development of our application. Let's begin!

### Outline

Here's what we'll tackle in this sixth part of the series:

 - Define layout shape
 - Download stylesheet and icons
 - Mount the fileserver
 - Create layout in base template
 - Create templates
	 - Navigation bar
	 - Peer list
	 - Post list
	 - Post content
 - Populate peer list with data

### Define Layout Shape

Before getting started with code, it might be helpful to know the shape of the layout we'll be building in this installment.

The layout is composed of a topbar for navigation, a peers column on the left and a column of posts and post content on the right. Here's a diagram to illustrate the basic shape:

```text
┌───────────────────────────────────────────────────┐
│ Navigation                                        │
├──────────────┬────────────────────────────────────┤
│ Peer List    │ Post List                          │
│              │                                    │
│              │                                    │
│              ├────────────────────────────────────┤
│              │ Post Content                       │
│              │                                    │
│              │                                    │
│              │                                    │
│              │                                    │
└──────────────┴────────────────────────────────────┘
```

### Download Icons and Stylesheet

We are going to use CSS grid to create the layout of our user interface. I am not going to deal with CSS in-detail in this tutorial so you may want to refer to [A Complete Guide to Grid](https://css-tricks.com/snippets/css/complete-guide-grid/), authored by Chris House on CSS-Tricks, to fill any gaps in your understanding. We will simply download the stylesheet and icons so that we can focus on the rest of the application.

We begin by creating a `static` directory in the root directory of our application. Next, we create subdirectories named `css` and `icons` inside the static directory. Like so:

```text
.
├── static
│   ├── css
│   └── icons
```

Now we can download the assets from the [lykin repo](https://git.coopcloud.tech/glyph/lykin):

```bash
# Ensure you are calling these commands from the root directory.
# You can download the files manually if you do not have wget.
# ...
# Download the CSS stylesheet:
wget -O static/css/lykin.css https://git.coopcloud.tech/glyph/lykin/raw/branch/main/static/css/lykin.css
# Move into the icons subdirectory:
cd static/icons
# Download the icons:
wget https://git.coopcloud.tech/glyph/lykin/raw/branch/main/static/icons/delete_post.png
wget https://git.coopcloud.tech/glyph/lykin/raw/branch/main/static/icons/download.png
wget https://git.coopcloud.tech/glyph/lykin/raw/branch/main/static/icons/read_post.png
wget https://git.coopcloud.tech/glyph/lykin/raw/branch/main/static/icons/unread_post.png
# Move back to the root directory:
cd ../..
```

**Note:** The icons we're using were created by [Kiranshastry](https://www.flaticon.com/authors/kiranshastry) and can be found on Flaticon.

### Mount the Fileserver

In order to be able to serve the CSS and icons, we need to mount a fileserver to our Rocket application and provide the path to the assets:

`src/main.rs`

```rust
use rocket::fs::{FileServer, relative};

#[launch]
async fn rocket() -> _ {
    // ...

    rocket::build()
        .manage(db)
        .manage(tx)
        .attach(Template::fairing())
        .mount("/", routes![home, subscribe_form, unsubscribe_form])
        // Mount the fileserver and set a relative path with `static` as root.
        .mount("/", FileServer::from(relative!("static")))
        .attach(AdHoc::on_shutdown("cancel task loop", |_| {
            Box::pin(async move {
                tx_clone.send(Task::Cancel).await.unwrap();
            })
        }))
}
```

### Create Layout in Base Template

Now that the assets and fileserver are in place, we can turn our attention to the templates. Let's begin by modifying the base HTML template we wrote previously. In it, we're going to create a grid container and include (`import`) the templates representing each section of the layout. We will then create the templates for each section of the layout.

`templates/base.html.tera`

```html
<!DOCTYPE html>
<html lang="en">
  <head>
    <meta charset="utf-8">
    <title>lykin</title>
    <meta name="description" content="lykin: an SSB tutorial application">
    <meta name="author" content="glyph">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <link rel="stylesheet" href="/css/lykin.css">
  </head>
  <body class="container">
    <h1>
      <a href="/">lykin</a>
    </h1>
    </a>
    <div class="grid-container">
      {% include "topbar" %}
      {% include "peer_list" %}
      {% include "post_list" %}
      {% include "post_content" %}
    </div>
  </body>
</html>
```

### Create Navigation Bar Template

With the base layout in place, we can begin to populate the constituent templates. The navigation / topbar consists of a row of four icons followed by a form for subscribing to and unsubscribing from peers. Clicking on each icon will eventually perform an action: download the latest posts, mark a post as 'read', mark a post as 'unread' and delete a post. We'll create the routes and handlers for those actions later in the series. For now, it's enough to have the icons without any associated actions.

You're invited to take a peek at the stylesheet (`lykin.css`) if you're curious about any of the classes used in the template, such as `disabled` or `flex-container`.

`templates/topbar.html.tera`

```html
<div class="nav">
  <div class="flex-container">
    <a class="disabled icon" title="Download latest posts">
      <img src="/icons/download.png">
    </a>
    <a class="disabled icon" title="Mark as read">
      <img src="/icons/read_post.png">
    </a>
    <a class="disabled icon" title="Mark as unread">
      <img src="/icons/unread_post.png">
    </a>
    <a class="disabled icon" title="Delete post">
      <img src="/icons/delete_post.png">
    </a>
    <form class="flex-container" action="/subscribe" method="post">
      <label for="public_key">Public Key</label>
      <input type="text" id="public_key" name="public_key" maxlength=53>
      <input type="submit" value="Subscribe">
      <input type="submit" value="Unsubscribe" formaction="/unsubscribe">
    </form>
    {% if flash and flash.kind == "error" %}
    <p class="flash-message">[ {{ flash.message }} ]</p>
    {% endif %}
  </div>
</div>
```

The `{% ... %}` syntax in the template code is Tera syntax (inspired by Jinja2 and Django templates). Consult the [documentation](https://tera.netlify.app/docs/) if you wish to know more. We will add similar control-flow syntax later in the tutorial series to selectively set the `href` tags of the anchor elements and to enable or disable the navigation elements.

### Create Peer List Template

This one couldn't be much simpler. We define a `div` element for our list of peers and populate an unordered list. We first try to display the name of the peer and fallback to the public key if the `name` string is empty. Each peer in this template corresponds with an instance of the `Peer` struct defined in our `src/db.rs` file, hence the `name` and `public_key` fields.

`templates/peer_list.html.tera`

```html
<div class="peers">
  <ul>
  {% for peer in peers -%} 
    <li>
    {% if peer.name %}
      {{ peer.name }}
    {% else %}
      {{ peer.public_key }}
    {% endif %}
    </li>
  {%- endfor %}
  </ul>
</div>
```

### Create Post List Template

Now we'll write another simple `for` loop to display a list of posts. Eventually we'll update this template to display the subject of each post authored by the selected peer. Clicking on a peer in the peer list will serve as the trigger to update the selected peer variable, allowing us to define whose posts we should be displaying in this list.

`templates/post_list.html.tera`

```html
<div class="posts">
  {% if posts %}
  <ul>
  {% for post in posts -%} 
    Subject placeholder
  {%- endfor %}
  </ul>
  {% endif %}
</div>  
```

### Create Post Content Template

Finally, we'll write the template to display the content of a selected post.

`templates/post_content.html.tera`

```html
<div class="content">
{% if post %}
  {{ post.text }}
{% endif %}
</div>
```

### Populate Peer List with Data

If we run our application at this point and visit `localhost:8000` in a browser we receive a `500: Internal Server Error`. The output in the Rocket application logs points to the problem:

```text
>> Error rendering Tera template 'base'.
>> Failed to render 'base'
>> Variable `peers` not found in context while rendering 'peer_list'
>> Template 'base' failed to render.
>> Outcome: Failure
```

The `peer_list.html.tera` template expects a `peers` variable which has not been provided. In other words, the template has not been provided with the context it requires to render. What we need to do is revisit our `home` route handler and provide the context by querying our key-value store for a list of peers.

`src/routes.rs`

```rust
#[get("/")]
pub async fn home(db: &State<Database>, flash: Option<FlashMessage<'_>>) -> Template {
    // Retrieve the list of peers to whom we subscribe.
    let peers = db.get_peers();
    
    // Render the template with `peers` and `flash`.
    Template::render("base", context! { peers: peers, flash: flash })
}
```

Great, the template will now be hydrated with the data it expects. There's just one more problem: the `db.get_peers()` method doesn't exist yet. Let's write it now:

`src/db.rs`

```rust
impl Database {
    // ...

    // Get a list of all peers in the peer tree. The byte value for each
    // peer entry is deserialized from bincode into an instance of the Peer
    // struct.
    pub fn get_peers(&self) -> Vec<Peer> {
        debug!("Retrieving data for all peers in the 'peers' database tree");
        // Define an empty vector to store the list of peers.
        let mut peers = Vec::new();

        self.peer_tree
            .iter()
            .map(|peer| peer.unwrap())
            .for_each(|peer| {
                debug!(
                    "Deserializing peer data for {} from bincode",
                    String::from_utf8_lossy(&peer.0).into_owned()
                );
                // Add a peer to the peers vector.
                peers.push(bincode::deserialize(&peer.1).unwrap())
            });

        peers
    }
}
```

The above method is very similar to the `get_peer` method we define previously. However, instead of retrieving a specific peer from the peer database tree, we iterate over all key-value pairs in the tree and push the deserialized value to a vector.

Run the application, visit `localhost:8000` in your browser and you should see a beautiful, colourful layout! After ensuring your instance of go-sbot is running, try to subscribe and unsubscribe to some peers to test things out. Feel free to play around with the styles in `static/css/lykin.css` if you wish to change the colours or other aspects of the design.

### Conclusion

In this installment we took strides in improving the visual aspect of our application. We defined a layout using CSS and HTML templates, added a fileserver to serve assets, updated our `home` route handler to provide the required context data to our templates and added a `get_peers()` method to the database.

Our application has come a long way. We can now subscribe and unsubscribe to the root posts of our peers and display a list of subscribed peers in a neat user interface.

In the next installment we will return to the database and Scuttlebutt-related code in our application, adding the ability to retrieve only the latest posts for each of our peers from the sbot. This will give us an efficient way of keeping our application up to date with the latest happenings in the Scuttleverse. In doing so, we will add a means of tracking the latest sequence number of each of the peers we subscribe to.

## Funding

This work has been funded by a Scuttlebutt Community Grant.

## Contributions

I would love to continue working on the Rust Scuttlebutt ecosystem, writing code and documentation, but I need your help. Please consider contributing to [my Liberapay account](https://liberapay.com/glyph) to support me in my coding and cultivation efforts.
