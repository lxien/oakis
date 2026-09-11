(function () {
    var tree = document.querySelector(".kb-admin-tree");
    if (!tree) return;

    var dragId = null;
    var dropTarget = null;
    var dropMode = null;
    var line = null;
    var expandTimer = null;
    var busy = false;

    function msg(text) {
        if (window.layui && layui.layer) layui.layer.msg(text);
        else window.alert(text);
    }

    function ensureLine() {
        if (line) return line;
        line = document.createElement("div");
        line.className = "kb-drop-line";
        line.hidden = true;
        tree.appendChild(line);
        return line;
    }

    function clearIndicators() {
        if (expandTimer) {
            clearTimeout(expandTimer);
            expandTimer = null;
        }
        tree.querySelectorAll(".kb-tree-item.is-drop-into").forEach(function (el) {
            el.classList.remove("is-drop-into");
        });
        if (line) line.hidden = true;
    }

    function clearDropUi() {
        clearIndicators();
        dropTarget = null;
        dropMode = null;
    }

    function nextSiblingItem(item) {
        var next = item.nextElementSibling;
        while (next && !next.classList.contains("kb-tree-item")) {
            next = next.nextElementSibling;
        }
        return next;
    }

    function isUnder(ancestorId, nodeEl) {
        var cur = nodeEl.parentElement;
        while (cur && cur !== tree) {
            if (cur.classList && cur.classList.contains("kb-tree-item")) {
                if (String(cur.getAttribute("data-id")) === String(ancestorId)) return true;
            }
            cur = cur.parentElement;
        }
        return false;
    }

    function rowOf(item) {
        return item.querySelector(":scope > .kb-row, :scope > .kb-branch > summary.kb-row");
    }

    function itemFromPoint(clientX, clientY) {
        var stack = document.elementsFromPoint(clientX, clientY);
        for (var i = 0; i < stack.length; i++) {
            var el = stack[i];
            if (!el || !el.closest) continue;
            if (el.classList && (el.classList.contains("kb-row") || el.tagName === "SUMMARY")) {
                var item = el.closest(".kb-tree-item");
                if (item && tree.contains(item)) return item;
            }
        }
        for (var j = 0; j < stack.length; j++) {
            var node = stack[j];
            if (node && node.classList && node.classList.contains("kb-tree-item") && tree.contains(node)) {
                return node;
            }
        }
        return null;
    }

    function lastRootItem() {
        var roots = tree.querySelectorAll(":scope > .kb-tree-list > .kb-tree-item");
        return roots.length ? roots[roots.length - 1] : null;
    }

    // Any node can host children: before / into / after.
    function resolveMode(item, clientY) {
        var row = rowOf(item);
        if (!row) return null;
        var rect = row.getBoundingClientRect();
        var y = clientY - rect.top;
        var h = rect.height || 1;
        var isLast = !nextSiblingItem(item);
        if (y < h * 0.22) return "before";
        if (isLast && y > h * 0.45) return "after";
        if (y > h * 0.78) return "after";
        return "into";
    }

    function showDrop(item, mode) {
        if (dropTarget === item && dropMode === mode) {
            if (mode === "into" && item.classList.contains("is-drop-into")) return;
            if (mode !== "into" && line && !line.hidden) return;
        }
        clearIndicators();
        dropTarget = item;
        dropMode = mode;
        if (mode === "into") {
            item.classList.add("is-drop-into");
            var details = item.querySelector(":scope > details.kb-branch");
            if (details && !details.open) {
                expandTimer = setTimeout(function () {
                    details.open = true;
                }, 400);
            }
            return;
        }
        var row = rowOf(item);
        if (!row) return;
        var ln = ensureLine();
        var treeRect = tree.getBoundingClientRect();
        var rowRect = row.getBoundingClientRect();
        var top = mode === "before" ? rowRect.top : rowRect.bottom;
        ln.style.top = top - treeRect.top + tree.scrollTop - 1 + "px";
        ln.style.left = rowRect.left - treeRect.left + "px";
        ln.style.width = Math.max(rowRect.width, 40) + "px";
        ln.hidden = false;
    }

    function toPayload(item, mode) {
        if (mode === "into") {
            return { parentId: item.getAttribute("data-id") || "", beforeId: "" };
        }
        var parentId = item.getAttribute("data-parent-id") || "";
        if (mode === "before") {
            return { parentId: parentId, beforeId: item.getAttribute("data-id") || "" };
        }
        var next = nextSiblingItem(item);
        return {
            parentId: parentId,
            beforeId: next ? next.getAttribute("data-id") || "" : "",
        };
    }

    function isNoop(dragEl, parentId, beforeId) {
        if ((dragEl.getAttribute("data-parent-id") || "") !== parentId) return false;
        var next = nextSiblingItem(dragEl);
        var curBefore = next ? next.getAttribute("data-id") || "" : "";
        return curBefore === beforeId;
    }

    function wouldCycle(dragEl, parentId) {
        if (!parentId) return false;
        var id = dragEl.getAttribute("data-id");
        if (parentId === id) return true;
        var parentLi = tree.querySelector('.kb-tree-item[data-id="' + parentId + '"]');
        return !!(parentLi && isUnder(id, parentLi));
    }

    // Leaf rows use <div class="kb-row">; promote to <details> so children can nest.
    function ensureBranch(parentLi) {
        var details = parentLi.querySelector(":scope > details.kb-branch");
        if (details) {
            details.open = true;
            var existing = details.querySelector(":scope > .kb-tree-list");
            if (!existing) {
                existing = document.createElement("ul");
                existing.className = "kb-tree-list";
                details.appendChild(existing);
            }
            parentLi.classList.add("has-children");
            return existing;
        }

        var row = parentLi.querySelector(":scope > .kb-row");
        if (!row) return null;

        details = document.createElement("details");
        details.className = "kb-branch";
        details.open = true;

        var summary = document.createElement("summary");
        summary.className = "kb-row";
        while (row.firstChild) {
            summary.appendChild(row.firstChild);
        }
        var chevron = summary.querySelector(".kb-chevron");
        if (chevron) chevron.classList.remove("kb-chevron-leaf");
        row.remove();

        var ul = document.createElement("ul");
        ul.className = "kb-tree-list";
        details.appendChild(summary);
        details.appendChild(ul);
        parentLi.appendChild(details);
        parentLi.classList.add("has-children");
        return ul;
    }

    function childList(parentId) {
        if (!parentId) {
            var rootUl = tree.querySelector(":scope > .kb-tree-list");
            if (!rootUl) {
                rootUl = document.createElement("ul");
                rootUl.className = "kb-tree-list";
                tree.appendChild(rootUl);
            }
            return rootUl;
        }
        var parentLi = tree.querySelector('.kb-tree-item[data-id="' + parentId + '"]');
        if (!parentLi) return null;
        return ensureBranch(parentLi);
    }

    function setDepth(li, depth) {
        li.style.setProperty("--kb-depth", String(depth));
        var kids = li.querySelectorAll(":scope > details.kb-branch > .kb-tree-list > .kb-tree-item");
        for (var i = 0; i < kids.length; i++) {
            setDepth(kids[i], depth + 1);
        }
    }

    function applyDom(dragEl, parentId, beforeId) {
        var oldUl = dragEl.parentElement;
        var ul = childList(parentId);
        if (!ul) return false;
        if (beforeId) {
            var beforeLi = ul.querySelector(':scope > .kb-tree-item[data-id="' + beforeId + '"]');
            if (beforeLi) ul.insertBefore(dragEl, beforeLi);
            else ul.appendChild(dragEl);
        } else {
            ul.appendChild(dragEl);
        }
        dragEl.setAttribute("data-parent-id", parentId);
        var depth = 0;
        if (parentId) {
            var parentLi = tree.querySelector('.kb-tree-item[data-id="' + parentId + '"]');
            if (parentLi) {
                depth = (parseInt(parentLi.style.getPropertyValue("--kb-depth"), 10) || 0) + 1;
            }
        }
        setDepth(dragEl, depth);
        if (oldUl && oldUl !== ul && !oldUl.querySelector(":scope > .kb-tree-item")) {
            oldUl.remove();
        }
        return true;
    }

    function postMove(id, parentId, beforeId) {
        var body = new URLSearchParams();
        body.set("parent_id", parentId);
        body.set("before_id", beforeId);
        return fetch("/admin/docs/nodes/" + id + "/move", {
            method: "POST",
            credentials: "same-origin",
            headers: {
                "Content-Type": "application/x-www-form-urlencoded;charset=UTF-8",
                Accept: "application/json",
            },
            body: body.toString(),
        });
    }

    function updateDropFromEvent(e) {
        var item = itemFromPoint(e.clientX, e.clientY);
        if (!item) {
            var last = lastRootItem();
            if (!last || last.getAttribute("data-id") === dragId) return false;
            if (isUnder(dragId, last)) return false;
            e.preventDefault();
            e.dataTransfer.dropEffect = "move";
            showDrop(last, "after");
            return true;
        }
        if (item.getAttribute("data-id") === dragId) return false;
        if (isUnder(dragId, item)) return false;

        var mode = resolveMode(item, e.clientY);
        if (!mode) return false;
        if (mode === "into") {
            var dragEl = tree.querySelector('.kb-tree-item[data-id="' + dragId + '"]');
            if (dragEl && wouldCycle(dragEl, item.getAttribute("data-id"))) return false;
        }

        e.preventDefault();
        e.dataTransfer.dropEffect = "move";
        showDrop(item, mode);
        return true;
    }

    tree.addEventListener("dragstart", function (e) {
        if (e.target.closest(".kb-row-actions, .kb-action-wrap, .kb-menu")) {
            e.preventDefault();
            return;
        }
        var item = e.target.closest(".kb-tree-item");
        if (!item || !tree.contains(item)) return;
        dragId = item.getAttribute("data-id");
        item.classList.add("is-dragging");
        e.dataTransfer.effectAllowed = "move";
        e.dataTransfer.setData("text/plain", dragId);
        try {
            e.dataTransfer.setDragImage(rowOf(item) || item, 12, 12);
        } catch (_) {}
    });

    tree.addEventListener("dragend", function () {
        var dragging = tree.querySelector(".kb-tree-item.is-dragging");
        if (dragging) dragging.classList.remove("is-dragging");
        clearDropUi();
        dragId = null;
    });

    tree.addEventListener("dragover", function (e) {
        if (!dragId || busy) return;
        updateDropFromEvent(e);
    });

    tree.addEventListener("dragleave", function (e) {
        if (!tree.contains(e.relatedTarget)) clearIndicators();
    });

    tree.addEventListener("drop", function (e) {
        e.preventDefault();
        e.stopPropagation();
        if (!dragId || busy) {
            clearDropUi();
            return;
        }
        updateDropFromEvent(e);
        if (!dropTarget || !dropMode) {
            clearDropUi();
            return;
        }

        var id = dragId;
        var dragEl = tree.querySelector('.kb-tree-item[data-id="' + id + '"]');
        var target = dropTarget;
        var mode = dropMode;
        clearDropUi();
        if (!dragEl || isUnder(id, target)) return;

        var payload = toPayload(target, mode);
        if (wouldCycle(dragEl, payload.parentId)) {
            msg("不能移动到自己的子目录中");
            return;
        }
        if (isNoop(dragEl, payload.parentId, payload.beforeId)) return;

        busy = true;
        postMove(id, payload.parentId, payload.beforeId)
            .then(function (res) {
                if (!res.ok) {
                    msg("移动失败，请刷新后重试");
                    location.reload();
                    return;
                }
                if (!applyDom(dragEl, payload.parentId, payload.beforeId)) {
                    location.reload();
                }
            })
            .catch(function () {
                msg("移动失败，请刷新后重试");
                location.reload();
            })
            .finally(function () {
                busy = false;
                var dragging = tree.querySelector(".kb-tree-item.is-dragging");
                if (dragging) dragging.classList.remove("is-dragging");
                dragId = null;
            });
    });

    tree.addEventListener(
        "click",
        function (e) {
            if (dragId && e.target.closest("a.kb-tree-link")) e.preventDefault();
        },
        true
    );
})();
