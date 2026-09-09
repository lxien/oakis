layui.use(["jquery", "layer"], function () {
    var $ = layui.$;
    var layer = layui.layer;

    var $root = $("#nav-items");
    var $addBtn = $("#nav-add");
    var tpl = document.getElementById("nav-row-template");
    if (!$root.length || !$addBtn.length || !tpl) return;

    var max = Number($root.attr("data-max") || 16) || 16;

    function $rows() {
        return $root.find(".nav-row");
    }

    function syncAddBtn() {
        var full = $rows().length >= max;
        $addBtn.prop("disabled", full).toggleClass("layui-btn-disabled", full);
    }

    function syncRowFields($row) {
        if ($row.attr("data-fixed") === "1") return;
        var kind = $row.find("[data-nav-type]").val() || "page";
        $row.attr("data-kind", kind);
        $row.find('[data-field="page"]').toggle(kind === "page");
        $row.find('[data-field="url"]').toggle(kind === "custom");
    }

    function addRow(focus) {
        if ($rows().length >= max) {
            layer.msg("最多 " + max + " 项", {icon: 0, time: 2000});
            return;
        }
        var node = tpl.content.firstElementChild.cloneNode(true);
        $root.append(node);
        var $row = $(node);
        syncRowFields($row);
        syncAddBtn();
        if (focus) {
            var sel = node.querySelector('select[name="nav_page_id"]');
            if (sel) sel.focus();
        }
    }

    $addBtn.on("click", function () {
        addRow(true);
    });

    $root.on("change", "[data-nav-type]", function () {
        syncRowFields($(this).closest(".nav-row"));
    });

    $root.on("click", "button[data-remove]", function (e) {
        e.preventDefault();
        var $row = $(this).closest(".nav-row");
        if ($row.attr("data-fixed") === "1") return;
        $row.remove();
        syncAddBtn();
    });

    $root.on("click", "button[data-move]", function (e) {
        e.preventDefault();
        var $row = $(this).closest(".nav-row");
        var delta = Number($(this).attr("data-move") || 0);
        if (!delta) return;
        if (delta < 0) {
            var $prev = $row.prevAll(".nav-row").first();
            if ($prev.length) $row.insertBefore($prev);
        } else {
            var $next = $row.nextAll(".nav-row").first();
            if ($next.length) $row.insertAfter($next);
        }
    });

    $rows().each(function () {
        syncRowFields($(this));
    });
    syncAddBtn();
});
