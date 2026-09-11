(function () {
    function bindToggle(btn, apply) {
        btn.addEventListener("click", function () {
            var open = !btn.classList.contains("is-expanded");
            btn.classList.toggle("is-expanded", open);
            btn.setAttribute("aria-pressed", open ? "true" : "false");
            var label = open ? "全部折叠" : "全部展开";
            btn.title = label;
            btn.setAttribute("aria-label", label);
            apply(open);
        });
    }

    document.querySelectorAll("[data-kb-tree-toggle]").forEach(function (btn) {
        var root = btn.closest("aside") || document;
        var tree = root.querySelector(".kb-admin-tree, .docs-tree");
        if (!tree) return;
        bindToggle(btn, function (open) {
            tree.querySelectorAll("details.kb-branch").forEach(function (el) {
                el.open = open;
            });
        });
    });

    document.querySelectorAll("[data-kb-catalog-toggle]").forEach(function (btn) {
        var home = btn.closest(".kb-book-home, .docs-home");
        var catalog = home && home.querySelector(".docs-catalog");
        if (!catalog) return;
        bindToggle(btn, function (open) {
            catalog.querySelectorAll(".docs-catalog-item.has-children").forEach(function (item) {
                item.classList.toggle("is-open", open);
                var rowBtn = item.querySelector(":scope > .docs-catalog-row > button.docs-catalog-toggle");
                if (rowBtn) rowBtn.setAttribute("aria-expanded", open ? "true" : "false");
            });
        });
    });

    document.querySelectorAll(".kb-admin-tree, .docs-tree").forEach(function (tree) {
        tree.addEventListener("click", function (e) {
            var summary = e.target.closest("details.kb-branch > summary");
            if (!summary || !tree.contains(summary)) return;
            if (e.target.closest("a, button, .kb-row-actions, .kb-action-wrap, .kb-menu")) {
                return;
            }
            if (!e.target.closest(".kb-chevron:not(.kb-chevron-leaf)")) {
                e.preventDefault();
            }
        });
    });
})();
