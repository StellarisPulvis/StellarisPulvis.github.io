(function () {
  var index = [];
  var input = document.getElementById("search-input");
  var results = document.getElementById("search-results");

  if (!input || !results) return;

  fetch("/search.json")
    .then(function (r) { return r.json(); })
    .then(function (data) { index = data; })
    .catch(function () {});

  var timer;
  input.addEventListener("input", function () {
    clearTimeout(timer);
    timer = setTimeout(search, 200);
  });

  input.addEventListener("focus", function () {
    if (results.querySelector(".search-result-item")) {
      results.classList.add("active");
    }
  });

  document.addEventListener("click", function (e) {
    if (!e.target.closest(".search-box")) {
      results.classList.remove("active");
    }
  });

  document.addEventListener("keydown", function (e) {
    if (e.key === "Escape") {
      results.classList.remove("active");
      input.blur();
    }
  });

  function search() {
    var q = input.value.trim().toLowerCase();
    if (q.length < 1) {
      results.classList.remove("active");
      return;
    }

    var hits = [];
    for (var i = 0; i < index.length; i++) {
      var p = index[i];
      if (
        p.title.toLowerCase().indexOf(q) !== -1 ||
        p.summary.toLowerCase().indexOf(q) !== -1 ||
        p.content.toLowerCase().indexOf(q) !== -1 ||
        p.tags.some(function (t) { return t.toLowerCase().indexOf(q) !== -1; })
      ) {
        hits.push(p);
        if (hits.length >= 10) break;
      }
    }

    if (hits.length === 0) {
      results.innerHTML = '<div class="search-result-empty">未找到相关文章</div>';
      results.classList.add("active");
      return;
    }

    var html = "";
    for (var i = 0; i < hits.length; i++) {
      var h = hits[i];
      html +=
        '<a href="/posts/' + h.slug + '/" class="search-result-item">' +
          '<span class="search-result-title">' + highlight(h.title, q) + "</span>" +
          '<span class="search-result-meta">' + h.date + "</span>" +
        "</a>";
    }
    results.innerHTML = html;
    results.classList.add("active");
  }

  function highlight(text, query) {
    var idx = text.toLowerCase().indexOf(query);
    if (idx === -1) return escapeHtml(text);
    return (
      escapeHtml(text.slice(0, idx)) +
      "<mark>" +
      escapeHtml(text.slice(idx, idx + query.length)) +
      "</mark>" +
      escapeHtml(text.slice(idx + query.length))
    );
  }

  function escapeHtml(s) {
    var div = document.createElement("div");
    div.appendChild(document.createTextNode(s));
    return div.innerHTML;
  }
})();
