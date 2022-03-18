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



const prefersDarkScheme = window.matchMedia('(prefers-color-scheme: dark)');
if (prefersDarkScheme.matches) {
  document.body.classList.add('dark-theme');
} else {
  document.body.classList.remove('dark-theme');
}

// toggle light and dark mode
const btn = document.querySelector(".dark-mode");
const currentTheme = localStorage.getItem("theme");

if (currentTheme == "dark") {
  document.body.classList.add("dark-theme");
}

btn.addEventListener("click", function() {
  document.body.classList.toggle("dark-theme");
  
  let theme = "light";
  if (document.body.classList.contains("dark-theme")) {
    theme = "dark";
  }
  // save theme to localstorage
  localStorage.setItem("theme", theme);
});


document.addEventListener("DOMContentLoaded", function(){
  // init after dom

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
