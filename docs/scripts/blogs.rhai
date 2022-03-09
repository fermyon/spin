// This function lists all of the blog posts, subject to the constraints.
// It assumes you have a `content/blog` subdirectory, and every file in there
// is treated like a blog post.
//
// It returns an array of objects of the form:
//  [
//    #{ uri: "path/to/page", page: PageObject }
// ]

// Param 1 should be `site.pages`
let pages = params[0];

// Loop through them and return all of the page objects that are in
// the blog path.

let blog_pages = [];

// Get each blog post, assigning it to {path: object}.
let keys = pages.keys();
for item in keys {
    if item.index_of("/content/blog/") == 0 {
        // Remove /content and .md
        let path = item.sub_string(8);
        path = path.sub_string(0, path.index_of(".md"));
        blog_pages.push(#{
            uri: path,
            page: pages[item],
        });
        //blog_pages[path] = pages[item];
    }
   
}
// Newest to oldest, assuming you put the date in the URI
blog_pages.reverse();
blog_pages