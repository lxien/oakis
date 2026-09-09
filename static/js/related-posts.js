(function (global) {
    var DEBOUNCE_MS = 280;

    function esc(s) {
        return String(s == null ? "" : s)
            .replace(/&/g, "&amp;")
            .replace(/</g, "&lt;")
            .replace(/"/g, "&quot;");
    }

    function mount(root) {
        if (!root || root.__relatedMounted) return;
        root.__relatedMounted = true;

        var max = parseInt(root.getAttribute("data-max") || "8", 10) || 8;
        var exclude = parseInt(root.getAttribute("data-exclude") || "0", 10) || 0;
        var selectedEl = root.querySelector("[data-selected]");
        var resultsEl = root.querySelector("[data-results]");
        var pagerEl = root.querySelector("[data-pager]");
        var qEl = root.querySelector("[data-q]");
        if (!selectedEl || !resultsEl || !pagerEl || !qEl) return;

        var page = 1;
        var total = 0;
        var limit = 24;
        var timer = null;
        var reqSeq = 0;

        function selectedIds() {
            return Array.prototype.map.call(
                selectedEl.querySelectorAll(".admin-related-chip"),
                function (el) {
                    return parseInt(el.getAttribute("data-id"), 10);
                }
            );
        }

        function syncEmpty() {
            root.classList.toggle("is-empty", selectedIds().length === 0);
        }

        function addItem(item) {
            var ids = selectedIds();
            if (ids.indexOf(item.id) >= 0) return;
            if (ids.length >= max) return;
            if (exclude > 0 && item.id === exclude) return;

            var li = document.createElement("li");
            li.className = "admin-related-chip";
            li.setAttribute("data-id", String(item.id));
            li.innerHTML =
                '<button type="button" class="admin-related-move" data-up aria-label="上移">↑</button>' +
                '<button type="button" class="admin-related-move" data-down aria-label="下移">↓</button>' +
                '<span class="admin-related-chip-title">' +
                esc(item.title) +
                "</span>" +
                '<button type="button" class="admin-related-chip-x" data-remove aria-label="移除">×</button>' +
                '<input type="hidden" name="related_ids" value="' +
                item.id +
                '">';
            selectedEl.appendChild(li);
            syncEmpty();
            renderResults(resultsEl.__lastItems || []);
        }

        function removeChip(chip) {
            if (chip && chip.parentNode) chip.parentNode.removeChild(chip);
            syncEmpty();
            renderResults(resultsEl.__lastItems || []);
        }

        function moveChip(chip, dir) {
            if (!chip || !chip.parentNode) return;
            if (dir < 0 && chip.previousElementSibling) {
                chip.parentNode.insertBefore(chip, chip.previousElementSibling);
            } else if (dir > 0 && chip.nextElementSibling) {
                chip.parentNode.insertBefore(chip.nextElementSibling, chip);
            }
        }

        function renderResults(items) {
            resultsEl.__lastItems = items || [];
            var ids = selectedIds();
            if (!items || !items.length) {
                resultsEl.hidden = true;
                resultsEl.innerHTML = "";
                return;
            }
            resultsEl.hidden = false;
            resultsEl.innerHTML = items
                .map(function (it) {
                    var taken = ids.indexOf(it.id) >= 0;
                    var full = ids.length >= max;
                    var disabled = taken || full || (exclude > 0 && it.id === exclude);
                    var meta = esc(it.date || "") + (it.status === "draft" ? " · 草稿" : "");
                    return (
                        '<button type="button" class="admin-related-hit' +
                        (disabled ? " is-disabled" : "") +
                        '" data-id="' +
                        it.id +
                        '"' +
                        (disabled ? " disabled" : "") +
                        ">" +
                        '<span class="admin-related-hit-title">' +
                        esc(it.title) +
                        "</span>" +
                        '<span class="admin-related-hit-meta">' +
                        meta +
                        "</span>" +
                        "</button>"
                    );
                })
                .join("");
        }

        function renderPager() {
            var pages = Math.max(1, Math.ceil(total / limit));
            if (total <= limit) {
                pagerEl.hidden = true;
                pagerEl.innerHTML = "";
                return;
            }
            pagerEl.hidden = false;
            pagerEl.innerHTML =
                '<button type="button" class="layui-btn layui-btn-primary layui-btn-xs" data-prev' +
                (page <= 1 ? " disabled" : "") +
                ">上一页</button>" +
                '<span class="admin-related-page">' +
                page +
                " / " +
                pages +
                "</span>" +
                '<button type="button" class="layui-btn layui-btn-primary layui-btn-xs" data-next' +
                (page >= pages ? " disabled" : "") +
                ">下一页</button>";
        }

        function fetchPage() {
            var seq = ++reqSeq;
            var q = (qEl.value || "").trim();
            var url =
                "/admin/api/posts?page=" +
                page +
                "&limit=" +
                limit +
                (exclude > 0 ? "&exclude=" + exclude : "") +
                (q ? "&q=" + encodeURIComponent(q) : "");
            fetch(url, {credentials: "same-origin", headers: {Accept: "application/json"}})
                .then(function (res) {
                    if (!res.ok) throw new Error("load failed");
                    return res.json();
                })
                .then(function (data) {
                    if (seq !== reqSeq) return;
                    total = data.total || 0;
                    limit = data.limit || 24;
                    page = data.page || page;
                    renderResults(data.items || []);
                    renderPager();
                })
                .catch(function () {
                    if (seq !== reqSeq) return;
                    resultsEl.hidden = true;
                    resultsEl.innerHTML = "";
                    pagerEl.hidden = true;
                });
        }

        function scheduleFetch(resetPage) {
            if (resetPage) page = 1;
            clearTimeout(timer);
            timer = setTimeout(fetchPage, DEBOUNCE_MS);
        }


        Array.prototype.forEach.call(selectedEl.querySelectorAll(".admin-related-chip"), function (chip) {
            if (chip.querySelector("[data-up]")) return;
            var up = document.createElement("button");
            up.type = "button";
            up.className = "admin-related-move";
            up.setAttribute("data-up", "");
            up.setAttribute("aria-label", "上移");
            up.textContent = "↑";
            var down = document.createElement("button");
            down.type = "button";
            down.className = "admin-related-move";
            down.setAttribute("data-down", "");
            down.setAttribute("aria-label", "下移");
            down.textContent = "↓";
            chip.insertBefore(down, chip.firstChild);
            chip.insertBefore(up, chip.firstChild);
        });
        syncEmpty();

        selectedEl.addEventListener("click", function (e) {
            var t = e.target;
            if (!t) return;
            var chip = t.closest(".admin-related-chip");
            if (!chip) return;
            if (t.closest("[data-remove]")) {
                e.preventDefault();
                removeChip(chip);
            } else if (t.closest("[data-up]")) {
                e.preventDefault();
                moveChip(chip, -1);
            } else if (t.closest("[data-down]")) {
                e.preventDefault();
                moveChip(chip, 1);
            }
        });

        resultsEl.addEventListener("click", function (e) {
            var btn = e.target && e.target.closest(".admin-related-hit");
            if (!btn || btn.disabled) return;
            e.preventDefault();
            var id = parseInt(btn.getAttribute("data-id"), 10);
            var items = resultsEl.__lastItems || [];
            for (var i = 0; i < items.length; i++) {
                if (items[i].id === id) {
                    addItem(items[i]);
                    break;
                }
            }
        });

        pagerEl.addEventListener("click", function (e) {
            var t = e.target;
            if (!t) return;
            if (t.closest("[data-prev]") && page > 1) {
                page -= 1;
                fetchPage();
            } else if (t.closest("[data-next]")) {
                var pages = Math.max(1, Math.ceil(total / limit));
                if (page < pages) {
                    page += 1;
                    fetchPage();
                }
            }
        });

        qEl.addEventListener("input", function () {
            scheduleFetch(true);
        });
        qEl.addEventListener("focus", function () {
            if (resultsEl.hidden) scheduleFetch(false);
        });

        scheduleFetch(true);
    }

    global.OakisRelatedPosts = {mount: mount};
})(window);
