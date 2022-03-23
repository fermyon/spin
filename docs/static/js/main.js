var header = new Headroom(document.querySelector("#topbar"), {
    tolerance: 5,
    offset : 80
});

var blogAd = new Headroom(document.querySelector("#blogSlogan"), {
  tolerance: 5,
  offset : 300
});

document.querySelectorAll('.modal-button').forEach(function(el) {
  el.addEventListener('click', function() {
    var target = document.querySelector(el.getAttribute('data-target'));
    
    target.classList.add('is-active');
    target.querySelector('.modal-close').addEventListener('click',   function() {
        target.classList.remove('is-active');
    });
    target.querySelector('.modal-background').addEventListener('click',   function() {
        target.classList.remove('is-active');
     });
  });
});


const systemTheme = window.matchMedia("(prefers-color-scheme: dark)").matches
  ? "dark"
  : "light";

// The default theme is the system theme, unless the user has
// explicitly overriden it.
var savedTheme = localStorage.getItem("theme") || systemTheme;
setTheme(savedTheme);

const btn = document.querySelector(".dark-mode");
btn.addEventListener("click", () => {
  if(savedTheme === "dark") {
    setTheme("light");
  } else if(savedTheme === "light") {
    setTheme("dark");
  }
});

// Change the website theme when the system theme changes.
window
  .matchMedia("(prefers-color-scheme: dark)")
  .addEventListener("change", (event) => {
    if (event.matches) {
      setTheme("dark");
    } else {
      setTheme("light");
    }
  });

// Change the website theme when a different tab changed it.
window.onstorage = () => {
  // When local storage changes, update the theme.
  setTheme(localStorage.getItem("theme"));
};


function setTheme(mode) {
  localStorage.setItem("theme", mode);
  savedTheme = mode;
  if (mode === "dark") {
    document.body.classList.add('dark-theme');
  } else if (mode === "light") {
    document.body.classList.remove('dark-theme');
  }
}

document.addEventListener("DOMContentLoaded", function(){
  // Initialize after the DOM.
  (function () {
    var burger = document.querySelector('.burger');
    var menu = document.querySelector('#' + burger.dataset.target);
    burger.addEventListener('click', function () {
        burger.classList.toggle('is-active');
        menu.classList.toggle('is-active');
    });
  })();

  header.init();
  hljs.highlightAll();
});

if (document.body.contains(document.getElementById('blogSlogan'))){
  blogAd.init();
};
