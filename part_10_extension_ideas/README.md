# lykin tutorial

## Part 10: Extension Ideas and Conclusion

### Introduction

The application we've written is very basic and still quite rough around the edges. No doubt, there are things you would have done differently if you had written such an application. Rather than being a conclusion, I hope the end of this tutorial series will be the beginning of further development of lykin in many different directions. Please feel free to fork the repo and shape the application as you wish! Here I share a few ideas for improvements and extensions.

### Improvements

 - Error handling and reporting
   - Replace all instances of `unwrap()` with robust error handling to ensure the application never panics
   - Report useful information via the web interface when errors occur
	   - For example, when a connection attempt with the sbot fails
 - Interface interactivity
   - Write JSON endpoints to serve peer and post data
	 - Use JavaScript written with unobstructive principles to update the DOM without having to reload the entire page

### Extensions

 - Batch selection of posts
   - Delete or change read / unread status of many posts at once
 - Bookmarks
   - Bookmark a post(s) for later reading
   - View only the bookmarked posts for a selected peer
 - Interactions
   - React to a post (publish a `like` or other reaction)
   - Reply to a post (publish a message referencing the root post)
 - Threads
   - Display thread replies in addition to root posts
 - Formatting
   - Render the markdown of post text as HTML
	 - Display images where blob references are included in the post text

### Conclusion

I hope I've given you a few ideas for further development and that you feel excited and equipped to continue playing with this application. Make it your own! Modify the CSS, add new features, remove what you don't like...have fun.

If you do end up taking this futher, I'd love to hear about it! Please send me an email on `glyph@mycelial.technology` or on Scuttlebutt. You could also post an issue on the `lykin` git repo.

Please also contact me with any feedback you may have about this tutorial series.

## Funding

This work has been funded by a Scuttlebutt Community Grant.

## Contributions

I would love to continue working on the Rust Scuttlebutt ecosystem, writing code and documentation, but I need your help. Please consider contributing to [my Liberapay account](https://liberapay.com/glyph) to support me in my coding and cultivation efforts.
