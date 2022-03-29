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
