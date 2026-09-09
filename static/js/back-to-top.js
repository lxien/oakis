(function () {
  var THRESHOLD = 480;
  var btn = document.querySelector("[data-back-to-top]");
  if (!btn) return;

  var reduceMotion = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
  var ticking = false;
  var visible = false;

  function setVisible(next) {
    if (next === visible) return;
    visible = next;
    btn.classList.toggle("is-visible", visible);
    btn.setAttribute("aria-hidden", visible ? "false" : "true");
  }

  function update() {
    ticking = false;
    setVisible(window.scrollY > THRESHOLD);
  }

  function onScroll() {
    if (ticking) return;
    ticking = true;
    window.requestAnimationFrame(update);
  }

  btn.addEventListener("click", function () {
    window.scrollTo({
      top: 0,
      behavior: reduceMotion ? "auto" : "smooth",
    });
  });

  window.addEventListener("scroll", onScroll, { passive: true });
  update();
})();
