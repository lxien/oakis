(function () {
    function msg(text) {
        if (window.layui && layui.layer) layui.layer.msg(text);
        else window.alert(text);
    }

    function askTitle(label, defVal, onOk) {
        if (window.layui && layui.layer) {
            layui.layer.prompt({title: label, value: defVal || "", formType: 0}, function (value, index) {
                layui.layer.close(index);
                onOk(value);
            });
            return;
        }
        var value = window.prompt(label, defVal || "");
        if (value != null) onOk(value);
    }

    function submitCreate(type, parentId, title) {
        title = String(title || "").trim();
        if (!title) {
            msg("标题不能为空");
            return;
        }
        var form = document.getElementById("kb-create-form");
        var parentInput = document.getElementById("kb-create-parent");
        var typeInput = document.getElementById("kb-create-type");
        var titleInput = document.getElementById("kb-create-title");
        if (!form || !parentInput || !typeInput || !titleInput) {
            msg("创建表单缺失，请刷新页面");
            return;
        }
        parentInput.value = parentId || "";
        typeInput.value = type === "folder" ? "folder" : "doc";
        titleInput.value = title;
        form.submit();
    }

    function createNode(type, parentId) {
        var isFolder = type === "folder";
        askTitle(
            isFolder ? "文件夹名称" : "文档标题",
            isFolder ? "新建文件夹" : "未命名文档",
            function (value) {
                submitCreate(type, parentId, value);
            }
        );
    }

    function closeMenus(except) {
        document.querySelectorAll(".kb-action-wrap > .kb-menu, .kb-side-action > .kb-menu").forEach(function (el) {
            if (el !== except) el.hidden = true;
        });
    }

    function toggleMenu(btn) {
        var wrap = btn.closest(".kb-action-wrap, .kb-side-action");
        if (!wrap) return;
        var menu = wrap.querySelector(":scope > .kb-menu");
        if (!menu) return;
        var willOpen = menu.hidden;
        closeMenus(menu);
        menu.hidden = !willOpen;
    }


    document.addEventListener(
        "click",
        function (e) {
            if (e.target.closest("summary .kb-row-actions") && !e.target.closest("a[href]")) {
                e.preventDefault();
            }
        },
        true
    );

    document.addEventListener("click", function (e) {
        var createBtn = e.target.closest("[data-kb-create]");
        if (createBtn) {
            e.preventDefault();
            e.stopPropagation();
            closeMenus();
            createNode(createBtn.getAttribute("data-kb-create") || "doc", createBtn.getAttribute("data-parent") || "");
            return;
        }

        var addBtn = e.target.closest("[data-kb-add]");
        if (addBtn) {
            e.preventDefault();
            e.stopPropagation();
            toggleMenu(addBtn);
            return;
        }

        var moreBtn = e.target.closest("[data-kb-more]");
        if (moreBtn) {
            e.preventDefault();
            e.stopPropagation();
            toggleMenu(moreBtn);
            return;
        }

        var renameBtn = e.target.closest("[data-kb-rename]");
        if (renameBtn) {
            e.preventDefault();
            e.stopPropagation();
            closeMenus();
            var id = renameBtn.getAttribute("data-id");
            var cur = renameBtn.getAttribute("data-title") || "";
            askTitle("重命名", cur, function (next) {
                next = String(next || "").trim();
                if (!next) {
                    msg("标题不能为空");
                    return;
                }
                var renameForm = document.getElementById("kb-rename-form");
                var renameTitle = document.getElementById("kb-rename-title");
                if (!renameForm || !renameTitle) return;
                renameForm.action = "/admin/docs/nodes/" + id + "/rename";
                renameTitle.value = next;
                renameForm.submit();
            });
            return;
        }

        var copyBtn = e.target.closest("[data-kb-copy]");
        if (copyBtn) {
            e.preventDefault();
            e.stopPropagation();
            var path = copyBtn.getAttribute("data-kb-copy") || "";
            var url = location.origin + path;
            if (navigator.clipboard && navigator.clipboard.writeText) {
                navigator.clipboard.writeText(url).then(
                    function () {
                        msg("链接已复制");
                    },
                    function () {
                        window.prompt("复制链接", url);
                    }
                );
            } else {
                window.prompt("复制链接", url);
            }
            closeMenus();
            return;
        }

        if (e.target.closest("[data-export]")) {
            closeMenus();
            return;
        }

        var delBtn = e.target.closest("[data-kb-delete]");
        if (delBtn) {
            e.preventDefault();
            e.stopPropagation();
            closeMenus();
            var delId = delBtn.getAttribute("data-id");
            var kind = delBtn.getAttribute("data-type") === "folder" ? "文件夹" : "文档";
            var confirmMsg = "确定删除该" + kind + "？其下内容将一并删除。";
            var doDelete = function () {
                var delForm = document.getElementById("kb-delete-form");
                if (!delForm) return;
                delForm.action = "/admin/docs/nodes/" + delId + "/delete";
                delForm.submit();
            };
            if (window.OakisConfirm) {
                window.OakisConfirm(confirmMsg, doDelete);
            } else if (window.layui && layui.layer) {
                layui.layer.confirm(
                    confirmMsg,
                    {icon: 3, title: "确认", btn: ["确定", "取消"]},
                    function (index) {
                        layui.layer.close(index);
                        doDelete();
                    }
                );
            }
            return;
        }

        if (!e.target.closest(".kb-action-wrap") && !e.target.closest(".kb-side-action")) {
            closeMenus();
        }
    });

    document.querySelectorAll(".kb-branch > summary .kb-tree-link").forEach(function (a) {
        a.addEventListener("click", function (e) {
            e.stopPropagation();
        });
    });


    var catalog = document.querySelector(".kb-catalog");
    if (catalog) {
        catalog.addEventListener("click", function (e) {
            var btn = e.target.closest("button.docs-catalog-toggle");
            if (!btn) return;
            e.preventDefault();
            e.stopPropagation();
            var item = btn.closest(".docs-catalog-item");
            if (!item || !item.classList.contains("has-children")) return;
            var open = item.classList.toggle("is-open");
            btn.setAttribute("aria-expanded", open ? "true" : "false");
        });
    }
})();
