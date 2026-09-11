(function () {
    var STORAGE_PREFIX = "oakis:kb-tree-open:";

    function storageKey(tree) {
        var id = tree.getAttribute("data-book-id");
        if (!id) return null;
        return STORAGE_PREFIX + id;
    }

    function readOpenIds(tree) {
        var key = storageKey(tree);
        if (!key) return null;
        try {
            var raw = localStorage.getItem(key);
            if (!raw) return null;
            var parsed = JSON.parse(raw);
            if (!Array.isArray(parsed)) return null;
            return new Set(parsed.map(String));
        } catch (e) {
            return null;
        }
    }

    function writeOpenIds(tree, ids) {
        var key = storageKey(tree);
        if (!key) return;
        try {
            localStorage.setItem(key, JSON.stringify(Array.from(ids)));
        } catch (e) {
            /* ignore quota / private mode */
        }
    }

    function collectOpenIds(tree) {
        var ids = new Set();
        tree.querySelectorAll("details.kb-branch[open]").forEach(function (el) {
            var item = el.closest(".kb-tree-item");
            if (item && item.getAttribute("data-id")) {
                ids.add(String(item.getAttribute("data-id")));
            }
        });
        return ids;
    }

    function openAncestorsOfActive(tree) {
        var active = tree.querySelector(".kb-tree-item.is-active");
        if (!active) return;
        var node = active.parentElement;
        while (node && tree.contains(node)) {
            if (node.matches && node.matches("details.kb-branch")) {
                node.open = true;
            }
            node = node.parentElement;
        }
    }

    function restoreTree(tree) {
        var saved = readOpenIds(tree);
        if (saved) {
            tree.querySelectorAll(".kb-tree-item.has-children").forEach(function (item) {
                var id = item.getAttribute("data-id");
                if (!id || !saved.has(String(id))) return;
                var details = item.querySelector(":scope > details.kb-branch");
                if (details) details.open = true;
            });
        }
        openAncestorsOfActive(tree);
        syncExpandAllButton(tree);
    }

    function syncExpandAllButton(tree) {
        var root = tree.closest("aside") || document;
        var btn = root.querySelector("[data-kb-tree-toggle]");
        if (!btn) return;
        var branches = tree.querySelectorAll("details.kb-branch");
        if (!branches.length) return;
        var allOpen = true;
        branches.forEach(function (el) {
            if (!el.open) allOpen = false;
        });
        btn.classList.toggle("is-expanded", allOpen);
        btn.setAttribute("aria-pressed", allOpen ? "true" : "false");
        var label = allOpen ? "全部折叠" : "全部展开";
        btn.title = label;
        btn.setAttribute("aria-label", label);
    }

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
            writeOpenIds(tree, open ? collectOpenIds(tree) : new Set());
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
        restoreTree(tree);

        tree.addEventListener("toggle", function (e) {
            var details = e.target;
            if (!details || !details.matches || !details.matches("details.kb-branch")) return;
            if (!tree.contains(details)) return;
            writeOpenIds(tree, collectOpenIds(tree));
            syncExpandAllButton(tree);
        }, true);

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
