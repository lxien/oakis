layui.use(["jquery", "layer"], function () {
    var $ = layui.$;
    var layer = layui.layer;

    var $root = $("#social-links");
    var $addBtn = $("#social-add");
    var tpl = document.getElementById("social-row-template");
    if (!$root.length || !$addBtn.length || !tpl) return;

    var max = Number($root.attr("data-max") || 12) || 12;
    var defaults = {
        github: "GitHub",
        gitee: "Gitee",
        site: "个人站",
        email: "邮箱",
        twitter: "X / Twitter",
        bilibili: "Bilibili",
        rss: "RSS",
        custom: "链接",
    };
    var defaultLabels = Object.keys(defaults).map(function (k) {
        return defaults[k];
    });

    function $rows() {
        return $root.find(".social-row");
    }

    function syncEmpty() {
        $root.find("[data-empty]").remove();
    }

    function syncAddBtn() {
        var full = $rows().length >= max;
        $addBtn.prop("disabled", full).toggleClass("layui-btn-disabled", full);
    }

    function applyKindLabel($row) {
        var kind = $row.find('select[name="social_kind"]').val() || "custom";
        var $label = $row.find('input[name="social_label"]');
        var next = defaults[kind] || "链接";
        var cur = ($label.val() || "").trim();
        if (!cur || defaultLabels.indexOf(cur) >= 0) {
            $label.val(next);
        }
    }

    function addRow(focus) {
        if ($rows().length >= max) {
            layer.msg("最多添加 " + max + " 条", {icon: 0, time: 2000});
            return;
        }
        var node = tpl.content.firstElementChild.cloneNode(true);
        $root.append(node);
        syncEmpty();
        syncAddBtn();
        if (focus) {
            var url = node.querySelector('input[name="social_url"]');
            if (url) url.focus();
        }
    }

    $addBtn.on("click", function () {
        addRow(true);
    });

    $root.on("change", 'select[name="social_kind"]', function () {
        applyKindLabel($(this).closest(".social-row"));
    });

    $root.on("click", "button[data-remove]", function (e) {
        e.preventDefault();
        $(this).closest(".social-row").remove();
        syncEmpty();
        syncAddBtn();
    });

    $root.on("click", "button[data-move]", function (e) {
        e.preventDefault();
        var $row = $(this).closest(".social-row");
        var delta = Number($(this).attr("data-move") || 0);
        if (!delta) return;
        if (delta < 0) {
            var $prev = $row.prevAll(".social-row").first();
            if ($prev.length) $row.insertBefore($prev);
        } else {
            var $next = $row.nextAll(".social-row").first();
            if ($next.length) $row.insertAfter($next);
        }
    });

    syncEmpty();
    syncAddBtn();
});
