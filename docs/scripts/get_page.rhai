// This gets a page path from its `md`  filename.
// `page /content/index.md site.pages` will return the path to the index file. 
let target = params[0];
let pages = params[1];

let page = #{};

for item in pages.keys() {
    if item == target {
        let path = item.sub_string(8);
        path = path.sub_string(0, path.index_of(".md"));
        page = #{
            uri: path,
            page: pages[item],
        };
    }
}

page;