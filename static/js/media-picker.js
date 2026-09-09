(function (global) {
    var LAYER = null;
    var $ = null;

    var ACCEPT_IMAGE = "image/png,image/jpeg,image/webp,image/gif";
    var ACCEPT_ALL =
        ACCEPT_IMAGE + ",.pdf,.zip,.txt,.md,.csv,.json,application/pdf,application/zip,text/plain";

    function ensureLayui(cb) {
        if (typeof layui === "undefined") {
            console.error("media-picker: layui required");
            return;
        }
        layui.use(["layer", "jquery"], function () {
            LAYER = layui.layer;
            $ = layui.$;
            cb();
        });
    }

    function esc(s) {
        return String(s == null ? "" : s)
            .replace(/&/g, "&amp;")
            .replace(/</g, "&lt;")
            .replace(/"/g, "&quot;");
    }

    function isImageMime(mime) {
        return String(mime || "")
            .toLowerCase()
            .indexOf("image/") === 0;
    }

    function fileExtLabel(name, mime) {
        var n = String(name || "");
        var i = n.lastIndexOf(".");
        if (i >= 0 && i < n.length - 1) {
            return n.slice(i + 1).toUpperCase().slice(0, 6);
        }
        if (mime && mime.indexOf("/") > 0) {
            return mime.split("/")[1].toUpperCase().slice(0, 6);
        }
        return "FILE";
    }


    function resolveCrop(opts) {
        if (opts.crop === false) return null;
        var c = opts.crop;
        if (c == null || c === true) {
            return {aspectRatio: 0, maxEdge: 1600};
        }
        if (typeof c === "object") {
            return {
                aspectRatio: typeof c.aspectRatio === "number" && c.aspectRatio > 0 ? c.aspectRatio : 0,
                maxEdge: c.maxEdge > 0 ? c.maxEdge : 1200,
            };
        }
        return {aspectRatio: 0, maxEdge: 1600};
    }

    function parseUploadResponse(res) {
        return res.text().then(function (text) {
            var data = null;
            try {
                data = text ? JSON.parse(text) : null;
            } catch (e) {
                data = null;
            }
            if (!res.ok) {
                var msg =
                    (data && data.error) ||
                    (res.status === 403
                        ? "请求无效或已过期，请刷新页面后重试"
                        : "上传失败");
                throw new Error(msg);
            }
            if (!data || !data.url) throw new Error("上传失败");
            return data;
        });
    }

    function uploadFile(file, onOk, onErr) {
        var body = new FormData();
        body.append("file", file);
        var tip = LAYER.load(1, {shade: 0.1});
        var headers = {Accept: "application/json"};
        if (global.OakisCsrf && global.OakisCsrf.token) {
            var token = global.OakisCsrf.token();
            if (token) headers[global.OakisCsrf.headerName || "X-CSRF-Token"] = token;
        }
        fetch("/admin/upload", {
            method: "POST",
            body: body,
            credentials: "same-origin",
            headers: headers,
        })
            .then(parseUploadResponse)
            .then(function (data) {
                LAYER.close(tip);
                onOk(data);
            })
            .catch(function (err) {
                LAYER.close(tip);
                onErr(err);
            });
    }

    function openPicker(opts) {
        opts = opts || {};
        var kind = opts.kind === "all" ? "all" : "image";
        var imagesOnly = kind !== "all";
        var crop = resolveCrop(opts);
        var onSelect = typeof opts.onSelect === "function" ? opts.onSelect : function () {
        };
        var page = 1;
        var total = 0;
        var limit = 24;
        var loading = false;

        var toolbar =
            '<div class="mp-toolbar">' +
            '<button type="button" class="layui-btn layui-btn-sm" id="mp-upload-btn">上传</button>' +
            '<input type="file" id="mp-file" accept="' +
            (imagesOnly ? ACCEPT_IMAGE : ACCEPT_ALL) +
            '" hidden>' +
            "</div>";

        var html =
            '<div class="mp-wrap">' + toolbar + '<div class="mp-grid" id="mp-grid"></div>' +
            '<div class="mp-pager" id="mp-pager"></div>' +
            "</div>";

        var areaW = window.innerWidth < 780 ? "92%" : "720px";
        var areaH = window.innerHeight < 640 ? "80%" : "520px";
        var index = LAYER.open({
            type: 1,
            title: imagesOnly ? "选择图片" : "选择媒体",
            area: [areaW, areaH],
            maxWidth: 720,
            shadeClose: true,
            content: html,
            success: function (layero) {
                var $root = $(layero);
                var $grid = $root.find("#mp-grid");
                var $pager = $root.find("#mp-pager");
                var $file = $root.find("#mp-file");

                function renderPager() {
                    var pages = Math.max(1, Math.ceil(total / limit));
                    if (total === 0) {
                        $pager.html("");
                        return;
                    }
                    $pager.html(
                        '<button type="button" class="layui-btn layui-btn-primary layui-btn-xs" id="mp-prev"' +
                        (page <= 1 ? " disabled" : "") +
                        ">上一页</button>" +
                        '<span class="mp-pager-info">' +
                        page +
                        " / " +
                        pages +
                        "</span>" +
                        '<button type="button" class="layui-btn layui-btn-primary layui-btn-xs" id="mp-next"' +
                        (page >= pages ? " disabled" : "") +
                        ">下一页</button>"
                    );
                    $pager.find("#mp-prev").on("click", function () {
                        if (page > 1) load(page - 1);
                    });
                    $pager.find("#mp-next").on("click", function () {
                        if (page < pages) load(page + 1);
                    });
                }

                function pick(item) {
                    LAYER.close(index);
                    onSelect(item);
                }

                function finishUpload(data, fallbackMime, fallbackName) {
                    var mime = data.mime || fallbackMime || "";
                    if (imagesOnly && !isImageMime(mime)) {
                        LAYER.msg("请选择图片文件", {icon: 2});
                        return;
                    }
                    pick({
                        id: 0,
                        name: data.name || fallbackName || "file",
                        url: data.url,
                        mime: mime,
                        thumb: isImageMime(mime) ? data.url : "",
                    });
                }

                function doUpload(uploadFileObj) {
                    uploadFile(
                        uploadFileObj,
                        function (data) {
                            finishUpload(data, uploadFileObj.type, uploadFileObj.name);
                        },
                        function (err) {
                            LAYER.msg(err.message || "上传失败", {icon: 2});
                        }
                    );
                }

                function openCropThenUpload(src) {
                    if (!global.OakisImageCrop) {
                        LAYER.msg("裁剪组件未加载", {icon: 2});
                        return;
                    }
                    var cropOpts = {
                        aspectRatio: crop.aspectRatio,
                        maxEdge: crop.maxEdge || 1200,
                        onDone: doUpload,
                        onCancel: function () {
                        },
                    };
                    if (src && src.file) cropOpts.file = src.file;
                    if (src && src.url) cropOpts.url = src.url;
                    global.OakisImageCrop.open(cropOpts);
                }


                function decideLocalImage(file) {
                    var previewUrl = URL.createObjectURL(file);
                    var decideIdx = LAYER.open({
                        type: 1,
                        title: "上传图片",
                        area: window.innerWidth < 560 ? "92%" : "420px",
                        shadeClose: true,
                        content:
                            '<div class="mp-decide">' +
                            '<div class="mp-decide-preview"><img src="' +
                            esc(previewUrl) +
                            '" alt=""></div>' +
                            '<p class="mp-decide-name">' +
                            esc(file.name || "image") +
                            "</p>" +
                            "</div>",
                        btn: ["直接上传", "裁剪", "取消"],
                        yes: function (idx) {
                            URL.revokeObjectURL(previewUrl);
                            LAYER.close(idx);
                            doUpload(file);
                        },
                        btn2: function (idx) {
                            URL.revokeObjectURL(previewUrl);
                            LAYER.close(idx);
                            openCropThenUpload({file: file});
                            return false;
                        },
                        btn3: function (idx) {
                            URL.revokeObjectURL(previewUrl);
                            LAYER.close(idx);
                        },
                        cancel: function () {
                            URL.revokeObjectURL(previewUrl);
                        },
                    });
                }

                function renderGrid(items) {
                    if (!items.length) {
                        $grid.html(
                            '<div class="mp-empty">' + (imagesOnly ? "暂无图片" : "暂无媒体") + "</div>"
                        );
                        return;
                    }
                    var buf = [];
                    items.forEach(function (it) {
                        var img = isImageMime(it.mime);
                        if (img) {
                            buf.push(
                                '<div class="mp-item' +
                                (crop ? " mp-item-cropable" : "") +
                                '" data-id="' +
                                it.id +
                                '" title="' +
                                esc(it.name) +
                                '">' +
                                '<button type="button" class="mp-item-main" data-act="use">' +
                                '<img src="' +
                                esc(it.thumb || it.url) +
                                '" alt="" loading="lazy" onerror="this.onerror=null;this.src=\'' +
                                esc(it.url) +
                                "';\">" +
                                "</button>" +
                                (crop
                                    ? '<button type="button" class="mp-crop-btn" data-act="crop" title="裁剪后使用">裁剪</button>'
                                    : "") +
                                "</div>"
                            );
                        } else {
                            buf.push(
                                '<button type="button" class="mp-item mp-item-file" data-id="' +
                                it.id +
                                '" data-act="use" title="' +
                                esc(it.name) +
                                '">' +
                                '<span class="mp-file-ext">' +
                                esc(fileExtLabel(it.name, it.mime)) +
                                "</span>" +
                                '<span class="mp-file-name">' +
                                esc(it.name) +
                                "</span>" +
                                "</button>"
                            );
                        }
                    });
                    $grid.html(buf.join(""));

                    function findItem(el) {
                        var id = Number($(el).closest("[data-id]").attr("data-id"));
                        return items.filter(function (x) {
                            return x.id === id;
                        })[0];
                    }

                    $grid.find('[data-act="use"]').on("click", function () {
                        var found = findItem(this);
                        if (found) pick(found);
                    });

                    $grid.find('[data-act="crop"]').on("click", function (e) {
                        e.preventDefault();
                        e.stopPropagation();
                        var found = findItem(this);
                        if (!found || !isImageMime(found.mime)) return;
                        openCropThenUpload({url: found.url});
                    });
                }

                function load(p) {
                    if (loading) return;
                    loading = true;
                    $grid.html('<div class="mp-empty">加载中…</div>');
                    var qs = "page=" + p + "&kind=" + encodeURIComponent(kind);
                    fetch("/admin/api/media?" + qs, {
                        credentials: "same-origin",
                        headers: {Accept: "application/json"},
                    })
                        .then(function (res) {
                            if (!res.ok) throw new Error("加载失败");
                            return res.json();
                        })
                        .then(function (data) {
                            page = data.page || 1;
                            total = data.total || 0;
                            limit = data.limit || 24;
                            renderGrid(data.items || []);
                            renderPager();
                        })
                        .catch(function () {
                            $grid.html('<div class="mp-empty">加载失败</div>');
                            $pager.html("");
                        })
                        .finally(function () {
                            loading = false;
                        });
                }

                $root.find("#mp-upload-btn").on("click", function () {
                    $file.trigger("click");
                });

                $file.on("change", function () {
                    var file = this.files && this.files[0];
                    this.value = "";
                    if (!file) return;
                    if (imagesOnly && !isImageMime(file.type)) {
                        LAYER.msg("请选择图片文件", {icon: 2});
                        return;
                    }

                    if (crop && isImageMime(file.type)) {
                        decideLocalImage(file);
                        return;
                    }

                    doUpload(file);
                });

                load(1);
            },
        });
    }

    global.OakisMediaPicker = {
        open: function (opts) {
            ensureLayui(function () {
                openPicker(opts);
            });
        },
    };
})(window);
