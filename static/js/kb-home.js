(function () {
    function closeMenus(except) {
        document.querySelectorAll(".kb-card-actions .kb-menu").forEach(function (el) {
            if (el !== except) el.hidden = true;
        });
    }

    document.addEventListener("click", function (e) {
        var more = e.target.closest("[data-kb-card-more]");
        if (more) {
            e.preventDefault();
            e.stopPropagation();
            var menu = more.parentElement && more.parentElement.querySelector(".kb-menu");
            if (!menu) return;
            var open = menu.hidden;
            closeMenus(menu);
            menu.hidden = !open;
            return;
        }
        if (e.target.closest("[data-export]")) {
            closeMenus();
            return;
        }
        if (!e.target.closest(".kb-card-actions")) {
            closeMenus();
        }
    });

    function bindCoverPicker(root) {
        var tile = root.querySelector("[data-asset-tile]");
        if (!tile) return;
        var urlInput = tile.querySelector('input[name="cover_url"]');
        var preview = tile.querySelector("[data-cover-preview], .admin-asset-img-cover");
        if (!urlInput || !preview) return;

        function setCover(url) {
            urlInput.value = url || "";
            if (url) {
                preview.setAttribute("src", url);
                preview.hidden = false;
                tile.classList.remove("is-empty");
            } else {
                preview.removeAttribute("src");
                preview.hidden = true;
                tile.classList.add("is-empty");
            }
        }

        var pick = tile.querySelector("[data-pick]");
        if (pick) {
            pick.addEventListener("click", function (e) {
                e.preventDefault();
                if (!window.OakisMediaPicker) return;
                OakisMediaPicker.open({
                    kind: "image",
                    crop: {aspectRatio: 1, maxEdge: 800},
                    onSelect: function (item) {
                        setCover(item.url);
                    },
                });
            });
        }

        var clear = tile.querySelector("[data-clear]");
        if (clear) {
            clear.addEventListener("click", function (e) {
                e.preventDefault();
                e.stopPropagation();
                setCover("");
            });
        }
    }

    function openCreate() {
        var tpl = document.getElementById("kb-create-dialog");
        if (!tpl || !window.layui) return;
        layui.use(["layer", "form"], function () {
            var layer = layui.layer;
            var form = layui.form;
            layer.open({
                type: 1,
                title: "新建知识库",
                area: ["480px", "auto"],
                content: tpl.innerHTML,
                btn: ["创建并进入"],
                yes: function (_index, layero) {
                    var formEl = layero.find("form.kb-dialog-form")[0];
                    if (!formEl) return false;
                    if (typeof formEl.reportValidity === "function" && !formEl.reportValidity()) {
                        return false;
                    }
                    formEl.submit();
                    return false;
                },
                success: function (layero) {
                    form.render(null, "kb-create");
                    bindCoverPicker(layero[0] || layero);
                },
            });
        });
    }

    ["kb-open-create"].forEach(function (id) {
        var el = document.getElementById(id);
        if (el) el.addEventListener("click", openCreate);
    });
})();
