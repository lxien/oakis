layui.use(["element", "form", "layer", "laypage"], function () {
    var element = layui.element;
    var form = layui.form;
    var layer = layui.layer;
    var laypage = layui.laypage;
    var $ = layui.$;

    function pathMatches(href, path, search) {
        if (!href || href === "#" || href.indexOf("javascript:") === 0) return false;
        var parts = href.split("?");
        var hrefPath = parts[0];
        var hrefQuery = parts[1] || "";

        if (hrefPath === "/admin") return path === "/admin";
        if (path !== hrefPath && path.indexOf(hrefPath + "/") !== 0) return false;
        if (!hrefQuery) return true;

        var params = new URLSearchParams(hrefQuery);
        var cur = new URLSearchParams(search || "");
        var ok = true;
        params.forEach(function (v, k) {
            if (cur.get(k) !== v) ok = false;
        });
        return ok;
    }

    function highlightNav() {
        var path = location.pathname;
        var search = location.search || "";
        var $items = $("#admin-side-nav").find("a[data-admin-nav]");
        var best = null;
        var bestScore = -1;

        $items.each(function () {
            var href = this.getAttribute("href") || "";
            if (!pathMatches(href, path, search)) return;
            var score = href.length;
            if (path.indexOf("/admin/taxonomies/") === 0 && href.indexOf("/admin/taxonomies") === 0) score += 50;
            if (path.indexOf("/admin/posts/") === 0 && href === "/admin/posts") score += 50;
            if (path.indexOf("/admin/pages/") === 0 && href === "/admin/pages") score += 50;
            if (path.indexOf("/admin/sparks/") === 0 && href === "/admin/sparks") score += 50;
            if (score > bestScore) {
                bestScore = score;
                best = this;
            }
        });

        $("#admin-side-nav").find(".layui-this").removeClass("layui-this");
        $("#admin-side-nav").find(".layui-nav-itemed").removeClass("layui-nav-itemed");
        if (!best) return;

        var $a = $(best);
        var $dd = $a.closest("dd");
        var $li = $a.closest("li.layui-nav-item");
        if ($dd.length) {
            $dd.addClass("layui-this");
            $li.addClass("layui-nav-itemed");
        } else {
            $li.addClass("layui-this");
        }
    }

    function confirmDialog(msg, onYes, title) {
        layer.confirm(
            msg,
            {
                icon: 3,
                title: title || "确认",
                btn: ["确定", "取消"],
            },
            function (index) {
                layer.close(index);
                if (typeof onYes === "function") onYes();
            }
        );
    }

    window.OakisConfirm = confirmDialog;

    function bindDeleteConfirm() {
        $(document).on("submit", "form[data-confirm]", function (e) {
            var formEl = this;
            var msg = formEl.getAttribute("data-confirm") || "确定删除？";
            e.preventDefault();
            confirmDialog(msg, function () {
                HTMLFormElement.prototype.submit.call(formEl);
            });
        });

        $(document).on("click", "button[data-confirm]", function (e) {
            var btn = this;
            if (btn.disabled || btn.classList.contains("layui-btn-disabled")) return;
            e.preventDefault();
            e.stopPropagation();
            var msg = btn.getAttribute("data-confirm") || "确定删除？";
            confirmDialog(msg, function () {
                var formEl = btn.form;
                if (!formEl) return;
                if (formEl.requestSubmit) {
                    formEl.requestSubmit(btn);
                } else {
                    var action = btn.getAttribute("formaction");
                    if (action) formEl.setAttribute("action", action);
                    HTMLFormElement.prototype.submit.call(formEl);
                }
            });
        });

        $(document).on("submit", "form.admin-batch-form", function (e) {
            var submitter = e.originalEvent && e.originalEvent.submitter;
            if (submitter && submitter.getAttribute("formaction")) {
                return;
            }
            if (submitter && submitter.getAttribute("data-confirm")) {
                return;
            }
            e.preventDefault();
            var $form = $(this);
            var ids = $form
                .find(".admin-check-item:checked")
                .map(function () {
                    return this.value;
                })
                .get();
            if (!ids.length) return;
            $form.find('input[name="ids"]').val(ids.join(","));
            var tpl = this.getAttribute("data-batch-confirm") || "确定删除选中的 {n} 项？";
            var msg = tpl.replace("{n}", String(ids.length));
            var formEl = this;
            confirmDialog(msg, function () {
                HTMLFormElement.prototype.submit.call(formEl);
            });
        });
    }

    function bindBatchSelect() {
        function sync($form) {
            var $items = $form.find(".admin-check-item");
            var n = $items.filter(":checked").length;
            var $btn = $form.find(".admin-batch-delete");
            $btn.prop("disabled", n === 0).toggleClass("layui-btn-disabled", n === 0);
            $form.find(".admin-check-all").prop("checked", $items.length > 0 && n === $items.length);
        }

        $(document).on("change", "form.admin-batch-form .admin-check-all", function () {
            var $form = $(this).closest("form.admin-batch-form");
            $form.find(".admin-check-item").prop("checked", this.checked);
            sync($form);
        });

        $(document).on("change", "form.admin-batch-form .admin-check-item", function () {
            sync($(this).closest("form.admin-batch-form"));
        });

        $("form.admin-batch-form").each(function () {
            sync($(this));
        });
    }

    function initPager() {
        var el = document.getElementById("admin-pager");
        if (!el) return;
        var count = Number(el.getAttribute("data-count") || 0);
        var limit = Number(el.getAttribute("data-limit") || 10);
        var curr = Number(el.getAttribute("data-curr") || 1);
        var base = el.getAttribute("data-base") || location.pathname;
        if (count <= limit) return;

        laypage.render({
            elem: "admin-pager",
            count: count,
            limit: limit,
            curr: curr,
            layout: ["count", "prev", "page", "next"],
            jump: function (obj, first) {
                if (first) return;
                var sep = base.indexOf("?") >= 0 ? "&" : "?";
                location.href = base + sep + "page=" + obj.curr;
            },
        });
    }

    function showToast(message, ok) {
        if (!message) return;
        layer.msg(message, {
            icon: ok ? 1 : 2,
            time: ok ? 2200 : 4200,
            anim: 0,
        });
    }

    function bindCopy() {
        $(document).on("click", "[data-copy]", function (e) {
            e.preventDefault();
            var text = this.getAttribute("data-copy") || "";
            if (!text) return;

            function ok() {
                layer.msg("已复制", {icon: 1, time: 1200});
            }

            function fail() {
                layer.msg("复制失败", {icon: 2, time: 1800});
            }

            if (navigator.clipboard && navigator.clipboard.writeText) {
                navigator.clipboard.writeText(text).then(ok, fail);
                return;
            }
            try {
                var ta = document.createElement("textarea");
                ta.value = text;
                ta.setAttribute("readonly", "");
                ta.style.position = "fixed";
                ta.style.left = "-9999px";
                document.body.appendChild(ta);
                ta.select();
                document.execCommand("copy");
                document.body.removeChild(ta);
                ok();
            } catch (err) {
                fail();
            }
        });
    }

    function bindPageToasts() {
        var errEl = document.querySelector(".admin-toast-error") || document.querySelector("[data-admin-error]");
        if (errEl) {
            var errText = (errEl.getAttribute("data-admin-error") || errEl.textContent || "").trim();
            if (errText) showToast(errText, false);
        }
        var okEl = document.querySelector(".admin-toast-ok, [data-admin-ok]");
        if (okEl) {
            var okText = (okEl.getAttribute("data-admin-ok") || okEl.textContent || "").trim();
            if (okText) showToast(okText, true);
        }
    }

    function loadFlash() {
        fetch("/admin/api/flash", {credentials: "same-origin", headers: {Accept: "application/json"}})
            .then(function (res) {
                if (!res.ok) return null;
                return res.json();
            })
            .then(function (data) {
                if (!data || !data.message) return;
                showToast(data.message, data.level === "ok");
            })
            .catch(function () {
            });
    }

    function bindMediaUpload() {
        var btns = document.querySelectorAll("#admin-media-pick");
        if (!btns.length) return;

        function openUpload() {
            if (!window.OakisMediaPicker) return;
            OakisMediaPicker.open({
                kind: "all",
                crop: {aspectRatio: 0, maxEdge: 1600},
                onSelect: function () {
                    window.location.reload();
                },
            });
        }

        btns.forEach(function (btn) {
            btn.addEventListener("click", function (e) {
                e.preventDefault();
                openUpload();
            });
        });
    }

    $('#admin-side-nav [data-nav-group="content"]').addClass("layui-nav-itemed");
    if (location.pathname.indexOf("/admin/taxonomies") === 0) {
        $('#admin-side-nav [data-nav-group="taxonomy"]').addClass("layui-nav-itemed");
    }
    if (location.pathname.indexOf("/admin/settings") === 0 || location.pathname.indexOf("/admin/nav") === 0) {
        $('#admin-side-nav [data-nav-group="system"]').addClass("layui-nav-itemed");
    }
    highlightNav();
    element.render("nav");
    form.render();
    bindDeleteConfirm();
    bindBatchSelect();
    bindMediaUpload();
    bindCopy();
    initPager();
    bindPageToasts();
    loadFlash();
});
