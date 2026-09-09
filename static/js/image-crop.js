(function (global) {
    var loading = null;
    var cropperCss = "/static/vendor/cropperjs/cropper.min.css";
    var cropperJs = "/static/vendor/cropperjs/cropper.min.js";

    function loadScript(src) {
        return new Promise(function (resolve, reject) {
            var s = document.createElement("script");
            s.src = src;
            s.async = true;
            s.onload = resolve;
            s.onerror = function () {
                reject(new Error("裁剪组件加载失败"));
            };
            document.head.appendChild(s);
        });
    }

    function loadCss(href) {
        if (document.querySelector('link[href="' + href + '"]')) {
            return Promise.resolve();
        }
        return new Promise(function (resolve, reject) {
            var l = document.createElement("link");
            l.rel = "stylesheet";
            l.href = href;
            l.onload = resolve;
            l.onerror = function () {
                reject(new Error("裁剪样式加载失败"));
            };
            document.head.appendChild(l);
        });
    }

    function ensureCropper() {
        if (global.Cropper) return Promise.resolve();
        if (loading) return loading;
        loading = Promise.all([loadCss(cropperCss), loadScript(cropperJs)]).then(function () {
            if (!global.Cropper) throw new Error("Cropper 未就绪");
        });
        return loading;
    }

    function openCrop(opts) {
        opts = opts || {};
        var file = opts.file;
        var url = opts.url;
        var aspectRatio =
            typeof opts.aspectRatio === "number" && opts.aspectRatio > 0 ? opts.aspectRatio : NaN;
        var maxEdge = opts.maxEdge > 0 ? opts.maxEdge : 1200;
        var onDone = typeof opts.onDone === "function" ? opts.onDone : function () {
        };
        var onCancel = typeof opts.onCancel === "function" ? opts.onCancel : function () {
        };
        var baseName = (file && file.name) || "image.jpg";

        if ((!file && !url) || !global.layui) {
            onCancel();
            return;
        }

        layui.use(["layer"], function () {
            var layer = layui.layer;
            var tip = layer.load(1, {shade: 0.08});

            ensureCropper()
                .then(function () {
                    layer.close(tip);
                    var objectUrl = file ? URL.createObjectURL(file) : "";
                    var src = objectUrl || url;
                    var cropper = null;
                    var areaW = window.innerWidth < 720 ? "92%" : "560px";

                    layer.open({
                        type: 1,
                        title: "裁剪",
                        area: [areaW, "auto"],
                        shadeClose: false,
                        zIndex: layer.zIndex + 10,
                        content:
                            '<div class="aic-wrap">' +
                            '<div class="aic-stage"><img class="aic-img" alt="" crossorigin="anonymous"></div>' +
                            "</div>",
                        btn: ["确认", "取消"],
                        success: function (layero) {
                            var img = layero.find(".aic-img")[0];
                            img.onload = function () {
                                cropper = new global.Cropper(img, {
                                    aspectRatio: aspectRatio,
                                    viewMode: 1,
                                    dragMode: "move",
                                    autoCropArea: 0.9,
                                    restore: false,
                                    guides: false,
                                    center: false,
                                    highlight: false,
                                    cropBoxMovable: true,
                                    cropBoxResizable: true,
                                    toggleDragModeOnDblclick: false,
                                    background: false,
                                    responsive: true,
                                    checkOrientation: true,
                                });
                            };
                            img.onerror = function () {
                                layer.msg("图片加载失败", {icon: 2});
                            };
                            img.src = src;
                        },
                        yes: function (idx) {
                            if (!cropper) return false;
                            var canvas = cropper.getCroppedCanvas({
                                maxWidth: maxEdge,
                                maxHeight: maxEdge,
                                imageSmoothingEnabled: true,
                                imageSmoothingQuality: "medium",
                            });
                            if (!canvas) {
                                layer.msg("裁剪失败", {icon: 2});
                                return false;
                            }
                            canvas.toBlob(
                                function (blob) {
                                    if (!blob) {
                                        layer.msg("裁剪失败", {icon: 2});
                                        return;
                                    }
                                    var name = String(baseName).replace(/\.[^.]+$/, "") + ".jpg";
                                    var out = new File([blob], name, {type: "image/jpeg"});
                                    cleanup();
                                    layer.close(idx);
                                    onDone(out);
                                },
                                "image/jpeg",
                                0.85
                            );
                            return false;
                        },
                        btn2: function (idx) {
                            cleanup();
                            layer.close(idx);
                            onCancel();
                            return false;
                        },
                        cancel: function () {
                            cleanup();
                            onCancel();
                        },
                    });

                    function cleanup() {
                        if (cropper) {
                            cropper.destroy();
                            cropper = null;
                        }
                        if (objectUrl) URL.revokeObjectURL(objectUrl);
                    }
                })
                .catch(function (err) {
                    layer.close(tip);
                    layer.msg(err.message || "裁剪组件加载失败", {icon: 2});
                    onCancel();
                });
        });
    }

    global.OakisImageCrop = {
        open: openCrop,
    };
})(window);
