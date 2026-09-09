(function (global) {
    var FORMATS = [
        {
            id: "markdown",
            label: "Markdown",
            ext: ".md / .zip",
            enabled: true,
        },
    ];

    function ensureLayui(cb) {
        if (typeof layui === "undefined") return;
        layui.use(["layer"], cb);
    }

    function open(opts) {
        opts = opts || {};
        var source = opts.source;
        var id = Number(opts.id || 0);
        if (!source || !id) return;

        ensureLayui(function () {
            var layer = layui.layer;
            var cards = FORMATS.map(function (f) {
                var disabled = f.enabled ? "" : " is-disabled";
                return (
                    '<button type="button" class="ae-card' +
                    disabled +
                    '" data-format="' +
                    f.id +
                    '"' +
                    (f.enabled ? "" : " disabled") +
                    ">" +
                    '<span class="ae-card-label">' +
                    f.label +
                    "</span>" +
                    '<span class="ae-card-ext">' +
                    f.ext +
                    "</span>" +
                    "</button>"
                );
            }).join("");

            var index = layer.open({
                type: 1,
                title: "导出",
                area: ["420px", "auto"],
                shadeClose: true,
                content: '<div class="ae-wrap"><div class="ae-grid">' + cards + "</div></div>",
                success: function (layero) {
                    layero.find(".ae-card:not(.is-disabled)").on("click", function () {
                        var format = this.getAttribute("data-format");
                        if (!format) return;
                        layer.close(index);
                        var url =
                            "/admin/export/" +
                            encodeURIComponent(source) +
                            "/" +
                            id +
                            "?format=" +
                            encodeURIComponent(format);
                        window.location.href = url;
                    });
                },
            });
        });
    }

    global.OakisExport = {
        open: open,
        formats: FORMATS,
    };

    document.addEventListener("click", function (e) {
        var btn = e.target.closest("[data-export]");
        if (!btn) return;
        e.preventDefault();
        var source = btn.getAttribute("data-export");
        var id = btn.getAttribute("data-export-id");
        open({source: source, id: id});
    });
})(window);
